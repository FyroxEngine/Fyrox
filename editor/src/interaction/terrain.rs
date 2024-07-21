use fyrox::scene::terrain::brushstroke::{BrushSender, BrushThreadMessage, UndoData};

use crate::fyrox::core::uuid::{uuid, Uuid};
use crate::fyrox::core::TypeUuidProvider;
use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::gui::{HorizontalAlignment, Thickness, VerticalAlignment};
use crate::fyrox::{
    core::{
        algebra::{Matrix2, Matrix4, Vector2, Vector3},
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
            Inspector, InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction,
        },
        key::HotKey,
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
            surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
            MeshBuilder, RenderPath,
        },
        node::Node,
        terrain::brushstroke::{Brush, BrushMode, BrushShape, BrushStroke, BrushTarget},
        terrain::{Terrain, TerrainRayCastResult},
    },
};
use crate::interaction::make_interaction_mode_button;
use crate::scene::controller::SceneController;
use crate::scene::SelectionContainer;
use crate::{
    interaction::InteractionMode,
    make_color_material,
    message::MessageSender,
    scene::{
        commands::terrain::{ModifyTerrainHeightCommand, ModifyTerrainLayerMaskCommand},
        GameScene, Selection,
    },
    settings::Settings,
    MSG_SYNC_FLAG,
};
use fyrox::asset::untyped::ResourceKind;
use std::sync::mpsc::channel;
use std::sync::Arc;

fn modify_clamp(x: &mut f32, delta: f32, min: f32, max: f32) {
    *x = (*x + delta).clamp(min, max)
}

fn handle_undo_chunks(undo_chunks: UndoData, sender: &MessageSender) {
    match undo_chunks.target {
        BrushTarget::HeightMap => sender.do_command(ModifyTerrainHeightCommand::new(
            undo_chunks.node,
            undo_chunks.chunks,
        )),
        BrushTarget::LayerMask { layer } => sender.do_command(ModifyTerrainLayerMaskCommand::new(
            undo_chunks.node,
            undo_chunks.chunks,
            layer,
        )),
    }
}

pub struct TerrainInteractionMode {
    message_sender: MessageSender,
    brush_sender: Option<BrushSender>,
    interacting: bool,
    brush_gizmo: BrushGizmo,
    prev_brush_position: Option<Vector2<f32>>,
    brush_position: Vector3<f32>,
    brush_value: f32,
    brush: Brush,
    brush_panel: BrushPanel,
    scene_viewer_frame: Handle<UiNode>,
}

