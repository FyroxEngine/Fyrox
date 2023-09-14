use crate::{
    interaction::InteractionMode,
    make_color_material,
    message::MessageSender,
    scene::{
        commands::terrain::{ModifyTerrainHeightCommand, ModifyTerrainLayerMaskCommand},
        EditorScene, Selection,
    },
    settings::Settings,
    MSG_SYNC_FLAG,
};
use fyrox::gui::inspector::PropertyAction;
use fyrox::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        arrayvec::ArrayVec,
        color::Color,
        log::{Log, MessageKind},
        math::vector_to_quat,
        pool::Handle,
    },
    engine::Engine,
    gui::{
        inspector::{
            editors::{
                enumeration::EnumPropertyEditorDefinition, PropertyEditorDefinitionContainer,
            },
            Inspector, InspectorBuilder, InspectorContext, InspectorMessage,
        },
        message::{MessageDirection, UiMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
    scene::{
        base::BaseBuilder,
        camera::Camera,
        graph::Graph,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        terrain::{Brush, BrushMode, BrushShape, Terrain, TerrainRayCastResult},
    },
};
use std::rc::Rc;

pub struct TerrainInteractionMode {
    heightmaps: Vec<Vec<f32>>,
    masks: Vec<Vec<u8>>,
    message_sender: MessageSender,
    interacting: bool,
    brush_gizmo: BrushGizmo,
    brush: Brush,
    brush_panel: BrushPanel,
}

impl TerrainInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut Engine,
        message_sender: MessageSender,
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
    pub fn new(editor_scene: &EditorScene, engine: &mut Engine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let brush = MeshBuilder::new(
            BaseBuilder::new()
                .with_cast_shadows(false)
                .with_depth_offset(0.01)
                .with_name("Brush")
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
            SurfaceData::make_quad(&Matrix4::identity()),
        ))
        .with_material(make_color_material(Color::from_rgba(0, 255, 0, 130)))
        .build()])
        .build(graph);

        graph.link_nodes(brush, editor_scene.editor_objects_root);

        Self { brush }
    }

    pub fn set_visible(&self, graph: &mut Graph, visibility: bool) {
        graph[self.brush].set_visibility(visibility);
    }
}

fn copy_layer_masks(terrain: &Terrain, layer: usize) -> Vec<Vec<u8>> {
    let mut masks = vec![];

    for chunk in terrain.chunks_ref() {
        masks.push(chunk.layer_masks[layer].data_ref().data().to_vec());
    }

    masks
}

impl InteractionMode for TerrainInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];
                if let Some(terrain) = &graph[handle].cast::<Terrain>() {
                    // Pick height value at the point of interaction.
                    if let BrushMode::FlattenHeightMap { height } = &mut self.brush.mode {
                        let camera = &graph[editor_scene.camera_controller.camera];
                        if let Some(camera) = camera.cast::<Camera>() {
                            let ray = camera.make_ray(mouse_pos, frame_size);

                            let mut intersections = ArrayVec::<TerrainRayCastResult, 128>::new();
                            terrain.raycast(ray, &mut intersections, true);

                            if let Some(closest) = intersections.first() {
                                *height = closest.height;
                            }
                        }
                    }

                    match self.brush.mode {
                        BrushMode::ModifyHeightMap { .. } | BrushMode::FlattenHeightMap { .. } => {
                            self.heightmaps = terrain
                                .chunks_ref()
                                .iter()
                                .map(|c| c.heightmap_owned())
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
        engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];

                if let Some(terrain) = &graph[handle].cast::<Terrain>() {
                    if self.interacting {
                        let new_heightmaps = terrain
                            .chunks_ref()
                            .iter()
                            .map(|c| c.heightmap_owned())
                            .collect();

                        match self.brush.mode {
                            BrushMode::ModifyHeightMap { .. }
                            | BrushMode::FlattenHeightMap { .. } => {
                                self.message_sender.do_scene_command(
                                    ModifyTerrainHeightCommand::new(
                                        handle,
                                        std::mem::take(&mut self.heightmaps),
                                        new_heightmaps,
                                    ),
                                );
                            }
                            BrushMode::DrawOnMask { layer, .. } => {
                                self.message_sender.do_scene_command(
                                    ModifyTerrainLayerMaskCommand::new(
                                        handle,
                                        std::mem::take(&mut self.masks),
                                        copy_layer_masks(terrain, layer),
                                        layer,
                                    ),
                                );
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
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let handle = selection.nodes()[0];

                let camera = &graph[camera];
                if let Some(camera) = camera.cast::<Camera>() {
                    let ray = camera.make_ray(mouse_position, frame_size);
                    if let Some(terrain) = graph[handle].cast_mut::<Terrain>() {
                        let mut intersections = ArrayVec::<TerrainRayCastResult, 128>::new();
                        terrain.raycast(ray, &mut intersections, true);

                        if let Some(closest) = intersections.first() {
                            self.brush.center = closest.position;

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
                                BrushMode::FlattenHeightMap { height } => {
                                    if engine.user_interface.keyboard_modifiers().shift {
                                        *height *= -1.0;
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
                                .set_position(closest.position)
                                .set_scale(scale)
                                .set_rotation(vector_to_quat(closest.normal));
                        }
                    }
                }
            }
        }
    }

    fn activate(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
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

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut Engine) {
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
        _engine: &mut Engine,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if selection.is_single_selection() {
                self.brush_panel.handle_ui_message(message, &mut self.brush);
            }
        }
    }

    fn on_drop(&mut self, engine: &mut Engine) {
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
            2 => BrushMode::FlattenHeightMap { height: 0.0 },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushMode::ModifyHeightMap { .. } => 0,
            BrushMode::DrawOnMask { .. } => 1,
            BrushMode::FlattenHeightMap { .. } => 2,
        },
        names_generator: || {
            vec![
                "Modify Height Map".to_string(),
                "Draw On Mask".to_string(),
                "Flatten Height Map".to_string(),
            ]
        },
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
        let property_editors = PropertyEditorDefinitionContainer::new();
        property_editors.insert(make_brush_mode_enum_property_editor_definition());
        property_editors.insert(make_brush_shape_enum_property_editor_definition());

        let context = InspectorContext::from_object(
            brush,
            ctx,
            Rc::new(property_editors),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
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

        if let Err(e) = ctx.sync(brush, ui, 0, true, Default::default()) {
            Log::writeln(
                MessageKind::Error,
                format!("Failed to sync BrushPanel's inspector. Reason: {:?}", e),
            )
        }
    }

    fn handle_ui_message(&self, message: &UiMessage, brush: &mut Brush) -> Option<()> {
        if message.destination() == self.inspector
            && message.direction() == MessageDirection::FromWidget
        {
            if let Some(InspectorMessage::PropertyChanged(msg)) = message.data::<InspectorMessage>()
            {
                PropertyAction::from_field_kind(&msg.value).apply(
                    &msg.path(),
                    brush,
                    &mut |result| {
                        Log::verify(result);
                    },
                );
            }
        }
        Some(())
    }
}
