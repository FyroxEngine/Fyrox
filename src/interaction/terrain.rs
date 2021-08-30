use crate::settings::Settings;
use crate::{
    interaction::InteractionModeTrait,
    make_color_material,
    scene::{
        commands::{
            terrain::{ModifyTerrainHeightCommand, ModifyTerrainLayerMaskCommand},
            SceneCommand,
        },
        EditorScene, Selection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        math::vector_to_quat,
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        graph::Graph,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        terrain::{Brush, BrushMode, BrushShape, Terrain, TerrainRayCastResult},
    },
};
use std::sync::{mpsc::Sender, Arc, Mutex, RwLock};

pub struct TerrainInteractionMode {
    heightmaps: Vec<Vec<f32>>,
    masks: Vec<Vec<u8>>,
    message_sender: Sender<Message>,
    interacting: bool,
    brush_gizmo: BrushGizmo,
    brush: Arc<Mutex<Brush>>,
}

impl TerrainInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
        brush: Arc<Mutex<Brush>>,
    ) -> Self {
        Self {
            heightmaps: Default::default(),
            brush_gizmo: BrushGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
            brush,
            masks: Default::default(),
        }
    }
}

pub struct BrushGizmo {
    brush: Handle<Node>,
}

impl BrushGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let brush = MeshBuilder::new(
            BaseBuilder::new()
                .with_depth_offset(0.01)
                .with_name("Brush")
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_cast_shadows(false)
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
            SurfaceData::make_quad(&Matrix4::identity()),
        )))
        .with_material(make_color_material(Color::from_rgba(0, 255, 0, 130)))
        .build()])
        .build(graph);

        graph.link_nodes(brush, editor_scene.root);

        Self { brush }
    }

    pub fn set_visible(&self, graph: &mut Graph, visibility: bool) {
        graph[self.brush].set_visibility(visibility);
    }
}

fn copy_layer_masks(terrain: &Terrain, layer: usize) -> Vec<Vec<u8>> {
    terrain
        .chunks_ref()
        .iter()
        .map(|c| {
            c.layers()[layer]
                .mask
                .as_ref()
                .unwrap()
                .data_ref()
                .data()
                .to_vec()
        })
        .collect()
}

impl InteractionModeTrait for TerrainInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];

                if let Node::Terrain(terrain) = &graph[handle] {
                    match self.brush.lock().unwrap().mode {
                        BrushMode::ModifyHeightMap { .. } => {
                            self.heightmaps = terrain
                                .chunks_ref()
                                .iter()
                                .map(|c| c.heightmap().to_vec())
                                .collect();
                        }
                        BrushMode::DrawOnMask { layer, .. } => {
                            self.masks = copy_layer_masks(terrain, layer);
                        }
                    }

                    self.interacting = true;
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];

                if let Node::Terrain(terrain) = &graph[handle] {
                    if self.interacting {
                        let new_heightmaps = terrain
                            .chunks_ref()
                            .iter()
                            .map(|c| c.heightmap().to_vec())
                            .collect();

                        match self.brush.lock().unwrap().mode {
                            BrushMode::ModifyHeightMap { .. } => {
                                self.message_sender
                                    .send(Message::DoSceneCommand(
                                        SceneCommand::ModifyTerrainHeight(
                                            ModifyTerrainHeightCommand::new(
                                                handle,
                                                std::mem::take(&mut self.heightmaps),
                                                new_heightmaps,
                                            ),
                                        ),
                                    ))
                                    .unwrap();
                            }
                            BrushMode::DrawOnMask { layer, .. } => {
                                self.message_sender
                                    .send(Message::DoSceneCommand(
                                        SceneCommand::ModifyTerrainLayerMask(
                                            ModifyTerrainLayerMaskCommand::new(
                                                handle,
                                                std::mem::take(&mut self.masks),
                                                copy_layer_masks(terrain, layer),
                                                layer,
                                            ),
                                        ),
                                    ))
                                    .unwrap();
                            }
                        }

                        self.interacting = false;
                    }
                }
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];

                let camera = &graph[camera];
                if let Node::Camera(camera) = camera {
                    let ray = camera.make_ray(mouse_position, frame_size);
                    if let Node::Terrain(terrain) = &mut graph[handle] {
                        let mut intersections = ArrayVec::<TerrainRayCastResult, 128>::new();
                        terrain.raycast(ray, &mut intersections, true);

                        if let Some(closest) = intersections.first() {
                            let global_position = terrain
                                .global_transform()
                                .transform_point(&Point3::from(closest.position))
                                .coords;

                            let mut brush = self.brush.lock().unwrap();
                            brush.center = global_position;

                            let mut brush_copy = brush.clone();
                            match &mut brush_copy.mode {
                                BrushMode::ModifyHeightMap { amount } => {
                                    if engine.user_interface.keyboard_modifiers().shift {
                                        *amount *= -1.0;
                                    }
                                }
                                BrushMode::DrawOnMask { alpha, .. } => {
                                    if engine.user_interface.keyboard_modifiers().shift {
                                        *alpha = -1.0;
                                    }
                                }
                            }

                            if self.interacting {
                                terrain.draw(&brush_copy);
                            }

                            let scale = match brush.shape {
                                BrushShape::Circle { radius } => Vector3::new(radius, 1.0, radius),
                                BrushShape::Rectangle { width, length } => {
                                    Vector3::new(width, 1.0, length)
                                }
                            };

                            graph[self.brush_gizmo.brush]
                                .local_transform_mut()
                                .set_position(global_position)
                                .set_scale(scale)
                                .set_rotation(vector_to_quat(closest.normal));
                        }
                    }
                }
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &mut EditorScene,
        _camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.brush_gizmo.set_visible(graph, true);
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.brush_gizmo.set_visible(graph, false);
    }
}
