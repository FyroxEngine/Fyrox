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
        core::{algebra::Vector2, pool::Handle},
        engine::Engine,
        graph::SceneGraph,
        gui::{
            image::{Image, ImageBuilder, ImageMessage},
            stack_panel::StackPanelBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
        },
        resource::texture::{TextureResource, TextureResourceExtension},
        scene::{camera::Camera, collider::BitMask, node::Node},
    },
    scene::{GameScene, Selection},
    Message,
};

pub struct CameraPreviewControlPanel {
    pub window: Handle<Window>,
    camera_state: Option<(Handle<Node>, Node)>,
    scene_viewer_frame: Handle<Image>,
    preview_frame: Handle<Image>,
}

impl CameraPreviewControlPanel {
    pub fn new(scene_viewer_frame: Handle<Image>, ctx: &mut BuildContext) -> Self {
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
            scene_viewer_frame,
            preview_frame,
        }
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        mut game_scene: Option<&mut GameScene>,
        engine: &mut Engine,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let any_camera = if let Some(game_scene) = game_scene.as_mut() {
                let scene = &engine.scenes[game_scene.scene];
                editor_selection.as_graph().is_some_and(|s| {
                    s.nodes
                        .iter()
                        .any(|n| scene.graph.try_get_of_type::<Camera>(*n).is_ok())
                })
            } else {
                false
            };
            if any_camera {
                if let Some(game_scene) = game_scene.as_mut() {
                    self.enter_preview_mode(editor_selection, game_scene, engine);
                }
                engine.user_interfaces.first_mut().send(
                    self.window,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Relative {
                            relative_to: self.scene_viewer_frame.to_base(),
                            horizontal_alignment: HorizontalAlignment::Right,
                            vertical_alignment: VerticalAlignment::Top,
                            margin: Thickness::top_right(5.0),
                        },
                        modal: false,
                        focus_content: false,
                    },
                );
            } else {
                if let Some(game_scene) = game_scene.as_mut() {
                    self.leave_preview_mode(game_scene, engine);
                }
                engine
                    .user_interfaces
                    .first_mut()
                    .send(self.window, WindowMessage::Close);
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
                if let Ok(camera) = scene.graph.try_get_mut_of_type::<Camera>(node_handle) {
                    assert!(node_overrides.insert(node_handle));

                    let rt = Some(TextureResource::new_render_target(200, 200));

                    engine
                        .user_interfaces
                        .first()
                        .send_sync(self.preview_frame, ImageMessage::Texture(rt.clone()));
                    camera.set_render_target(rt);
                    camera
                        .render_mask
                        .set_value_and_mark_modified(BitMask(!GameScene::EDITOR_OBJECTS_MASK.0));

                    game_scene.preview_camera = node_handle;

                    engine
                        .user_interfaces
                        .first()
                        .send_sync(self.preview_frame, WidgetMessage::Visibility(true));

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
            if let Ok(camera) = scene.graph.try_get_node_mut(camera_handle) {
                *camera = original
            }

            assert!(node_overrides.remove(&camera_handle));
        }

        game_scene.preview_camera = Handle::NONE;

        let ui = engine.user_interfaces.first();

        // Don't keep the render target alive after the preview mode is off.
        ui.send_sync(self.preview_frame, ImageMessage::Texture(None));
        ui.send_sync(self.preview_frame, WidgetMessage::Visibility(false));
    }

    pub fn is_in_preview_mode(&self) -> bool {
        self.camera_state.is_some()
    }
}
