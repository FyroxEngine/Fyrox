use crate::{
    gui::make_dropdown_list_option_with_height, load_image, make_delete_selection_command,
    AddModelCommand, AssetItem, AssetKind, ChangeSelectionCommand, CommandGroup,
    DropdownListBuilder, EditorScene, GameEngine, GraphSelection, InteractionMode,
    InteractionModeKind, Message, Mode, PasteCommand, SceneCommand, Selection,
    SetMeshTextureCommand, SetParticleSystemTextureCommand, SetSpriteTextureCommand, Settings,
    SettingsSectionKind,
};
use fyrox::{
    core::{algebra::Vector2, color::Color, make_relative_path, math::Rect, pool::Handle},
    engine::Engine,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        canvas::CanvasBuilder,
        dropdown_list::DropdownListMessage,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::{ImageBuilder, ImageMessage},
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    resource::texture::{Texture, TextureState},
    scene::camera::Projection,
    utils::into_gui_texture,
};
use std::sync::mpsc::Sender;

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
    unload_plugins: Handle<UiNode>,
    reload_plugins: Handle<UiNode>,
    switch_mode: Handle<UiNode>,
    sender: Sender<Message>,
    interaction_mode_panel: Handle<UiNode>,
}

fn make_interaction_mode_button(
    ctx: &mut BuildContext,
    image: &[u8],
    tooltip: &str,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tooltip(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_max_size(Vector2::new(300.0, f32::MAX))
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(tooltip)
                            .build(ctx),
                        ),
                )
                .build(ctx),
            )
            .with_margin(Thickness::uniform(1.0)),
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
        let unload_plugins;
        let reload_plugins;
        let switch_mode;
        let interaction_mode_panel;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .on_row(0)
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .with_child({
                                        switch_mode = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(100.0),
                                        )
                                        .with_text("Play/Stop")
                                        .build(ctx);
                                        switch_mode
                                    })
                                    .with_child({
                                        unload_plugins = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(100.0),
                                        )
                                        .with_text("Unload Plugins")
                                        .build(ctx);
                                        unload_plugins
                                    })
                                    .with_child({
                                        reload_plugins = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(100.0),
                                        )
                                        .with_text("Reload Plugins")
                                        .build(ctx);
                                        reload_plugins
                                    })
                                    .with_child({
                                        camera_projection = DropdownListBuilder::new(
                                            WidgetBuilder::new().with_width(150.0),
                                        )
                                        .with_items(vec![
                                            make_dropdown_list_option_with_height(
                                                ctx,
                                                "Perspective (3D)",
                                                22.0,
                                            ),
                                            make_dropdown_list_option_with_height(
                                                ctx,
                                                "Orthographic (2D)",
                                                22.0,
                                            ),
                                        ])
                                        .with_selected(0)
                                        .build(ctx);
                                        camera_projection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
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
                                    .with_child({
                                        interaction_mode_panel = StackPanelBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(0)
                                                .on_column(0)
                                                .with_child({
                                                    select_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/select.png"
                                                        ),
                                                        select_mode_tooltip,
                                                    );
                                                    select_mode
                                                })
                                                .with_child({
                                                    move_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/move_arrow.png"
                                                        ),
                                                        move_mode_tooltip,
                                                    );
                                                    move_mode
                                                })
                                                .with_child({
                                                    rotate_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/rotate_arrow.png"
                                                        ),
                                                        rotate_mode_tooltip,
                                                    );
                                                    rotate_mode
                                                })
                                                .with_child({
                                                    scale_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/scale_arrow.png"
                                                        ),
                                                        scale_mode_tooltip,
                                                    );
                                                    scale_mode
                                                })
                                                .with_child({
                                                    navmesh_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/navmesh.png"
                                                        ),
                                                        navmesh_mode_tooltip,
                                                    );
                                                    navmesh_mode
                                                })
                                                .with_child({
                                                    terrain_mode = make_interaction_mode_button(
                                                        ctx,
                                                        include_bytes!(
                                                            "../resources/embed/terrain.png"
                                                        ),
                                                        terrain_mode_tooltip,
                                                    );
                                                    terrain_mode
                                                }),
                                        )
                                        .build(ctx);
                                        interaction_mode_panel
                                    }),
                            )
                            .add_row(Row::stretch())
                            .add_column(Column::auto())
                            .add_column(Column::stretch())
                            .build(ctx),
                        ),
                )
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
            move_mode,
            rotate_mode,
            scale_mode,
            selection_frame,
            select_mode,
            navmesh_mode,
            terrain_mode,
            camera_projection,
            click_mouse_pos: None,
            unload_plugins,
            reload_plugins,
            switch_mode,
            interaction_mode_panel,
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

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
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
            } else if message.destination() == self.unload_plugins {
                self.sender.send(Message::UnloadPlugins).unwrap()
            } else if message.destination() == self.reload_plugins {
                self.sender.send(Message::ReloadPlugins).unwrap()
            } else if message.destination() == self.switch_mode {
                self.sender.send(Message::SwitchMode).unwrap();
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) =
            message.data::<WidgetMessage>()
        {
            if ui.is_node_child_of(message.destination(), self.move_mode)
                && *button == MouseButton::Right
            {
                self.sender
                    .send(Message::OpenSettings(SettingsSectionKind::MoveModeSettings))
                    .unwrap();
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
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
            }
        }

        if let (Some(editor_scene), Some(msg), Mode::Edit) =
            (editor_scene, message.data::<WidgetMessage>(), mode)
        {
            if message.destination() == self.frame() {
                match *msg {
                    WidgetMessage::MouseDown { button, pos, .. } => {
                        self.on_mouse_down(button, pos, editor_scene, interaction_mode, engine)
                    }
                    WidgetMessage::MouseUp { button, pos, .. } => {
                        self.on_mouse_up(button, pos, editor_scene, interaction_mode, engine)
                    }
                    WidgetMessage::MouseWheel { amount, .. } => {
                        editor_scene
                            .camera_controller
                            .on_mouse_wheel(amount, &mut engine.scenes[editor_scene.scene].graph);
                    }
                    WidgetMessage::MouseMove { pos, .. } => {
                        self.on_mouse_move(pos, editor_scene, interaction_mode, engine, settings)
                    }
                    WidgetMessage::KeyUp(key) => {
                        self.on_key_up(key, editor_scene, interaction_mode, engine)
                    }
                    WidgetMessage::KeyDown(key) => {
                        self.on_key_down(key, editor_scene, interaction_mode, engine)
                    }
                    WidgetMessage::Drop(handle) => self.on_drop(handle, engine, editor_scene),
                    _ => {}
                }
            }
        }
    }

    pub fn on_mode_changed(&self, ui: &UserInterface, mode: &Mode) {
        let enabled = mode.is_edit();
        for widget in [
            self.unload_plugins,
            self.reload_plugins,
            self.interaction_mode_panel,
            self.camera_projection,
        ] {
            ui.send_message(WidgetMessage::enabled(
                widget,
                MessageDirection::ToWidget,
                enabled,
            ));
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

    pub fn frame_bounds(&self, ui: &UserInterface) -> Rect<f32> {
        ui.node(self.frame).screen_bounds()
    }

    fn on_key_up(
        &mut self,
        key: KeyCode,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
    ) {
        editor_scene.camera_controller.on_key_up(key);

        if let Some(interaction_mode) = active_interaction_mode {
            interaction_mode.on_key_up(key, editor_scene, engine);
        }
    }

    fn on_key_down(
        &mut self,
        key: KeyCode,
        editor_scene: &mut EditorScene,
        active_interaction_mode: Option<&mut Box<dyn InteractionMode>>,
        engine: &mut Engine,
    ) {
        editor_scene.camera_controller.on_key_down(key);

        if let Some(interaction_mode) = active_interaction_mode {
            interaction_mode.on_key_down(key, editor_scene, engine);
        }

        match key {
            KeyCode::Y if engine.user_interface.keyboard_modifiers().control => {
                self.sender.send(Message::RedoSceneCommand).unwrap();
            }
            KeyCode::Z if engine.user_interface.keyboard_modifiers().control => {
                self.sender.send(Message::UndoSceneCommand).unwrap();
            }
            KeyCode::Key1 => {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Select))
                    .unwrap();
            }
            KeyCode::Key2 => {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Move))
                    .unwrap();
            }
            KeyCode::Key3 => {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Rotate))
                    .unwrap();
            }
            KeyCode::Key4 => {
                self.sender
                    .send(Message::SetInteractionMode(InteractionModeKind::Scale))
                    .unwrap();
            }
            KeyCode::L if engine.user_interface.keyboard_modifiers().control => {
                self.sender.send(Message::OpenLoadSceneDialog).unwrap();
            }
            KeyCode::C if engine.user_interface.keyboard_modifiers().control => {
                if let Selection::Graph(graph_selection) = &editor_scene.selection {
                    editor_scene.clipboard.fill_from_selection(
                        graph_selection,
                        editor_scene.scene,
                        engine,
                    );
                }
            }
            KeyCode::V if engine.user_interface.keyboard_modifiers().control => {
                if !editor_scene.clipboard.is_empty() {
                    self.sender
                        .send(Message::do_scene_command(PasteCommand::new()))
                        .unwrap();
                }
            }
            KeyCode::Delete => {
                if !editor_scene.selection.is_empty() {
                    if let Selection::Graph(_) = editor_scene.selection {
                        self.sender
                            .send(Message::DoSceneCommand(make_delete_selection_command(
                                editor_scene,
                                engine,
                            )))
                            .unwrap();
                    }
                }
            }
            _ => (),
        }
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
        editor_scene.camera_controller.on_mouse_move(mouse_offset);
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
                );
            }
        }

        editor_scene.camera_controller.on_mouse_button_down(button);
    }

    fn on_drop(&self, handle: Handle<UiNode>, engine: &mut Engine, editor_scene: &mut EditorScene) {
        if handle.is_none() {
            return;
        }

        let screen_bounds = self.frame_bounds(&engine.user_interface);
        let frame_size = screen_bounds.size;

        if let Some(item) = engine.user_interface.node(handle).cast::<AssetItem>() {
            // Make sure all resources loaded with relative paths only.
            // This will make scenes portable.
            let relative_path = make_relative_path(&item.path);

            match item.kind {
                AssetKind::Model => {
                    // No model was loaded yet, do it.
                    if let Ok(model) = fyrox::core::futures::executor::block_on(
                        engine.resource_manager.request_model(&item.path),
                    ) {
                        let scene = &mut engine.scenes[editor_scene.scene];

                        // Instantiate the model.
                        let instance = model.instantiate(scene);
                        // Enable instantiated animations.
                        for &animation in instance.animations.iter() {
                            scene.animations[animation].set_enabled(true);
                        }

                        // Immediately after extract if from the scene to subgraph. This is required to not violate
                        // the rule of one place of execution, only commands allowed to modify the scene.
                        let sub_graph = scene.graph.take_reserve_sub_graph(instance.root);
                        let animations_container = instance
                            .animations
                            .iter()
                            .map(|&anim| scene.animations.take_reserve(anim))
                            .collect();

                        let group = vec![
                            SceneCommand::new(AddModelCommand::new(
                                sub_graph,
                                animations_container,
                            )),
                            // We also want to select newly instantiated model.
                            SceneCommand::new(ChangeSelectionCommand::new(
                                Selection::Graph(GraphSelection::single_or_empty(instance.root)),
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
                    if let Some(result) = editor_scene.camera_controller.pick(
                        rel_pos,
                        graph,
                        editor_scene.root,
                        frame_size,
                        false,
                        |_, _| true,
                    ) {
                        let tex = engine.resource_manager.request_texture(&relative_path);
                        let texture = tex.clone();
                        let texture = texture.state();
                        if let TextureState::Ok(_) = *texture {
                            let node = &mut engine.scenes[editor_scene.scene].graph[result.node];

                            if node.is_mesh() {
                                self.sender
                                    .send(Message::do_scene_command(SetMeshTextureCommand::new(
                                        result.node,
                                        tex,
                                    )))
                                    .unwrap();
                            } else if node.is_sprite() {
                                self.sender
                                    .send(Message::do_scene_command(SetSpriteTextureCommand::new(
                                        result.node,
                                        Some(tex),
                                    )))
                                    .unwrap();
                            } else if node.is_particle_system() {
                                self.sender
                                    .send(Message::do_scene_command(
                                        SetParticleSystemTextureCommand::new(
                                            result.node,
                                            Some(tex),
                                        ),
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
