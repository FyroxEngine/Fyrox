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

use crate::{
    fyrox::{
        core::pool::Handle,
        engine::Engine,
        graph::SceneGraph,
        gui::{
            check_box::{CheckBoxBuilder, CheckBoxMessage},
            image::{ImageBuilder, ImageMessage},
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            widget::WidgetBuilder,
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, VerticalAlignment,
        },
        resource::texture::{TextureResource, TextureResourceExtension},
        scene::{camera::Camera, node::Node},
    },
    scene::{GameScene, Selection},
    send_sync_message, send_sync_messages, Message,
};
use fyrox::core::algebra::Vector2;
use fyrox::gui::widget::WidgetMessage;
use fyrox::scene::collider::BitMask;

pub struct CameraPreviewControlPanel {
    pub window: Handle<UiNode>,
    preview: Handle<UiNode>,
    camera_state: Option<(Handle<Node>, Node)>,
    scene_viewer_frame: Handle<UiNode>,
    preview_frame: Handle<UiNode>,
}

impl CameraPreviewControlPanel {
    pub fn new(scene_viewer_frame: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let preview;
        let preview_frame;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("CameraPanel")
                .with_min_size(Vector2::new(180.0, 45.0)),
        )
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
                    })
                    .with_child({
                        preview_frame = ImageBuilder::new(
                            WidgetBuilder::new().with_width(200.0).with_height(200.0),
                        )
                        .with_flip(true)
                        .build(ctx);
                        preview_frame
                    }),
            )
            .with_orientation(Orientation::Vertical)
            .build(ctx),
        )
        .open(false)
        .build(ctx);

        Self {
            window,
            camera_state: Default::default(),
            preview,
            scene_viewer_frame,
            preview_frame,
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

            let any_camera = editor_selection.as_graph().is_some_and(|s| {
                s.nodes
                    .iter()
                    .any(|n| scene.graph.try_get_of_type::<Camera>(*n).is_some())
            });
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

    fn enter_preview_mode(
        &mut self,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        assert!(self.camera_state.is_none());

        let scene = &mut engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        if let Some(new_graph_selection) = editor_selection.as_graph() {
            // Enable the first camera from the new selection.
            for &node_handle in &new_graph_selection.nodes {
                if let Some(camera) = scene.graph.try_get_mut_of_type::<Camera>(node_handle) {
                    assert!(node_overrides.insert(node_handle));

                    let rt = Some(TextureResource::new_render_target(200, 200));
                    send_sync_message(
                        engine.user_interfaces.first(),
                        ImageMessage::texture(
                            self.preview_frame,
                            MessageDirection::ToWidget,
                            rt.clone(),
                        ),
                    );
                    camera.set_render_target(rt);
                    camera
                        .render_mask
                        .set_value_and_mark_modified(BitMask(!GameScene::EDITOR_OBJECTS_MASK.0));

                    game_scene.preview_camera = node_handle;

                    send_sync_message(
                        engine.user_interfaces.first(),
                        WidgetMessage::visibility(
                            self.preview_frame,
                            MessageDirection::ToWidget,
                            true,
                        ),
                    );

                    self.camera_state = Some((node_handle, scene.graph[node_handle].clone_box()));
                    break;
                }
            }
        }
    }

    pub fn leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        let scene = &mut engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        if let Some((camera_handle, original)) = self.camera_state.take() {
            if let Some(camera) = scene.graph.try_get_node_mut(camera_handle) {
                *camera = original
            }

            assert!(node_overrides.remove(&camera_handle));
        }

        game_scene.preview_camera = Handle::NONE;

        let ui = engine.user_interfaces.first();
        send_sync_messages(
            ui,
            [
                // Don't keep the render target alive after the preview mode is off.
                ImageMessage::texture(self.preview_frame, MessageDirection::ToWidget, None),
                CheckBoxMessage::checked(self.preview, MessageDirection::ToWidget, Some(false)),
                WidgetMessage::visibility(self.preview_frame, MessageDirection::ToWidget, false),
            ],
        );
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.camera_state.is_some()
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
