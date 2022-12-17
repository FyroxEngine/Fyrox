use crate::{
    camera::PickingOptions, gui::make_dropdown_list_option,
    gui::make_dropdown_list_option_with_height, load_image, utils::enable_widget, AddModelCommand,
    AssetItem, AssetKind, BuildProfile, ChangeSelectionCommand, CommandGroup, DropdownListBuilder,
    EditorScene, GameEngine, GraphSelection, InteractionMode, InteractionModeKind, Message, Mode,
    SceneCommand, Selection, SetMeshTextureCommand, Settings,
};
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        make_relative_path,
        math::{plane::Plane, Rect},
        pool::Handle,
    },
    engine::Engine,
    fxhash::FxHashSet,
    gui::{
        border::BorderBuilder,
        brush::{Brush, GradientPoint},
        button::{Button, ButtonBuilder, ButtonContent, ButtonMessage},
        canvas::CanvasBuilder,
        decorator::{DecoratorBuilder, DecoratorMessage},
        dropdown_list::DropdownListMessage,
        grid::{Column, GridBuilder, Row},
        image::{ImageBuilder, ImageMessage},
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        utils::make_simple_tooltip,
        vec::vec3::{Vec3EditorBuilder, Vec3EditorMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        BRUSH_BRIGHT_BLUE, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST, COLOR_DARKEST,
        COLOR_LIGHTEST,
    },
    resource::texture::{Texture, TextureState},
    scene::{
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        camera::{Camera, Projection},
        node::Node,
    },
    utils::into_gui_texture,
};
use std::sync::mpsc::Sender;

struct PreviewInstance {
    instance: Handle<Node>,
    nodes: FxHashSet<Handle<Node>>,
}

pub struct SceneViewer {
    frame: Handle<UiNode>,
    window: Handle<UiNode>,
    pub last_mouse_pos: Option<Vector2<f32>>,
    pub click_mouse_pos: Option<Vector2<f32>>,
    selection_frame: Handle<UiNode>,
    // Side bar stuff
    select_mode: Handle<UiNode>,
    move_mode: Handle<UiNode>,
    rotate_mode: Handle<UiNode>,
    scale_mode: Handle<UiNode>,
    navmesh_mode: Handle<UiNode>,
    terrain_mode: Handle<UiNode>,
    camera_projection: Handle<UiNode>,
    switch_mode: Handle<UiNode>,
    build_profile: Handle<UiNode>,
    sender: Sender<Message>,
    interaction_mode_panel: Handle<UiNode>,
    contextual_actions: Handle<UiNode>,
    global_position_display: Handle<UiNode>,
    preview_instance: Option<PreviewInstance>,
}

fn make_interaction_mode_button(
    ctx: &mut BuildContext,
    image: &[u8],
    tooltip: &str,
    selected: bool,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(WidgetBuilder::new().with_foreground(Brush::LinearGradient {
                from: Vector2::new(0.5, 0.0),
                to: Vector2::new(0.5, 1.0),
                stops: vec![
                    GradientPoint {
                        stop: 0.0,
                        color: COLOR_LIGHTEST,
                    },
                    GradientPoint {
                        stop: 0.25,
                        color: COLOR_LIGHTEST,
                    },
                    GradientPoint {
                        stop: 1.0,
                        color: COLOR_DARKEST,
                    },
                ],
            }))
            .with_stroke_thickness(Thickness::uniform(1.0)),
        )
        .with_normal_brush(BRUSH_LIGHT)
        .with_hover_brush(BRUSH_LIGHTER)
        .with_pressed_brush(BRUSH_LIGHTEST)
        .with_selected_brush(BRUSH_BRIGHT_BLUE)
        .with_selected(selected)
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_width(32.0)
                .with_height(32.0),
        )
        .with_opt_texture(load_image(image))
        .build(ctx),
    )
    .build(ctx)
}

impl SceneViewer {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let select_mode_tooltip = "Select Object(s) - Shortcut: [1]\n\nSelection interaction mode \
        allows you to select an object by a single left mouse button click or multiple objects using either \
        frame selection (click and drag) or by holding Ctrl+Click";

