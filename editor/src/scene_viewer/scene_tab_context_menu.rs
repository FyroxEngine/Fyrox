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

//! Context menu for scene tabs in the scene viewer.

use crate::{
    fyrox::{
        core::{pool::Handle, uuid::Uuid},
        graph::BaseSceneGraph,
        gui::{
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::UiMessage,
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::WidgetBuilder,
            BuildContext, RcUiNodeHandle, UiNode, UserInterface,
        },
    },
    message::MessageSender,
    Message,
};
use std::path::Path;

/// Context menu for scene tab headers.
pub struct SceneTabContextMenu {
    /// The context menu popup handle.
    pub menu: RcUiNodeHandle,
    /// "Show in Explorer" menu item.
    pub show_in_explorer: Handle<UiNode>,
    /// "Close" menu item.
    pub close: Handle<UiNode>,
    /// "Close All Tabs" menu item.
    pub close_all: Handle<UiNode>,
    /// The UUID of the scene that was right-clicked.
    pub target_scene_id: Option<Uuid>,
}

impl SceneTabContextMenu {
    /// Creates a new scene tab context menu.
    pub fn new(ctx: &mut BuildContext) -> Self {
        fn item(text: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
            MenuItemBuilder::new(WidgetBuilder::new())
                .with_content(MenuItemContent::text(text))
                .build(ctx)
        }

        let show_in_explorer = item("Show In Explorer", ctx);
        let close = item("Close", ctx);
        let close_all = item("Close All Tabs", ctx);

        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new())
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child(show_in_explorer)
                            .with_child(close)
                            .with_child(close_all),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            show_in_explorer,
            close,
            close_all,
            target_scene_id: None,
        }
    }

    /// Handles UI messages for the context menu.
    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        scene_ids: &[Uuid],
        scene_paths: &[Option<&Path>],
        ui: &UserInterface,
    ) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                // Get the scene ID from the user data of the target widget
                if let Some(uuid) = ui.node(*target).user_data_cloned::<Uuid>() {
                    self.target_scene_id = Some(uuid);

                    // Enable/disable "Show in Explorer" based on whether the scene has a path
                    let has_path = scene_ids
                        .iter()
                        .zip(scene_paths.iter())
                        .find(|(id, _)| **id == uuid)
                        .map(|(_, path)| path.is_some())
                        .unwrap_or(false);

                    ui.send(
                        self.show_in_explorer,
                        fyrox::gui::widget::WidgetMessage::Enabled(has_path),
                    );
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if let Some(target_id) = self.target_scene_id {
                if message.destination() == self.show_in_explorer {
                    // Find the path for this scene
                    if let Some(path) = scene_ids
                        .iter()
                        .zip(scene_paths.iter())
                        .find(|(id, _)| **id == target_id)
                        .and_then(|(_, path)| *path)
                    {
                        if let Ok(canonical_path) = path.canonicalize() {
                            show_in_explorer(canonical_path);
                        }
                    }
                } else if message.destination() == self.close {
                    sender.send(Message::CloseScene(target_id));
                } else if message.destination() == self.close_all {
                    // Close all scenes
                    for &id in scene_ids {
                        sender.send(Message::CloseScene(id));
                    }
                }
            }
        }
    }
}

/// Opens the file explorer and highlights the specified path.
fn show_in_explorer<P: AsRef<Path>>(path: P) {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        fn execute_command(command: &mut Command) {
            match command.spawn() {
                Ok(mut process) => {
                    let _ = process.wait();
                }
                Err(err) => {
                    fyrox::core::log::Log::err(format!(
                        "Failed to show in explorer. Reason: {err:?}"
                    ));
                }
            }
        }

        let path = path.as_ref();
        if path.is_dir() {
            execute_command(Command::new("explorer").arg(path));
        } else if let Some(parent) = path.parent() {
            execute_command(
                Command::new("explorer")
                    .arg("/select,")
                    .arg(path.as_os_str()),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        let path = path.as_ref();
        if path.is_dir() {
            let _ = std::process::Command::new("open").arg(path).spawn();
        } else {
            let _ = std::process::Command::new("open")
                .args(["-R", &path.to_string_lossy()])
                .spawn();
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try to use xdg-open for the parent directory
        let path = path.as_ref();
        let target = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.to_path_buf())
        };
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}
