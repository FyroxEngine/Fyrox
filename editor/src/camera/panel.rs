use crate::fyrox::graph::SceneGraph;
use crate::fyrox::gui::{HorizontalAlignment, Thickness};
use crate::fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, UiNode, VerticalAlignment,
    },
    scene::{camera::Camera, node::Node},
};
use crate::{
    scene::{GameScene, Selection},
    send_sync_message, Message,
};

pub struct CameraPreviewControlPanel {
    pub window: Handle<UiNode>,
    preview: Handle<UiNode>,
    cameras_state: Vec<(Handle<Node>, Node)>,
    scene_viewer_frame: Handle<UiNode>,
}

impl CameraPreviewControlPanel {
    pub fn new(scene_viewer_frame: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let preview;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("CameraPanel"))
            .with_title(WindowTitle::text("Camera Preview"))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_child({
                            preview = CheckBoxBuilder::new(WidgetBuilder::new())
                                .with_content(
                                    TextBuilder::new(
                                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Preview")
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx),
                                )
                                .build(ctx);
                            preview
                        }),
                )
                .with_orientation(Orientation::Vertical)
                .build(ctx),
            )
            .open(false)
            .build(ctx);

        Self {
            window,
            cameras_state: Default::default(),
            preview,
            scene_viewer_frame,
        }
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        if let Message::DoCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.leave_preview_mode(game_scene, engine);
        }

        if let Message::SelectionChanged { .. } = message {
            let scene = &engine.scenes[game_scene.scene];
            if let Some(selection) = editor_selection.as_graph() {
                let any_camera = selection
                    .nodes
                    .iter()
                    .any(|n| scene.graph.try_get_of_type::<Camera>(*n).is_some());
                if any_camera {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WindowMessage::open_and_align(
                            self.window,
                            MessageDirection::ToWidget,
                            self.scene_viewer_frame,
                            HorizontalAlignment::Right,
                            VerticalAlignment::Top,
                            Thickness::top_right(5.0),
                            false,
                            false,
                        ));
                } else {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WindowMessage::close(
                            self.window,
                            MessageDirection::ToWidget,
                        ));
                }
            }
        }
    }

    fn enter_preview_mode(
        &mut self,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        assert!(self.cameras_state.is_empty());

        let scene = &engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        if let Some(new_graph_selection) = editor_selection.as_graph() {
            // Enable cameras from new selection.
            for &node_handle in &new_graph_selection.nodes {
                if scene.graph.try_get_of_type::<Camera>(node_handle).is_some() {
                    self.cameras_state
                        .push((node_handle, scene.graph[node_handle].clone_box()));

                    assert!(node_overrides.insert(node_handle));

                    game_scene.preview_camera = node_handle;
                }
            }
        }
    }

    pub fn leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        let scene = &mut engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        for (camera_handle, original) in self.cameras_state.drain(..) {
            scene.graph[camera_handle] = original;

            assert!(node_overrides.remove(&camera_handle));
        }

        game_scene.preview_camera = Handle::NONE;

        send_sync_message(
            engine.user_interfaces.first(),
            CheckBoxMessage::checked(self.preview, MessageDirection::ToWidget, Some(false)),
        );
    }

    pub fn is_in_preview_mode(&self) -> bool {
        !self.cameras_state.is_empty()
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.preview
                && message.direction() == MessageDirection::FromWidget
            {
                if *value {
                    self.enter_preview_mode(editor_selection, game_scene, engine);
                } else {
                    self.leave_preview_mode(game_scene, engine);
                }
            }
        }
    }
}
