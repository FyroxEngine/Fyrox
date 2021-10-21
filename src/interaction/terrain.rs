use crate::{
    interaction::InteractionMode,
    make_color_material,
    scene::{
        commands::terrain::{ModifyTerrainHeightCommand, ModifyTerrainLayerMaskCommand},
        EditorScene, Selection,
    },
    settings::Settings,
    GameEngine, Message, MSG_SYNC_FLAG,
};
use rg3d::engine::Engine;
use rg3d::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        math::vector_to_quat,
        pool::Handle,
    },
    gui::{
        inspector::{
            editors::{
                enumeration::EnumPropertyEditorDefinition, PropertyEditorDefinitionContainer,
            },
            Inspector, InspectorBuilder, InspectorContext,
        },
        message::{
            FieldKind, InspectorMessage, MessageDirection, UiMessage, UiMessageData, WidgetMessage,
            WindowMessage,
        },
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, UiNode, UserInterface,
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
    utils::log::{Log, MessageKind},
};
use std::{
    rc::Rc,
    sync::{mpsc::Sender, Arc, RwLock},
};

pub struct TerrainInteractionMode {
    heightmaps: Vec<Vec<f32>>,
    masks: Vec<Vec<u8>>,
    message_sender: Sender<Message>,
    interacting: bool,
    brush_gizmo: BrushGizmo,
    brush: Brush,
    brush_panel: BrushPanel,
}

impl TerrainInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        let brush = Brush {
            center: Default::default(),
            shape: BrushShape::Circle { radius: 1.0 },
            mode: BrushMode::ModifyHeightMap { amount: 1.0 },
        };

        let brush_panel = BrushPanel::new(&mut engine.user_interface.build_ctx(), &brush);

        Self {
            brush_panel,
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
    terrain.layers()[layer]
        .chunk_masks()
        .iter()
        .map(|mask| mask.data_ref().data().to_vec())
        .collect()
}

impl InteractionMode for TerrainInteractionMode {
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
                    match self.brush.mode {
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

                        match self.brush.mode {
                            BrushMode::ModifyHeightMap { .. } => {
                                self.message_sender
                                    .send(Message::do_scene_command(
                                        ModifyTerrainHeightCommand::new(
                                            handle,
                                            std::mem::take(&mut self.heightmaps),
                                            new_heightmaps,
                                        ),
                                    ))
                                    .unwrap();
                            }
                            BrushMode::DrawOnMask { layer, .. } => {
                                self.message_sender
                                    .send(Message::do_scene_command(
                                        ModifyTerrainLayerMaskCommand::new(
                                            handle,
                                            std::mem::take(&mut self.masks),
                                            copy_layer_masks(terrain, layer),
                                            layer,
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

                            self.brush.center = global_position;

                            let mut brush_copy = self.brush.clone();
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

                            let scale = match self.brush.shape {
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

    fn activate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        self.brush_gizmo
            .set_visible(&mut engine.scenes[editor_scene.scene].graph, true);

        self.brush_panel
            .sync_to_model(&mut engine.user_interface, &self.brush);

        engine.user_interface.send_message(WindowMessage::open(
            self.brush_panel.window,
            MessageDirection::ToWidget,
            false,
        ));
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        self.brush_gizmo
            .set_visible(&mut engine.scenes[editor_scene.scene].graph, false);

        engine.user_interface.send_message(WindowMessage::close(
            self.brush_panel.window,
            MessageDirection::ToWidget,
        ));
    }

    fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                self.brush_panel.handle_ui_message(
                    message,
                    &mut self.brush,
                    selection.nodes()[0],
                    editor_scene,
                    engine,
                );
            }
        }
    }

    fn on_drop(&mut self, engine: &mut GameEngine) {
        engine.user_interface.send_message(WidgetMessage::remove(
            self.brush_panel.window,
            MessageDirection::ToWidget,
        ));
    }
}

struct BrushPanel {
    window: Handle<UiNode>,
    inspector: Handle<UiNode>,
}

fn make_brush_mode_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushMode> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushMode::ModifyHeightMap { amount: 0.1 },
            1 => BrushMode::DrawOnMask {
                layer: 0,
                alpha: 1.0,
            },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushMode::ModifyHeightMap { .. } => 0,
            BrushMode::DrawOnMask { .. } => 1,
        },
        names_generator: || vec!["Modify Height Map".to_string(), "Draw On Mask".to_string()],
    }
}

