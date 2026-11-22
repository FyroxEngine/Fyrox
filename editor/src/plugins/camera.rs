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
    Editor, Message,
    fyrox::{
        core::{algebra::Vector2, pool::Handle, some_or_return},
        engine::Engine,
        graph::SceneGraph,
        gui::{
            BuildContext, HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
            dock::DockingManagerMessage,
            image::{Image, ImageBuilder, ImageMessage},
            stack_panel::StackPanelBuilder,
            texture::{TextureResource, TextureResourceExtension},
            widget::{WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
        },
        scene::{bitmask::BitMask, camera::Camera},
    },
    game_scene::{GameScene, Selection},
    plugin::EditorPlugin,
};
use fyrox::gui::UserInterface;

pub struct CameraPreviewControlPanel {
    pub window: Handle<Window>,
    scene_viewer_frame: Handle<Image>,
    preview_frame: Handle<Image>,
}

impl CameraPreviewControlPanel {
    pub fn new(scene_viewer_frame: Handle<Image>, ctx: &mut BuildContext) -> Self {
        let preview_frame;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("CameraPanel")
                .with_min_size(Vector2::new(180.0, 225.0)),
        )
        .can_maximize(false)
        .can_minimize(false)
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
            scene_viewer_frame,
            preview_frame,
        }
    }

    fn enter_preview_mode(
        &mut self,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
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
                        .send_sync(self.preview_frame, ImageMessage::Texture(rt.clone().into()));
                    camera.set_render_target(rt);
                    camera
                        .render_mask
                        .set_value_and_mark_modified(BitMask(!GameScene::EDITOR_OBJECTS_MASK.0));

                    game_scene.preview_camera = node_handle;

                    engine
                        .user_interfaces
                        .first()
                        .send_sync(self.preview_frame, WidgetMessage::Visibility(true));

                    break;
                }
            }
        }
    }

    fn leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        if game_scene.preview_camera.is_some() {
            assert!(node_overrides.remove(&game_scene.preview_camera));
        }

        game_scene.preview_camera = Handle::NONE;

        let ui = engine.user_interfaces.first();

        // Don't keep the render target alive after the preview mode is off.
        ui.send_sync(self.preview_frame, ImageMessage::Texture(None.into()));
        ui.send_sync(self.preview_frame, WidgetMessage::Visibility(false));
    }

    fn destroy(self, ui: &UserInterface) {
        ui.send(self.window, WidgetMessage::Remove);
    }
}

#[derive(Default)]
pub struct CameraPlugin {
    panel: Option<CameraPreviewControlPanel>,
}

impl EditorPlugin for CameraPlugin {
    fn on_leave_preview_mode(&mut self, editor: &mut Editor) {
        let panel = some_or_return!(self.panel.as_mut());
        let entry = editor.scenes.current_scene_entry_mut();
        let game_scene = some_or_return!(entry.controller.downcast_mut::<GameScene>());
        panel.leave_preview_mode(game_scene, &mut editor.engine);
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let entry = editor.scenes.current_scene_entry_mut();
        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            if let Some(panel) = self.panel.take() {
                panel.destroy(editor.engine.user_interfaces.first());
            }
            return;
        };

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            let any_camera = entry.selection.as_graph().is_some_and(|s| {
                s.nodes
                    .iter()
                    .any(|n| scene.graph.try_get_of_type::<Camera>(*n).is_ok())
            });

            if any_camera {
                if self.panel.is_none() {
                    let ui = editor.engine.user_interfaces.first_mut();
                    let mut panel = CameraPreviewControlPanel::new(
                        editor.scene_viewer.frame(),
                        &mut ui.build_ctx(),
                    );
                    ui.send(
                        editor.docking_manager,
                        DockingManagerMessage::AddFloatingWindow(panel.window),
                    );

                    ui.send(
                        panel.window,
                        WindowMessage::Open {
                            alignment: WindowAlignment::Relative {
                                relative_to: panel.scene_viewer_frame.to_base(),
                                horizontal_alignment: HorizontalAlignment::Right,
                                vertical_alignment: VerticalAlignment::Top,
                                margin: Thickness::top_right(5.0),
                            },
                            modal: false,
                            focus_content: false,
                        },
                    );

                    panel.enter_preview_mode(&entry.selection, game_scene, &mut editor.engine);

                    self.panel = Some(panel);
                }
            } else if let Some(mut panel) = self.panel.take() {
                panel.leave_preview_mode(game_scene, &mut editor.engine);
                panel.destroy(editor.engine.user_interfaces.first())
            }
        }
    }
}
