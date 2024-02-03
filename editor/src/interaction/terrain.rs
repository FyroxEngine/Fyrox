use crate::interaction::make_interaction_mode_button;
use crate::scene::controller::SceneController;
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
use fyrox::core::uuid::{uuid, Uuid};
use fyrox::core::TypeUuidProvider;
use fyrox::graph::SceneGraph;
use fyrox::gui::{HorizontalAlignment, Thickness, VerticalAlignment};
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
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        terrain::{Brush, BrushMode, BrushShape, Terrain, TerrainRayCastResult},
    },
};
use std::sync::Arc;

pub struct TerrainInteractionMode {
    heightmaps: Vec<Vec<f32>>,
    masks: Vec<Vec<u8>>,
    message_sender: MessageSender,
    interacting: bool,
    brush_gizmo: BrushGizmo,
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
            center: Default::default(),
            shape: BrushShape::Circle { radius: 1.0 },
            mode: BrushMode::ModifyHeightMap { amount: 1.0 },
        };

        let brush_panel = BrushPanel::new(&mut engine.user_interface.build_ctx(), &brush);

        Self {
            brush_panel,
            heightmaps: Default::default(),
            brush_gizmo: BrushGizmo::new(game_scene, engine),
            interacting: false,
            message_sender,
            brush,
            masks: Default::default(),
            scene_viewer_frame,
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
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
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

fn copy_layer_masks(terrain: &Terrain, layer: usize) -> Vec<Vec<u8>> {
    let mut masks = Vec::new();

    for chunk in terrain.chunks_ref() {
        match chunk.layer_masks.get(layer) {
            Some(mask) => masks.push(mask.data_ref().data().to_vec()),
            None => Log::err("layer index out of range"),
        }
    }

    masks
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

        if let Selection::Graph(selection) = editor_selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[game_scene.scene].graph;
                let handle = selection.nodes()[0];
                if let Some(terrain) = &graph[handle].cast::<Terrain>() {
                    // Pick height value at the point of interaction.
                    if let BrushMode::FlattenHeightMap { height } = &mut self.brush.mode {
                        let camera = &graph[game_scene.camera_controller.camera];
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
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[game_scene.scene].graph;
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
        editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Selection::Graph(selection) = editor_selection {
            if selection.is_single_selection() {
                let graph = &mut engine.scenes[game_scene.scene].graph;
                let handle = selection.nodes()[0];

                let camera = &graph[game_scene.camera_controller.camera];
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
                                BrushShape::Circle { radius } => Vector3::new(radius, radius, 1.0),
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
    }

    fn activate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        self.brush_gizmo
            .set_visible(&mut engine.scenes[game_scene.scene].graph, true);

        self.brush_panel
            .sync_to_model(&mut engine.user_interface, &self.brush);

        engine
            .user_interface
            .send_message(WindowMessage::open_and_align(
                self.brush_panel.window,
                MessageDirection::ToWidget,
                self.scene_viewer_frame,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::top_right(5.0),
                false,
            ));
    }

    fn deactivate(&mut self, controller: &dyn SceneController, engine: &mut Engine) {
        let Some(game_scene) = controller.downcast_ref::<GameScene>() else {
            return;
        };

        self.brush_gizmo
            .set_visible(&mut engine.scenes[game_scene.scene].graph, false);

        engine.user_interface.send_message(WindowMessage::close(
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
        if let Selection::Graph(selection) = editor_selection {
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

    fn on_hot_key(
        &mut self,
        hotkey: &HotKey,
        _controller: &mut dyn SceneController,
        engine: &mut Engine,
        settings: &Settings,
    ) -> bool {
        let mut processed = false;

        fn modify_clamp(x: &mut f32, delta: f32, min: f32, max: f32) {
            *x = (*x + delta).clamp(min, max)
        }

        let key_bindings = &settings.key_bindings.terrain_key_bindings;
        if hotkey == &key_bindings.draw_on_mask_mode {
            self.brush.mode = BrushMode::DrawOnMask {
                layer: 0,
                alpha: 1.0,
            };
            processed = true;
        } else if hotkey == &key_bindings.modify_height_map_mode {
            self.brush.mode = BrushMode::ModifyHeightMap { amount: 1.0 };
            processed = true;
        } else if hotkey == &key_bindings.flatten_slopes_mode {
            self.brush.mode = BrushMode::FlattenHeightMap { height: 0.0 };
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
            match &mut self.brush.mode {
                BrushMode::ModifyHeightMap { amount } => {
                    *amount -= 0.01;
                }
                BrushMode::FlattenHeightMap { height } => {
                    *height -= 0.01;
                }
                BrushMode::DrawOnMask { alpha, .. } => modify_clamp(alpha, -0.01, 0.0, 1.0),
            }
            processed = true;
        } else if hotkey == &key_bindings.increase_brush_opacity {
            match &mut self.brush.mode {
                BrushMode::ModifyHeightMap { amount } => {
                    *amount += 0.01;
                }
                BrushMode::FlattenHeightMap { height } => {
                    *height += 0.01;
                }
                BrushMode::DrawOnMask { alpha, .. } => modify_clamp(alpha, 0.01, 0.0, 1.0),
            }
            processed = true;
        } else if hotkey == &key_bindings.prev_layer {
            if let BrushMode::DrawOnMask { layer, .. } = &mut self.brush.mode {
                *layer = layer.saturating_sub(1);
            }
            processed = true;
        } else if hotkey == &key_bindings.next_layer {
            if let BrushMode::DrawOnMask { layer, .. } = &mut self.brush.mode {
                *layer = layer.saturating_add(1);
            }
            processed = true;
        }

        if processed {
            self.brush_panel
                .sync_to_model(&mut engine.user_interface, &self.brush);
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
            Arc::new(property_editors),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
        );

        let inspector;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(150.0))
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
