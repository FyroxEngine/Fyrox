use crate::{
    camera::PickingOptions,
    gui::{make_dropdown_list_option, make_dropdown_list_option_with_height},
    load_image,
    message::MessageSender,
    scene_viewer::gizmo::{SceneGizmo, SceneGizmoAction},
    send_sync_message,
    settings::keys::KeyBindings,
    utils::enable_widget,
    AddModelCommand, AssetItem, BuildProfile, ChangeSelectionCommand, CommandGroup,
    DropdownListBuilder, EditorScene, EditorSceneEntry, GraphSelection, InteractionMode, Message,
    Mode, SaveSceneConfirmationDialogAction, SceneCommand, SceneContainer, Selection,
    SetMeshTextureCommand, Settings,
};
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        futures::executor::block_on,
        make_relative_path,
        math::{plane::Plane, Rect},
        pool::Handle,
        uuid::Uuid,
    },
    engine::Engine,
    fxhash::{FxHashMap, FxHashSet},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{Button, ButtonBuilder, ButtonMessage},
        canvas::CanvasBuilder,
        decorator::DecoratorMessage,
        dropdown_list::DropdownListMessage,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::{ImageBuilder, ImageMessage},
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        tab_control::{
            Tab, TabControl, TabControlBuilder, TabControlMessage, TabDefinition, TabUserData,
        },
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        vec::{Vec3EditorBuilder, Vec3EditorMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        BRUSH_DARKEST,
    },
    resource::{
        model::{Model, ModelResourceExtension},
        texture::{Texture, TextureResource},
    },
    scene::{
        camera::{Camera, Projection},
        node::Node,
        Scene,
    },
    utils::into_gui_texture,
};
use std::cmp::Ordering;

mod gizmo;

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
    interaction_modes: FxHashMap<Uuid, Handle<UiNode>>,
    camera_projection: Handle<UiNode>,
    play: Handle<UiNode>,
    stop: Handle<UiNode>,
    build_profile: Handle<UiNode>,
    sender: MessageSender,
    interaction_mode_panel: Handle<UiNode>,
    contextual_actions: Handle<UiNode>,
    global_position_display: Handle<UiNode>,
    preview_instance: Option<PreviewInstance>,
    no_scene_reminder: Handle<UiNode>,
    tab_control: Handle<UiNode>,
    scene_gizmo: SceneGizmo,
    scene_gizmo_image: Handle<UiNode>,
}

impl SceneViewer {
    pub fn new(engine: &mut Engine, sender: MessageSender) -> Self {
        let scene_gizmo = SceneGizmo::new(engine);

        let ctx = &mut engine.user_interface.build_ctx();

        let frame;
        let selection_frame;
        let camera_projection;
        let play;
        let stop;
        let build_profile;

        let interaction_mode_panel = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_horizontal_alignment(HorizontalAlignment::Left),
        )
        .build(ctx);