        let move_mode_tooltip =
            "Move Object(s) - Shortcut: [2]\n\nMovement interaction mode allows you to move selected \
        objects. Keep in mind that movement always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        let rotate_mode_tooltip =
            "Rotate Object(s) - Shortcut: [3]\n\nRotation interaction mode allows you to rotate selected \
        objects. Keep in mind that rotation always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        let scale_mode_tooltip =
            "Scale Object(s) - Shortcut: [4]\n\nScaling interaction mode allows you to scale selected \
        objects. Keep in mind that scaling always works in local coordinates!\n\n\
        This also allows you to select an object or add an object to current selection using Ctrl+Click";

        let navmesh_mode_tooltip =
            "Edit Navmesh\n\nNavmesh edit mode allows you to modify selected \
        navigational mesh.";

        let terrain_mode_tooltip =
            "Edit Terrain\n\nTerrain edit mode allows you to modify selected \
        terrain.";

        let frame;
        let select_mode;
        let move_mode;
        let rotate_mode;
        let scale_mode;
        let navmesh_mode;
        let terrain_mode;
        let selection_frame;
        let camera_projection;
        let switch_mode;
        let build_profile;

        let interaction_mode_panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(0)
                .with_child({
                    select_mode = make_interaction_mode_button(
                        ctx,
                        include_bytes!("../resources/embed/select.png"),
                        select_mode_tooltip,
                        true,
                    );
                    select_mode
                })
                .with_child({
                    move_mode = make_interaction_mode_button(
                        ctx,
                        include_bytes!("../resources/embed/move_arrow.png"),
                        move_mode_tooltip,
                        false,
                    );
                    move_mode
                })
                .with_child({
                    rotate_mode = make_interaction_mode_button(
                        ctx,
                        include_bytes!("../resources/embed/rotate_arrow.png"),
                        rotate_mode_tooltip,
                        false,
                    );
                    rotate_mode
                })
                .with_child({
                    scale_mode = make_interaction_mode_button(
                        ctx,
                        include_bytes!("../resources/embed/scale_arrow.png"),
                        scale_mode_tooltip,
                        false,
                    );
                    scale_mode
                })
                .with_child({
                    navmesh_mode = make_interaction_mode_button(
                        ctx,
                        include_bytes!("../resources/embed/navmesh.png"),
                        navmesh_mode_tooltip,
                        false,
                    );
                    navmesh_mode
                })
                .with_child({
                    terrain_mode = make_interaction_mode_button(
                        ctx,
                        include_bytes!("../resources/embed/terrain.png"),
                        terrain_mode_tooltip,
                        false,
                    );
                    terrain_mode
                }),
        )
        .build(ctx);

        let contextual_actions = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    camera_projection = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(150.0),
                    )
                    .with_items(vec![
                        make_dropdown_list_option_with_height(ctx, "Perspective (3D)", 22.0),
                        make_dropdown_list_option_with_height(ctx, "Orthographic (2D)", 22.0),
                    ])
                    .with_selected(0)
                    .build(ctx);
                    camera_projection
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let top_ribbon = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .with_child({
                                switch_mode = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(100.0),
                                )
                                .with_text("Play")
                                .build(ctx);
                                switch_mode
                            })
                            .with_child({
                                build_profile = DropdownListBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(100.0),
                                )
                                .with_items(vec![
                                    make_dropdown_list_option(ctx, "Debug"),
                                    make_dropdown_list_option(ctx, "Release"),
                                ])
                                .with_selected(0)
                                .build(ctx);
                                build_profile
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx)
                })
                .with_child(contextual_actions),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let global_position_display;
        let bottom_toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_margin(Thickness::uniform(1.0))
                .with_child({
                    global_position_display = Vec3EditorBuilder::<f32>::new(
                        WidgetBuilder::new()
                            .with_tooltip(make_simple_tooltip(
                                ctx,
                                "Global Coordinates of the Current Selection",
                            ))
                            .with_width(200.0),
                    )
                    .with_editable(false)
                    .build(ctx);
                    global_position_display
                })
                .on_row(2),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .on_row(0)
                        .with_child(top_ribbon)
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_child({
                                        frame = ImageBuilder::new(
                                            WidgetBuilder::new()
                                                .on_row(0)
                                                .on_column(1)
                                                .with_allow_drop(true),
                                        )
                                        .with_flip(true)
                                        .build(ctx);
                                        frame
                                    })
                                    .with_child(
                                        CanvasBuilder::new(
                                            WidgetBuilder::new().on_column(1).with_child({
                                                selection_frame = BorderBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_visibility(false)
                                                        .with_background(Brush::Solid(
                                                            Color::from_rgba(255, 255, 255, 40),
                                                        ))
                                                        .with_foreground(Brush::Solid(
                                                            Color::opaque(0, 255, 0),
                                                        )),
                                                )
                                                .with_stroke_thickness(Thickness::uniform(1.0))
                                                .build(ctx);
                                                selection_frame
                                            }),
                                        )
                                        .build(ctx),
                                    )
                                    .with_child(interaction_mode_panel),
                            )
                            .add_row(Row::stretch())
                            .add_column(Column::auto())
                            .add_column(Column::stretch())
                            .build(ctx),
                        )
                        .with_child(bottom_toolbar),
                )
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Scene Preview"))
            .build(ctx);

        Self {
            sender,
            window,
            frame,
            last_mouse_pos: None,
            move_mode,
            rotate_mode,
            scale_mode,
            selection_frame,
            select_mode,
            navmesh_mode,
            terrain_mode,
            camera_projection,
            click_mouse_pos: None,
            switch_mode,
            interaction_mode_panel,
            contextual_actions,
            global_position_display,
            build_profile,
            preview_instance: None,
        }
    }
}