impl TerrainInteractionMode {
    pub fn new(
        game_scene: &GameScene,
        engine: &mut Engine,
        message_sender: MessageSender,
        scene_viewer_frame: Handle<UiNode>,
    ) -> Self {
        let brush = Brush {
            shape: BrushShape::Circle { radius: 1.0 },
            mode: BrushMode::Raise { amount: 1.0 },
            target: BrushTarget::HeightMap,
            alpha: 1.0,
            hardness: 1.0,
            transform: Matrix2::identity(),
        };

        let brush_panel =
            BrushPanel::new(&mut engine.user_interfaces.first_mut().build_ctx(), &brush);

        Self {
            message_sender,
            brush_sender: None,
            brush_panel,
            brush_gizmo: BrushGizmo::new(game_scene, engine),
            interacting: false,
            brush,
            brush_value: Default::default(),
            prev_brush_position: None,
            brush_position: Vector3::default(),
            scene_viewer_frame,
        }
    }
    fn modify_brush_opacity(&mut self, direction: f32) {
        modify_clamp(&mut self.brush.alpha, 0.01 * direction, 0.0, 1.0);
    }
    fn start_background_thread(&mut self) {
        let (sender, receiver) = channel::<BrushThreadMessage>();
        self.brush_sender = Some(BrushSender::new(sender));
        let sender_clone = self.message_sender.clone();
        let mut stroke = BrushStroke::with_chunk_handler(Box::new(move |undo_chunks| {
            handle_undo_chunks(undo_chunks, &sender_clone)
        }));
        match std::thread::Builder::new()
            .name("Terrain Brush".into())
            .spawn(move || stroke.accept_messages(receiver))
        {
            Ok(_) => (),
            Err(_) => {
                Log::err("Brush thread failed to start.");
                self.brush_sender = None;
            }
        }
    }
    fn end_background_thread(&mut self) {
        self.brush_sender = None;
    }
    fn start_stroke(&self, terrain: &mut Terrain, handle: Handle<Node>, shift: bool) {
        let mut brush = self.brush.clone();
        // Ignore stroke with a non-existent layer index.
        if let BrushTarget::LayerMask { layer } = brush.target {
            if layer >= terrain.layers().len() {
                return;
            }
        }
        // Reverse the behavior of a brush when shift is held.
        if shift {
            match &mut brush.mode {
                BrushMode::Raise { amount } => {
                    *amount *= -1.0;
                }
                BrushMode::Assign { value } => {
                    *value = 1.0 - *value;
                }
                _ => (),
            }
        }
        let data = terrain.texture_data(brush.target);
        if let Some(sender) = &self.brush_sender {
            sender.start_stroke(brush, handle, data);
        } else {
            Log::err("Brush thread failure");
        }
    }
    fn draw(&mut self, terrain: &mut Terrain) {
        let Some(position) = terrain.project(self.brush_position) else {
            return;
        };
        let position = match self.brush.target {
            BrushTarget::HeightMap => terrain.local_to_height_pixel(position),
            BrushTarget::LayerMask { .. } => terrain.local_to_mask_pixel(position),
        };
        let scale = match self.brush.target {
            BrushTarget::HeightMap => terrain.height_grid_scale(),
            BrushTarget::LayerMask { .. } => terrain.mask_grid_scale(),
        };
        if let Some(sender) = &self.brush_sender {
            if let Some(start) = self.prev_brush_position.take() {
                self.brush.smear(start, position, scale, |p, a| {
                    sender.draw_pixel(p, a, self.brush_value)
                });
            } else {
                self.brush.stamp(position, scale, |p, a| {
                    sender.draw_pixel(p, a, self.brush_value)
                });
            }
            self.prev_brush_position = Some(position);
        }
    }
}

pub struct BrushGizmo {
    brush: Handle<Node>,
}

impl BrushGizmo {
    pub fn new(game_scene: &GameScene, engine: &mut Engine) -> Self {
        let scene = &mut engine.scenes[game_scene.scene];
        let graph = &mut scene.graph;

        let brush = MeshBuilder::new(
            BaseBuilder::new()
                .with_cast_shadows(false)
                .with_depth_offset(0.01)
                .with_name("Brush")
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
            ResourceKind::Embedded,
            SurfaceData::make_quad(&Matrix4::identity()),
        ))
        .with_material(make_color_material(Color::from_rgba(0, 255, 0, 130)))
        .build()])
        .build(graph);

        graph.link_nodes(brush, game_scene.editor_objects_root);

        Self { brush }
    }

    pub fn set_visible(&self, graph: &mut Graph, visibility: bool) {
        graph[self.brush].set_visibility(visibility);
    }
}

impl TypeUuidProvider for TerrainInteractionMode {
    fn type_uuid() -> Uuid {
        uuid!("bc19eff3-3e3a-49c0-9a9d-17d36fccc34e")
    }
}