        let global_position_display;
        let contextual_actions = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    camera_projection = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(40.0),
                    )
                    .with_items(vec![
                        make_dropdown_list_option_with_height(ctx, "3D", 22.0),
                        make_dropdown_list_option_with_height(ctx, "2D", 22.0),
                    ])
                    .with_selected(0)
                    .build(ctx);
                    camera_projection
                })
                .with_child({
                    global_position_display = Vec3EditorBuilder::<f32>::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_tooltip(make_simple_tooltip(
                                ctx,
                                "Global Coordinates of the Current Selection",
                            ))
                            .with_width(200.0),
                    )
                    .with_editable(false)
                    .build(ctx);
                    global_position_display
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
                                build_profile = DropdownListBuilder::new(
                                    WidgetBuilder::new()
                                        .with_tooltip(make_simple_tooltip(
                                            ctx,
                                            "Build Configuration\nDefines cargo build \
                                            profile - debug or release.",
                                        ))
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(90.0),
                                )
                                .with_items(vec![
                                    make_dropdown_list_option(ctx, "Debug"),
                                    make_dropdown_list_option(ctx, "Release"),
                                ])
                                .with_selected(0)
                                .build(ctx);
                                build_profile
                            })
                            .with_child({
                                play = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(26.0),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(16.0)
                                            .with_height(16.0)
                                            .with_margin(Thickness::uniform(4.0))
                                            .with_background(Brush::Solid(Color::opaque(
                                                0, 200, 0,
                                            ))),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/play.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                play
                            })
                            .with_child({
                                stop = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(26.0),
                                )
                                .with_content(
                                    ImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(16.0)
                                            .with_height(16.0)
                                            .with_margin(Thickness::uniform(4.0))
                                            .with_background(Brush::Solid(Color::opaque(
                                                200, 0, 0,
                                            ))),
                                    )
                                    .with_opt_texture(load_image(include_bytes!(
                                        "../../resources/stop.png"
                                    )))
                                    .build(ctx),
                                )
                                .build(ctx);
                                stop
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

        let no_scene_reminder = TextBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_foreground(BRUSH_DARKEST),
        )
        .with_text("No scene loaded. Create a new scene (File -> New Scene) or load existing (File -> Load Scene)")
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Center)
        .with_wrap(WrapMode::Word)
        .build(ctx);

        let scene_gizmo_image = ImageBuilder::new(
            WidgetBuilder::new()
                .with_width(85.0)
                .with_height(85.0)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_flip(true)
        .with_texture(into_gui_texture(scene_gizmo.render_target.clone()))
        .build(ctx);

        let tab_control;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("SceneViewer"))
            .can_close(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .on_row(0)
                        .with_child(top_ribbon)
                        .with_child({
                            tab_control =
                                TabControlBuilder::new(WidgetBuilder::new().on_row(1)).build(ctx);
                            tab_control
                        })
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .with_child({
                                        frame = ImageBuilder::new(
                                            WidgetBuilder::new()
                                                .with_child(no_scene_reminder)
                                                .with_child(interaction_mode_panel)
                                                .with_child(scene_gizmo_image)
                                                .with_allow_drop(true),
                                        )
                                        .with_flip(true)
                                        .build(ctx);
                                        frame
                                    })
                                    .with_child(
                                        CanvasBuilder::new(WidgetBuilder::new().with_child({
                                            selection_frame = BorderBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_visibility(false)
                                                    .with_background(Brush::Solid(
                                                        Color::from_rgba(255, 255, 255, 40),
                                                    ))
                                                    .with_foreground(Brush::Solid(Color::opaque(
                                                        0, 255, 0,
                                                    ))),
                                            )
                                            .with_stroke_thickness(Thickness::uniform(1.0))
                                            .build(ctx);
                                            selection_frame
                                        }))
                                        .build(ctx),
                                    ),
                            )
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
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
            interaction_modes: Default::default(),
            selection_frame,
            camera_projection,
            click_mouse_pos: None,
            play,
            interaction_mode_panel,
            contextual_actions,
            global_position_display,
            build_profile,
            preview_instance: None,
            stop,
            no_scene_reminder,
            tab_control,
            scene_gizmo,
            scene_gizmo_image,
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
            if let Some(&active_button) = self.interaction_modes.get(mode) {
                for &mode_button in self.interaction_modes.values() {
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
    }

    pub fn on_current_scene_changed(
        &mut self,
        new_scene: Option<&mut EditorSceneEntry>,
        ui: &mut UserInterface,
    ) {
        // Remove interaction mode buttons first.
        for (_, button) in self.interaction_modes.drain() {
            ui.send_message(WidgetMessage::remove(button, MessageDirection::ToWidget));
        }

        // Create new buttons for each mode.
        if let Some(scene_entry) = new_scene {
            for (id, mode) in scene_entry.interaction_modes.map.iter_mut() {
                let button = mode.make_button(
                    &mut ui.build_ctx(),
                    scene_entry.current_interaction_mode.unwrap_or_default() == *id,
                );
                ui.send_message(WidgetMessage::link(
                    button,
                    MessageDirection::ToWidget,
                    self.interaction_mode_panel,
                ));
                self.interaction_modes.insert(*id, button);
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &mut UiMessage,
        engine: &mut Engine,
        scenes: &mut SceneContainer,
        settings: &Settings,
        mode: &Mode,
    ) {
        let ui = &engine.user_interface;

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            for (mode_id, mode_button) in self.interaction_modes.iter() {
                if message.destination() == *mode_button {
                    self.sender.send(Message::SetInteractionMode(*mode_id));
                }
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) =
            message.data::<WidgetMessage>()
        {
            for &mode_button in self.interaction_modes.values() {
                if ui.is_node_child_of(message.destination(), mode_button)
                    && *button == MouseButton::Right
                {
                    self.sender.send(Message::OpenSettings);
                }
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.direction == MessageDirection::FromWidget {
                if message.destination() == self.camera_projection {
                    if *index == 0 {
                        self.sender.send(Message::SetEditorCameraProjection(
                            Projection::Perspective(Default::default()),
                        ))
                    } else {
                        self.sender.send(Message::SetEditorCameraProjection(
                            Projection::Orthographic(Default::default()),
                        ))
                    }
                } else if message.destination() == self.build_profile {
                    if *index == 0 {
                        self.sender
                            .send(Message::SetBuildProfile(BuildProfile::Debug));
                    } else {
                        self.sender
                            .send(Message::SetBuildProfile(BuildProfile::Release));
                    }
                }
            }
        } else if let Some(msg) = message.data::<TabControlMessage>() {
            if message.destination() == self.tab_control
                && message.direction() == MessageDirection::FromWidget
            {
                match msg {
                    TabControlMessage::CloseTab(tab_index) => {
                        if let Some(entry) = scenes.try_get(*tab_index) {
                            if entry.editor_scene.need_save() {
                                self.sender.send(Message::OpenSaveSceneConfirmationDialog {
                                    scene: entry.editor_scene.scene,
                                    action: SaveSceneConfirmationDialogAction::CloseScene(
                                        entry.editor_scene.scene,
                                    ),
                                });
                            } else {
                                self.sender
                                    .send(Message::CloseScene(entry.editor_scene.scene));
                            }
                        }
                    }
                    TabControlMessage::ActiveTab(Some(active_tab)) => {
                        if let Some(entry) = scenes.try_get(*active_tab) {
                            self.sender
                                .send(Message::SetCurrentScene(entry.editor_scene.scene));
                        }
                    }
                    _ => (),
                }
            }
        }

        if let Some(entry) = scenes.current_scene_entry_mut() {
            let editor_scene = &mut entry.editor_scene;
            let interaction_mode = entry
                .current_interaction_mode
                .and_then(|uuid| entry.interaction_modes.map.get_mut(&uuid));

            if let (Some(msg), Mode::Edit) = (message.data::<WidgetMessage>(), mode) {
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
                            editor_scene.camera_controller.on_mouse_wheel(
                                amount * settings.camera.zoom_speed,
                                &mut engine.scenes[editor_scene.scene].graph,
                                settings,
                            );
                        }
                        WidgetMessage::MouseMove { pos, .. } => self.on_mouse_move(
                            pos,
                            editor_scene,
                            interaction_mode,
                            engine,
                            settings,
                        ),
                        WidgetMessage::KeyUp(key) => {
                            if self.on_key_up(
                                key,
                                editor_scene,
                                interaction_mode,
                                engine,
                                &settings.key_bindings,
                            ) {
                                message.set_handled(true);
                            }
                        }
                        WidgetMessage::KeyDown(key) => {
                            if self.on_key_down(
                                key,
                                editor_scene,
                                interaction_mode,
                                engine,
                                &settings.key_bindings,
                            ) {
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
                                            // No model was loaded yet, do it.
                                            if let Some(model) = engine
                                                .resource_manager
                                                .try_request::<Model>(relative_path)
                                                .and_then(|m| block_on(m).ok())
                                            {
                                                let scene = &mut engine.scenes[editor_scene.scene];

                                                // Instantiate the model.
                                                let instance = model.instantiate(scene);

                                                scene.graph.link_nodes(
                                                    instance,
                                                    editor_scene.scene_content_root,
                                                );

                                                scene.graph[instance]
                                                    .local_transform_mut()
                                                    .set_scale(settings.model.instantiation_scale);

                                                let nodes = scene
                                                    .graph
                                                    .traverse_handle_iter(instance)
                                                    .collect::<FxHashSet<Handle<Node>>>();

                                                self.preview_instance =
                                                    Some(PreviewInstance { instance, nodes });
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

                                    let position = if let Some(result) =
                                        editor_scene.camera_controller.pick(PickingOptions {
                                            cursor_pos: rel_pos,
                                            graph,
                                            editor_objects_root: editor_scene.editor_objects_root,
                                            scene_content_root: editor_scene.scene_content_root,
                                            screen_size: frame_size,
                                            editor_only: false,
                                            filter: |handle, _| !preview.nodes.contains(&handle),
                                            ignore_back_faces: settings.selection.ignore_back_faces,
                                            // We need info only about closest intersection.
                                            use_picking_loop: false,
                                            only_meshes: false,
                                        }) {
                                        Some(result.position)
                                    } else {
                                        // In case of empty space, check intersection with oXZ plane (3D) or oXY (2D).
                                        let camera = graph[editor_scene.camera_controller.camera]
                                            .query_component_ref::<Camera>()
                                            .unwrap();

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

                                        ray.plane_intersection_point(&plane)
                                    };

                                    if let Some(position) = position {
                                        graph[preview.instance].local_transform_mut().set_position(
                                            settings
                                                .move_mode_settings
                                                .try_snap_vector_to_grid(position),
                                        );
                                    }
                                }
                            }
                        }
                        WidgetMessage::Drop(handle) => {
                            self.on_drop(handle, engine, editor_scene, settings)
                        }
                        _ => {}
                    }
                } else if message.destination() == self.scene_gizmo_image {
                    match *msg {
                        WidgetMessage::MouseDown { button, pos, .. } => {
                            if button == MouseButton::Left {
                                let rel_pos = pos
                                    - engine
                                        .user_interface
                                        .node(self.scene_gizmo_image)
                                        .screen_position();
                                if let Some(action) = self.scene_gizmo.on_click(rel_pos, engine) {
                                    match action {
                                        SceneGizmoAction::Rotate(rotation) => {
                                            editor_scene.camera_controller.pitch = rotation.pitch;
                                            editor_scene.camera_controller.yaw = rotation.yaw;
                                        }
                                        SceneGizmoAction::SwitchProjection => {
                                            let graph = &engine.scenes[editor_scene.scene].graph;
                                            match graph[editor_scene.camera_controller.camera]
                                                .as_camera()
                                                .projection()
                                            {
                                                Projection::Perspective(_) => {
                                                    ui.send_message(
                                                        DropdownListMessage::selection(
                                                            self.camera_projection,
                                                            MessageDirection::ToWidget,
                                                            Some(1),
                                                        ),
                                                    );
                                                }
                                                Projection::Orthographic(_) => {
                                                    ui.send_message(
                                                        DropdownListMessage::selection(
                                                            self.camera_projection,
                                                            MessageDirection::ToWidget,
                                                            Some(0),
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            let rel_pos = pos
                                - engine
                                    .user_interface
                                    .node(self.scene_gizmo_image)
                                    .screen_position();
                            self.scene_gizmo.on_mouse_move(rel_pos, engine);
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    pub fn sync_to_model(&self, scenes: &SceneContainer, engine: &mut Engine) {
        // Sync tabs first.
        fn fetch_tab_scene_handle(tab: &Tab) -> Handle<Scene> {
            tab.user_data
                .as_ref()
                .unwrap()
                .0
                .downcast_ref::<Handle<Scene>>()
                .cloned()
                .unwrap()
        }

        let tabs = engine
            .user_interface
            .node(self.tab_control)
            .query_component::<TabControl>()
            .expect("Must be TabControl!")
            .tabs
            .clone();
        match tabs.len().cmp(&scenes.len()) {
            Ordering::Less => {
                // Some scenes were added.
                for entry in scenes.iter() {
                    if tabs
                        .iter()
                        .all(|tab| fetch_tab_scene_handle(tab) != entry.editor_scene.scene)
                    {
                        let header =
                            TextBuilder::new(WidgetBuilder::new().with_margin(Thickness {
                                left: 4.0,
                                top: 2.0,
                                right: 4.0,
                                bottom: 2.0,
                            }))
                            .with_text(entry.editor_scene.name())
                            .build(&mut engine.user_interface.build_ctx());

                        send_sync_message(
                            &engine.user_interface,
                            TabControlMessage::add_tab(
                                self.tab_control,
                                MessageDirection::ToWidget,
                                TabDefinition {
                                    header,
                                    content: Default::default(),
                                    can_be_closed: true,
                                    user_data: Some(TabUserData::new(entry.editor_scene.scene)),
                                },
                            ),
                        );
                    }
                }
            }
            Ordering::Equal => {
                // Nothing to do.
            }
            Ordering::Greater => {
                // Some scenes were removed.
                for (tab_index, tab) in tabs.iter().enumerate() {
                    let tab_scene = fetch_tab_scene_handle(tab);
                    if scenes.iter().all(|s| tab_scene != s.editor_scene.scene) {
                        send_sync_message(
                            &engine.user_interface,
                            TabControlMessage::remove_tab(
                                self.tab_control,
                                MessageDirection::ToWidget,
                                tab_index,
                            ),
                        );
                    }
                }
            }
        }

        for tab in tabs.iter() {
            if let Some(scene) = scenes.entry_by_scene_handle(fetch_tab_scene_handle(tab)) {
                engine.user_interface.send_message(TextMessage::text(
                    tab.header_content,
                    MessageDirection::ToWidget,
                    format!(
                        "{}{}",
                        scene.editor_scene.name(),
                        if scene.editor_scene.need_save() {
                            "*"
                        } else {
                            ""
                        }
                    ),
                ));
            }
        }

        send_sync_message(
            &engine.user_interface,
            TabControlMessage::active_tab(
                self.tab_control,
                MessageDirection::ToWidget,
                scenes.current_scene_index(),
            ),
        );

        // Then sync to the current scene.
        if let Some(editor_scene) = scenes.current_editor_scene_ref() {
            let scene = &engine.scenes[editor_scene.scene];

            self.set_title(
                &engine.user_interface,
                format!(
                    "Scene Preview - {}",
                    editor_scene
                        .path
                        .as_ref()
                        .map_or("Unnamed Scene".to_string(), |p| p
                            .to_string_lossy()
                            .to_string())
                ),
            );

            self.set_render_target(
                &engine.user_interface,
                scene.rendering_options.render_target.clone(),
            );

            if let Selection::Graph(ref selection) = editor_scene.selection {
                if let Some((_, position)) = selection.global_rotation_position(&scene.graph) {
                    engine.user_interface.send_message(Vec3EditorMessage::value(
                        self.global_position_display,
                        MessageDirection::ToWidget,
                        position,
                    ));
                }
            }
        }

        send_sync_message(
            &engine.user_interface,
            WidgetMessage::visibility(
                self.no_scene_reminder,
                MessageDirection::ToWidget,
                scenes.current_editor_scene_ref().is_none(),
            ),
        );
    }

    pub fn on_mode_changed(&self, ui: &UserInterface, mode: &Mode) {
        let enabled = mode.is_edit();
        for widget in [self.interaction_mode_panel, self.contextual_actions] {
            enable_widget(widget, enabled, ui);
        }

        ui.send_message(WidgetMessage::enabled(
            self.play,
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
        ui.send_message(WidgetMessage::enabled(
            self.stop,
            MessageDirection::ToWidget,
            !mode.is_edit(),
        ));
    }

    pub fn set_render_target(&self, ui: &UserInterface, render_target: Option<TextureResource>) {
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
        key_bindings: &KeyBindings,
    ) -> bool {
        if editor_scene.camera_controller.on_key_up(key_bindings, key) {
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
        key_bindings: &KeyBindings,
    ) -> bool {
        if editor_scene
            .camera_controller
            .on_key_down(key_bindings, key)
        {
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

        editor_scene
            .camera_controller
            .on_mouse_button_up(button, &mut engine.scenes[editor_scene.scene].graph);
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

        editor_scene.camera_controller.on_mouse_button_down(
            button,
            engine.user_interface.keyboard_modifiers(),
            &mut engine.scenes[editor_scene.scene].graph,
        );
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
                if let Some(preview) = self.preview_instance.take() {
                    let scene = &mut engine.scenes[editor_scene.scene];

                    // Immediately after extract if from the scene to subgraph. This is required to not violate
                    // the rule of one place of execution, only commands allowed to modify the scene.
                    let sub_graph = scene.graph.take_reserve_sub_graph(preview.instance);

                    let group = vec![
                        SceneCommand::new(AddModelCommand::new(sub_graph)),
                        // We also want to select newly instantiated model.
                        SceneCommand::new(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(preview.instance)),
                            editor_scene.selection.clone(),
                        )),
                    ];

                    self.sender.do_scene_command(CommandGroup::from(group));
                } else if let Some(tex) = engine
                    .resource_manager
                    .try_request::<Texture>(relative_path)
                    .and_then(|t| block_on(t).ok())
                {
                    let cursor_pos = engine.user_interface.cursor_position();
                    let rel_pos = cursor_pos - screen_bounds.position;
                    let graph = &engine.scenes[editor_scene.scene].graph;
                    if let Some(result) = editor_scene.camera_controller.pick(PickingOptions {
                        cursor_pos: rel_pos,
                        graph,
                        editor_objects_root: editor_scene.editor_objects_root,
                        scene_content_root: editor_scene.scene_content_root,
                        screen_size: frame_size,
                        editor_only: false,
                        filter: |_, _| true,
                        ignore_back_faces: settings.selection.ignore_back_faces,
                        use_picking_loop: true,
                        only_meshes: false,
                    }) {
                        let texture = tex.clone();
                        let mut texture = texture.state();
                        if texture.data().is_some() {
                            let node = &mut engine.scenes[editor_scene.scene].graph[result.node];

                            if node.is_mesh() {
                                self.sender
                                    .do_scene_command(SetMeshTextureCommand::new(result.node, tex));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update(&self, editor_scene: &EditorScene, engine: &mut Engine) {
        self.scene_gizmo.sync_rotations(editor_scene, engine);
    }
}