impl SceneViewer {
    pub fn window(&self) -> Handle<UiNode> {
        self.window
    }

    pub fn frame(&self) -> Handle<UiNode> {
        self.frame
    }

    pub fn selection_frame(&self) -> Handle<UiNode> {
        self.selection_frame
    }

    pub fn handle_message(&mut self, message: &Message, engine: &mut Engine) {
        if let Message::SetInteractionMode(mode) = message {
            let active_button = match mode {
                InteractionModeKind::Select => self.select_mode,
                InteractionModeKind::Move => self.move_mode,
                InteractionModeKind::Scale => self.scale_mode,
                InteractionModeKind::Rotate => self.rotate_mode,
                InteractionModeKind::Navmesh => self.navmesh_mode,
                InteractionModeKind::Terrain => self.terrain_mode,
            };

            for mode_button in [
                self.select_mode,
                self.move_mode,
                self.scale_mode,
                self.rotate_mode,
                self.navmesh_mode,
                self.terrain_mode,
            ] {
                let decorator = engine
                    .user_interface
                    .node(mode_button)
                    .query_component::<Button>()
                    .unwrap()
                    .decorator;

                engine.user_interface.send_message(DecoratorMessage::select(
                    decorator,
                    MessageDirection::ToWidget,
                    mode_button == active_button,
                ));
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &mut UiMessage,
        engine: &mut Engine,
        editor_scene: Option<&mut EditorScene>,
        interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        settings: &Settings,
        mode: &Mode,
    ) {
        let ui = &engine.user_interface;

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.scale_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Scale))
                    .unwrap();
            } else if message.destination() == self.rotate_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Rotate))
                    .unwrap();
            } else if message.destination() == self.move_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Move))
                    .unwrap();
            } else if message.destination() == self.select_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Select))
                    .unwrap();
            } else if message.destination() == self.navmesh_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Navmesh))
                    .unwrap();
            } else if message.destination() == self.terrain_mode {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Terrain))
                    .unwrap();
            } else if message.destination() == self.switch_mode {
                self.sender.send(Message::SwitchMode).unwrap();
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) =
            message.data::<WidgetMessage>()
        {
            if ui.is_node_child_of(message.destination(), self.move_mode)
                && *button == MouseButton::Right
            {
                self.sender.send(Message::OpenSettings).unwrap();
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.direction == MessageDirection::FromWidget {
                if message.destination() == self.camera_projection {
                    if *index == 0 {
                        self.sender
                            .send(Message::SetEditorCameraProjection(Projection::Perspective(
                                Default::default(),
                            )))
                            .unwrap()
                    } else {
                        self.sender
                            .send(Message::SetEditorCameraProjection(
                                Projection::Orthographic(Default::default()),
                            ))
                            .unwrap()
                    }
                } else if message.destination() == self.build_profile {
                    if *index == 0 {
                        self.sender
                            .send(Message::SetBuildProfile(BuildProfile::Debug))
                            .unwrap();
                    } else {
                        self.sender
                            .send(Message::SetBuildProfile(BuildProfile::Release))
                            .unwrap();
                    }
                }
            }
        }

        if let (Some(editor_scene), Some(msg), Mode::Edit) =
            (editor_scene, message.data::<WidgetMessage>(), mode)
        {
            if message.destination() == self.frame() {
                match *msg {
                    WidgetMessage::MouseDown { button, pos, .. } => self.on_mouse_down(
                        button,
                        pos,
                        editor_scene,
                        interaction_mode,
                        engine,
                        settings,
                    ),
                    WidgetMessage::MouseUp { button, pos, .. } => self.on_mouse_up(
                        button,
                        pos,
                        editor_scene,
                        interaction_mode,
                        engine,
                        settings,
                    ),
                    WidgetMessage::MouseWheel { amount, .. } => {
                        editor_scene
                            .camera_controller
                            .on_mouse_wheel(amount, &mut engine.scenes[editor_scene.scene].graph);
                    }
                    WidgetMessage::MouseMove { pos, .. } => {
                        self.on_mouse_move(pos, editor_scene, interaction_mode, engine, settings)
                    }
                    WidgetMessage::KeyUp(key) => {
                        if self.on_key_up(key, editor_scene, interaction_mode, engine) {
                            message.set_handled(true);
                        }
                    }
                    WidgetMessage::KeyDown(key) => {
                        if self.on_key_down(key, editor_scene, interaction_mode, engine) {
                            message.set_handled(true);
                        }
                    }
                    WidgetMessage::MouseLeave => {
                        if let Some(preview) = self.preview_instance.take() {
                            let scene = &mut engine.scenes[editor_scene.scene];

                            scene.graph.remove_node(preview.instance);
                        }
                    }
                    WidgetMessage::DragOver(handle) => {
                        match self.preview_instance.as_ref() {
                            None => {
                                if let Some(item) =
                                    engine.user_interface.node(handle).cast::<AssetItem>()
                                {
                                    // Make sure all resources loaded with relative paths only.
                                    // This will make scenes portable.
                                    if let Ok(relative_path) = make_relative_path(&item.path) {
                                        if let AssetKind::Model = item.kind {
                                            // No model was loaded yet, do it.
                                            if let Ok(model) =
                                                fyrox::core::futures::executor::block_on(
                                                    engine
                                                        .resource_manager
                                                        .request_model(relative_path),
                                                )
                                            {
                                                let scene = &mut engine.scenes[editor_scene.scene];

                                                // Instantiate the model.
                                                let instance = model.instantiate(scene);

                                                scene.graph[instance]
                                                    .local_transform_mut()
                                                    .set_scale(settings.model.instantiation_scale);

                                                let nodes = scene
                                                    .graph
                                                    .traverse_handle_iter(instance)
                                                    .collect::<FxHashSet<Handle<Node>>>();

                                                // Disable animations and state machines.
                                                for handle in nodes.iter() {
                                                    let node = &mut scene.graph[*handle];
                                                    if let Some(animation_player) =
                                                        node.query_component_mut::<AnimationPlayer>()
                                                    {
                                                        for animation in animation_player
                                                            .animations_mut()
                                                            .get_value_mut_silent()
                                                            .iter_mut()
                                                        {
                                                            animation.set_enabled(false);
                                                        }
                                                    } else if let Some(absm) =
                                                        node.query_component_mut::<AnimationBlendingStateMachine>()
                                                    {
                                                        absm.set_enabled(false);
                                                    }
                                                }

                                                self.preview_instance =
                                                    Some(PreviewInstance { instance, nodes });
                                            }
                                        }
                                    }
                                }
                            }
                            Some(preview) => {
                                let screen_bounds = self.frame_bounds(&engine.user_interface);
                                let frame_size = screen_bounds.size;
                                let cursor_pos = engine.user_interface.cursor_position();
                                let rel_pos = cursor_pos - screen_bounds.position;
                                let graph = &mut engine.scenes[editor_scene.scene].graph;
                                if let Some(result) =
                                    editor_scene.camera_controller.pick(PickingOptions {
                                        cursor_pos: rel_pos,
                                        graph,
                                        editor_objects_root: editor_scene.editor_objects_root,
                                        screen_size: frame_size,
                                        editor_only: false,
                                        filter: |handle, _| !preview.nodes.contains(&handle),
                                        ignore_back_faces: settings.selection.ignore_back_faces,
                                        // We need info only about closest intersection.
                                        use_picking_loop: false,
                                        only_meshes: false,
                                    })
                                {
                                    graph[preview.instance]
                                        .local_transform_mut()
                                        .set_position(result.position);
                                } else {
                                    // In case of empty space, check intersection with oXZ plane (3D) or oXY (2D).
                                    if let Some(camera) = graph
                                        [editor_scene.camera_controller.camera]
                                        .cast::<Camera>()
                                    {
                                        let normal = match camera.projection() {
                                            Projection::Perspective(_) => {
                                                Vector3::new(0.0, 1.0, 0.0)
                                            }
                                            Projection::Orthographic(_) => {
                                                Vector3::new(0.0, 0.0, 1.0)
                                            }
                                        };

                                        let plane = Plane::from_normal_and_point(
                                            &normal,
                                            &Default::default(),
                                        )
                                        .unwrap_or_default();

                                        let ray = camera.make_ray(rel_pos, frame_size);

                                        if let Some(point) = ray.plane_intersection_point(&plane) {
                                            graph[preview.instance]
                                                .local_transform_mut()
                                                .set_position(point);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    WidgetMessage::Drop(handle) => {
                        self.on_drop(handle, engine, editor_scene, settings)
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn sync_to_model(&self, editor_scene: &EditorScene, engine: &Engine) {
        if let Selection::Graph(ref selection) = editor_scene.selection {
            let scene = &engine.scenes[editor_scene.scene];
            if let Some((_, position)) = selection.global_rotation_position(&scene.graph) {
                engine.user_interface.send_message(Vec3EditorMessage::value(
                    self.global_position_display,
                    MessageDirection::ToWidget,
                    position,
                ));
            }
        }
    }

    pub fn on_mode_changed(&self, ui: &UserInterface, mode: &Mode) {
        let enabled = mode.is_edit();
        ui.send_message(ButtonMessage::content(
            self.switch_mode,
            MessageDirection::ToWidget,
            ButtonContent::text(if enabled { "Play" } else { "Stop" }),
        ));
        for widget in [self.interaction_mode_panel, self.contextual_actions] {
            enable_widget(widget, enabled, ui);
        }
    }

    pub fn set_render_target(&self, ui: &UserInterface, render_target: Option<Texture>) {
        ui.send_message(ImageMessage::texture(
            self.frame,
            MessageDirection::ToWidget,
            render_target.map(into_gui_texture),
        ));
    }

    pub fn set_title(&self, ui: &UserInterface, title: String) {
        ui.send_message(WindowMessage::title(
            self.window,
            MessageDirection::ToWidget,
            WindowTitle::Text(title),
        ));
    }

    pub fn reset_camera_projection(&self, ui: &UserInterface) {
        // Default camera projection is Perspective.
        ui.send_message(DropdownListMessage::selection(
            self.camera_projection,
            MessageDirection::ToWidget,
            Some(0),
        ));
    }

    pub fn frame_bounds(&self, ui: &UserInterface) -> Rect<f32> {
        ui.node(self.frame).screen_bounds()
    }

    #[must_use]
    fn on_key_up(
        &mut self,
        key: KeyCode,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
    ) -> bool {
        if editor_scene.camera_controller.on_key_up(key) {
            return true;
        }

        if let Some(interaction_mode) = active_interaction_mode {
            if interaction_mode.on_key_up(key, editor_scene, engine) {
                return true;
            }
        }

        false
    }

    #[must_use]
    fn on_key_down(
        &mut self,
        key: KeyCode,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
    ) -> bool {
        if editor_scene.camera_controller.on_key_down(key) {
            return true;
        }

        if let Some(interaction_mode) = active_interaction_mode {
            if interaction_mode.on_key_down(key, editor_scene, engine) {
                return true;
            }
        }

        false
    }

    fn on_mouse_move(
        &mut self,
        pos: Vector2<f32>,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        let screen_bounds = self.frame_bounds(&engine.user_interface);

        let last_pos = *self.last_mouse_pos.get_or_insert(pos);
        let mouse_offset = pos - last_pos;
        editor_scene
            .camera_controller
            .on_mouse_move(mouse_offset, &settings.camera);
        let rel_pos = pos - screen_bounds.position;

        if let Some(interaction_mode) = active_interaction_mode {
            interaction_mode.on_mouse_move(
                mouse_offset,
                rel_pos,
                editor_scene.camera_controller.camera,
                editor_scene,
                engine,
                screen_bounds.size,
                settings,
            );
        }

        self.last_mouse_pos = Some(pos);
    }

    fn on_mouse_up(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        engine.user_interface.release_mouse_capture();

        let screen_bounds = self.frame_bounds(&engine.user_interface);

        if button == MouseButton::Left {
            self.click_mouse_pos = None;
            if let Some(current_im) = active_interaction_mode {
                let rel_pos = pos - screen_bounds.position;
                current_im.on_left_mouse_button_up(
                    editor_scene,
                    engine,
                    rel_pos,
                    screen_bounds.size,
                    settings,
                );
            }
        }

        editor_scene.camera_controller.on_mouse_button_up(button);
    }

    fn on_mouse_down(
        &mut self,
        button: MouseButton,
        pos: Vector2<f32>,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        engine.user_interface.capture_mouse(self.frame());

        let screen_bounds = self.frame_bounds(&engine.user_interface);

        if button == MouseButton::Left {
            if let Some(current_im) = active_interaction_mode {
                let rel_pos = pos - screen_bounds.position;

                self.click_mouse_pos = Some(rel_pos);

                current_im.on_left_mouse_button_down(
                    editor_scene,
                    engine,
                    rel_pos,
                    screen_bounds.size,
                    settings,
                );
            }
        }

        editor_scene.camera_controller.on_mouse_button_down(button);
    }

    fn on_drop(
        &mut self,
        handle: Handle<UiNode>,
        engine: &mut Engine,
        editor_scene: &mut EditorScene,
        settings: &Settings,
    ) {
        if handle.is_none() {
            return;
        }

        let screen_bounds = self.frame_bounds(&engine.user_interface);
        let frame_size = screen_bounds.size;

        if let Some(item) = engine.user_interface.node(handle).cast::<AssetItem>() {
            // Make sure all resources loaded with relative paths only.
            // This will make scenes portable.
            if let Ok(relative_path) = make_relative_path(&item.path) {
                match item.kind {
                    AssetKind::Model => {
                        if let Some(preview) = self.preview_instance.take() {
                            let scene = &mut engine.scenes[editor_scene.scene];

                            // Immediately after extract if from the scene to subgraph. This is required to not violate
                            // the rule of one place of execution, only commands allowed to modify the scene.
                            let sub_graph = scene.graph.take_reserve_sub_graph(preview.instance);

                            let group = vec![
                                SceneCommand::new(AddModelCommand::new(sub_graph)),
                                // We also want to select newly instantiated model.
                                SceneCommand::new(ChangeSelectionCommand::new(
                                    Selection::Graph(GraphSelection::single_or_empty(
                                        preview.instance,
                                    )),
                                    editor_scene.selection.clone(),
                                )),
                            ];

                            self.sender
                                .send(Message::do_scene_command(CommandGroup::from(group)))
                                .unwrap();
                        }
                    }
                    AssetKind::Texture => {
                        let cursor_pos = engine.user_interface.cursor_position();
                        let rel_pos = cursor_pos - screen_bounds.position;
                        let graph = &engine.scenes[editor_scene.scene].graph;
                        if let Some(result) = editor_scene.camera_controller.pick(PickingOptions {
                            cursor_pos: rel_pos,
                            graph,
                            editor_objects_root: editor_scene.editor_objects_root,
                            screen_size: frame_size,
                            editor_only: false,
                            filter: |_, _| true,
                            ignore_back_faces: settings.selection.ignore_back_faces,
                            use_picking_loop: true,
                            only_meshes: false,
                        }) {
                            let tex = engine.resource_manager.request_texture(relative_path);
                            let texture = tex.clone();
                            let texture = texture.state();
                            if let TextureState::Ok(_) = *texture {
                                let node =
                                    &mut engine.scenes[editor_scene.scene].graph[result.node];

                                if node.is_mesh() {
                                    self.sender
                                        .send(Message::do_scene_command(
                                            SetMeshTextureCommand::new(result.node, tex),
                                        ))
                                        .unwrap();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