impl InteractionMode for TerrainInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Some(selection) = editor_selection.as_graph() {
            if selection.is_single_selection() {
                let shift = engine
                    .user_interfaces
                    .first_mut()
                    .keyboard_modifiers()
                    .shift;
                let graph = &mut engine.scenes[game_scene.scene].graph;
                let handle = selection.nodes()[0];
                let ray = graph[game_scene.camera_controller.camera]
                    .cast::<Camera>()
                    .map(|cam| cam.make_ray(mouse_pos, frame_size));
                if let Some(terrain) = graph[handle].cast_mut::<Terrain>() {
                    // Pick height value at the point of interaction.
                    if let BrushMode::Flatten { .. } = &mut self.brush.mode {
                        if let Some(ray) = ray {
                            let mut intersections = ArrayVec::<TerrainRayCastResult, 128>::new();
                            terrain.raycast(ray, &mut intersections, true);

                            let first = intersections.first();
                            if let (Some(closest), BrushTarget::HeightMap) =
                                (first, self.brush.target)
                            {
                                self.brush_value = closest.height;
                            } else if let Some(closest) = first {
                                let p = terrain.project(closest.position);
                                self.brush_value = if let Some(position) = p {
                                    terrain.interpolate_value(position, self.brush.target)
                                } else {
                                    0.0
                                };
                            } else {
                                self.brush_value = 0.0;
                            }
                        }
                    }
                    self.start_stroke(terrain, handle, shift);
                    self.prev_brush_position = None;
                    self.draw(terrain);
                    self.interacting = true;
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if self.interacting {
            if let Some(s) = &self.brush_sender {
                s.end_stroke()
            }
            self.interacting = false;
        }
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };
        let graph = &mut engine.scenes[game_scene.scene].graph;

        let mut gizmo_visible = false;

        if let Some(selection) = editor_selection.as_graph() {
            if selection.is_single_selection() {
                let handle = selection.nodes()[0];

                let camera = &graph[game_scene.camera_controller.camera];
                if let Some(camera) = camera.cast::<Camera>() {
                    let ray = camera.make_ray(mouse_position, frame_size);
                    if let Some(terrain) = graph[handle].cast_mut::<Terrain>() {
                        let mut intersections = ArrayVec::<TerrainRayCastResult, 128>::new();
                        terrain.raycast(ray, &mut intersections, true);

                        if let Some(closest) = intersections.first() {
                            self.brush_position = closest.position;
                            gizmo_visible = true;

                            if self.interacting {
                                self.draw(terrain);
                            }

                            let scale = match self.brush.shape {
                                BrushShape::Circle { radius } => {
                                    Vector3::new(radius * 2.0, radius * 2.0, 1.0)
                                }
                                BrushShape::Rectangle { width, length } => {
                                    Vector3::new(width, length, 1.0)
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
        let gizmo = &mut graph[self.brush_gizmo.brush];
        if gizmo.visibility() != gizmo_visible {
            gizmo.set_visibility(gizmo_visible);
        }
    }

    fn activate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        self.start_background_thread();

        self.brush_gizmo
            .set_visible(&mut engine.scenes[game_scene.scene].graph, true);

        self.brush_panel
            .sync_to_model(engine.user_interfaces.first_mut(), &self.brush);

        engine
            .user_interfaces
            .first_mut()
            .send_message(WindowMessage::open_and_align(
                self.brush_panel.window,
                MessageDirection::ToWidget,
                self.scene_viewer_frame,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::top_right(5.0),
                false,
                false,
            ));
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        self.end_background_thread();

        self.brush_gizmo
            .set_visible(&mut engine.scenes[game_scene.scene].graph, false);

        engine
            .user_interfaces
            .first_mut()
            .send_message(WindowMessage::close(
                self.brush_panel.window,
                MessageDirection::ToWidget,
            ));
    }

    fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
    ) {
        if let Some(selection) = editor_selection.as_graph() {
            if selection.is_single_selection() {
                self.brush_panel.handle_ui_message(message, &mut self.brush);
            }
        }
    }

    fn on_drop(&mut self, engine: &mut Engine) {
        engine
            .user_interfaces
            .first_mut()
            .send_message(WidgetMessage::remove(
                self.brush_panel.window,
                MessageDirection::ToWidget,
            ));
    }

    fn on_hot_key_pressed(
        &mut self,
        hotkey: &HotKey,
        _controller: &mut dyn SceneController,
        engine: &mut Engine,
        settings: &Settings,
    ) -> bool {
        let mut processed = false;

        let key_bindings = &settings.key_bindings.terrain_key_bindings;
        if hotkey == &key_bindings.draw_on_mask_mode {
            self.brush.target = BrushTarget::LayerMask { layer: 0 };
            processed = true;
        } else if hotkey == &key_bindings.modify_height_map_mode {
            self.brush.target = BrushTarget::HeightMap;
            processed = true;
        } else if hotkey == &key_bindings.flatten_slopes_mode {
            self.brush.mode = BrushMode::Flatten;
            processed = true;
        } else if hotkey == &key_bindings.increase_brush_size {
            match &mut self.brush.shape {
                BrushShape::Circle { radius } => modify_clamp(radius, 0.05, 0.0, f32::MAX),
                BrushShape::Rectangle { width, length } => {
                    modify_clamp(width, 0.05, 0.0, f32::MAX);
                    modify_clamp(length, 0.05, 0.0, f32::MAX);
                }
            }
            processed = true;
        } else if hotkey == &key_bindings.decrease_brush_size {
            match &mut self.brush.shape {
                BrushShape::Circle { radius } => modify_clamp(radius, -0.05, 0.0, f32::MAX),
                BrushShape::Rectangle { width, length } => {
                    modify_clamp(width, -0.05, 0.0, f32::MAX);
                    modify_clamp(length, -0.05, 0.0, f32::MAX);
                }
            }
            processed = true;
        } else if hotkey == &key_bindings.decrease_brush_opacity {
            self.modify_brush_opacity(-1.0);
            processed = true;
        } else if hotkey == &key_bindings.increase_brush_opacity {
            self.modify_brush_opacity(1.0);
            processed = true;
        } else if hotkey == &key_bindings.prev_layer {
            if let BrushTarget::LayerMask { layer, .. } = &mut self.brush.target {
                *layer = layer.saturating_sub(1);
            }
            processed = true;
        } else if hotkey == &key_bindings.next_layer {
            if let BrushTarget::LayerMask { layer, .. } = &mut self.brush.target {
                *layer = layer.saturating_add(1);
            }
            processed = true;
        }

        if processed {
            self.brush_panel
                .sync_to_model(engine.user_interfaces.first_mut(), &self.brush);
        }

        processed
    }

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        let terrain_mode_tooltip =
            "Edit Terrain\n\nTerrain edit mode allows you to modify selected \
        terrain.";

        make_interaction_mode_button(
            ctx,
            include_bytes!("../../resources/terrain.png"),
            terrain_mode_tooltip,
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }
}

struct BrushPanel {
    window: Handle<UiNode>,
    inspector: Handle<UiNode>,
}

fn make_brush_mode_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushMode> {
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushMode::Raise { amount: 0.1 },
            1 => BrushMode::Assign { value: 0.0 },
            2 => BrushMode::Flatten,
            3 => BrushMode::Smooth { kernel_radius: 5 },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushMode::Raise { .. } => 0,
            BrushMode::Assign { .. } => 1,
            BrushMode::Flatten { .. } => 2,
            BrushMode::Smooth { .. } => 3,
        },
        names_generator: || {
            vec![
                "Raise or Lower".to_string(),
                "Assign Value".to_string(),
                "Flatten".to_string(),
                "Smooth".to_string(),
            ]
        },
    }
}

fn make_brush_target_enum_property_editor_definition() -> EnumPropertyEditorDefinition<BrushTarget>
{
    EnumPropertyEditorDefinition {
        variant_generator: |i| match i {
            0 => BrushTarget::HeightMap,
            1 => BrushTarget::LayerMask { layer: 0 },
            _ => unreachable!(),
        },
        index_generator: |v| match v {
            BrushTarget::HeightMap => 0,
            BrushTarget::LayerMask { .. } => 1,
        },
        names_generator: || vec!["Height Map".to_string(), "Layer Mask".to_string()],
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
        let property_editors = PropertyEditorDefinitionContainer::with_default_editors();
        property_editors.insert(make_brush_mode_enum_property_editor_definition());
        property_editors.insert(make_brush_target_enum_property_editor_definition());
        property_editors.insert(make_brush_shape_enum_property_editor_definition());

        let context = InspectorContext::from_object(
            brush,
            ctx,
            Arc::new(property_editors),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
            150.0,
        );

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(250.0))
            .can_minimize(false)
            .can_maximize(false)
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
