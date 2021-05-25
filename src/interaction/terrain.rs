use crate::scene::Selection;
use crate::{interaction::InteractionModeTrait, scene::EditorScene, GameEngine, Message};
use rg3d::scene::graph::Graph;
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        terrain::TerrainRayCastResult,
    },
};
use std::sync::{mpsc::Sender, Arc, RwLock};

pub struct TerrainInteractionMode {
    message_sender: Sender<Message>,
    interacting: bool,
    brush_gizmo: BrushGizmo,
}

impl TerrainInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            brush_gizmo: BrushGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
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
            SurfaceData::make_quad(
                &UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                    .to_homogeneous(),
            ),
        )))
        .with_color(Color::opaque(0, 255, 0))
        .build()])
        .build(graph);

        graph.link_nodes(brush, editor_scene.root);

        Self { brush }
    }

    pub fn set_visible(&self, graph: &mut Graph, visibility: bool) {
        graph[self.brush].set_visibility(visibility);
    }
}

impl InteractionModeTrait for TerrainInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_scene: &mut EditorScene,
        _engine: &mut GameEngine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
    ) {
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_scene: &mut EditorScene,
        _engine: &mut GameEngine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
    ) {
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];

                if let Node::Terrain(terrain) = &graph[handle] {
                    let camera = &graph[camera];
                    if let Node::Camera(camera) = camera {
                        let ray = camera.make_ray(mouse_position, frame_size);

                        let mut intersections = ArrayVec::<TerrainRayCastResult, 128>::new();
                        terrain.raycast(ray, &mut intersections, true);

                        if let Some(closest) = intersections.first() {
                            graph[self.brush_gizmo.brush]
                                .local_transform_mut()
                                .set_position(closest.position);
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