fn make_brush_shape_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushShape> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushShape::Circle { radius: 0.5 },
            1 => BrushShape::Rectangle {
                width: 0.5,
                length: 0.5,
            },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushShape::Circle { .. } => 0,
            BrushShape::Rectangle { .. } => 1,
        },
        names_generator: || vec!["Circle".to_string(), "Rectangle".to_string()],
    }
}

impl BrushPanel {
    fn new(ctx: &mut BuildContext, brush: &Brush) -> Self {
        let mut property_editors = PropertyEditorDefinitionContainer::new();
        property_editors.insert(make_brush_mode_enum_property_editor_definition());
        property_editors.insert(make_brush_shape_enum_property_editor_definition());

        let context = InspectorContext::from_object(
            brush,
            ctx,
            Rc::new(property_editors),
            None,
            MSG_SYNC_FLAG,
        );

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(200.0).with_height(250.0))
            .can_close(false)
            .with_content({
                inspector = InspectorBuilder::new(WidgetBuilder::new())
                    .with_context(context)
                    .build(ctx);
                inspector
            })
            .open(false)
            .with_title(WindowTitle::text("Brush Options"))
            .build(ctx);

        Self { window, inspector }
    }

    fn sync_to_model(&self, ui: &mut UserInterface, brush: &Brush) {
        let ctx = ui
            .node(self.inspector)
            .cast::<Inspector>()
            .expect("Must be Inspector!")
            .context()
            .clone();

        if let Err(e) = ctx.sync(brush, ui) {
            Log::writeln(
                MessageKind::Error,
                format!("Failed to sync BrushPanel's inspector. Reason: {:?}", e),
            )
        }
    }

    fn handle_ui_message(
        &self,
        message: &UiMessage,
        brush: &mut Brush,
        terrain: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &Engine,
    ) -> Option<()> {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let UiMessageData::Inspector(InspectorMessage::PropertyChanged(msg)) = message.data()
            {
                match msg.value {
                    FieldKind::Object(ref args) => match msg.name.as_ref() {
                        Brush::SHAPE => {
                            brush.shape = args.cast_value().cloned()?;
                        }
                        Brush::MODE => {
                            brush.mode = args.cast_value().cloned()?;
                        }
                        _ => (),
                    },
                    FieldKind::Inspectable(ref inner) => {
                        if let FieldKind::Object(ref args) = inner.value {
                            match msg.name.as_ref() {
                                Brush::SHAPE => match inner.name.as_ref() {
                                    BrushShape::CIRCLE_RADIUS => {
                                        if let BrushShape::Circle { ref mut radius } = brush.shape {
                                            *radius = args.cast_value().cloned()?;
                                        }
                                    }
                                    BrushShape::RECTANGLE_WIDTH => {
                                        if let BrushShape::Rectangle { ref mut width, .. } =
                                            brush.shape
                                        {
                                            *width = args.cast_value().cloned()?;
                                        }
                                    }
                                    BrushShape::RECTANGLE_LENGTH => {
                                        if let BrushShape::Rectangle { ref mut length, .. } =
                                            brush.shape
                                        {
                                            *length = args.cast_value().cloned()?;
                                        }
                                    }
                                    _ => (),
                                },
                                Brush::MODE => match inner.name.as_ref() {
                                    BrushMode::MODIFY_HEIGHT_MAP_AMOUNT => {
                                        if let BrushMode::ModifyHeightMap { ref mut amount } =
                                            brush.mode
                                        {
                                            *amount = args.cast_value().cloned()?;
                                        }
                                    }
                                    BrushMode::DRAW_ON_MASK_LAYER => {
                                        if let BrushMode::DrawOnMask { ref mut layer, .. } =
                                            brush.mode
                                        {
                                            let node =
                                                &engine.scenes[editor_scene.scene].graph[terrain];
                                            if node.is_terrain() {
                                                let terrain = node.as_terrain();

                                                *layer = args
                                                    .cast_value::<usize>()
                                                    .cloned()?
                                                    .min(terrain.layers().len());
                                            }
                                        }
                                    }
                                    BrushMode::DRAW_ON_MASK_ALPHA => {
                                        if let BrushMode::DrawOnMask { ref mut alpha, .. } =
                                            brush.mode
                                        {
                                            *alpha = args.cast_value().cloned()?;
                                        }
                                    }
                                    _ => (),
                                },
                                _ => (),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Some(())
    }
}
