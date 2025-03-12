// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use fyrox::gui::widget::WidgetMessage;

use crate::{
    fyrox::{
        core::pool::Handle,
        engine::Engine,
        graph::SceneGraph,
        gui::Thickness,
        gui::{
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            widget::WidgetBuilder,
            BuildContext, Orientation, UiNode, VerticalAlignment,
        },
        scene::{camera::Camera, node::Node},
    },
    scene::{GameScene, Selection},
    send_sync_message, Message,
};

pub struct CameraPreviewControlPanel {
    pub root_widget: Handle<UiNode>,
    preview: Handle<UiNode>,
    cameras_state: Vec<(Handle<Node>, Node)>,
}

impl CameraPreviewControlPanel {
    pub fn new(inspector_head: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let preview;
        let root_widget = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
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
        .build(ctx);

        ctx.send_message(WidgetMessage::link(
            root_widget,
            MessageDirection::ToWidget,
            inspector_head,
        ));

        Self {
            root_widget,
            cameras_state: Default::default(),
            preview,
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
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(WidgetMessage::visibility(
                        self.root_widget,
                        MessageDirection::ToWidget,
                        any_camera,
                    ));
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
