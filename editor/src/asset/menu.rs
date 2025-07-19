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
    asset::{self, item::AssetItem},
    fyrox::{
        core::{log::Log, pool::Handle},
        engine::Engine,
        graph::SceneGraph,
        gui::{
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::{MessageDirection, UiMessage},
            messagebox::{
                MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult,
            },
            popup::{Placement, PopupBuilder, PopupMessage},
            stack_panel::StackPanelBuilder,
            widget::WidgetBuilder,
            widget::WidgetMessage,
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, RcUiNodeHandle, UiNode,
        },
    },
};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

pub struct AssetItemContextMenu {
    pub menu: RcUiNodeHandle,
    pub open: Handle<UiNode>,
    pub duplicate: Handle<UiNode>,
    pub copy_path: Handle<UiNode>,
    pub copy_file_name: Handle<UiNode>,
    pub show_in_explorer: Handle<UiNode>,
    pub delete: Handle<UiNode>,
    pub placement_target: Handle<UiNode>,
    pub dependencies: Handle<UiNode>,
    pub delete_confirmation_dialog: Handle<UiNode>,
    pub path_to_delete: PathBuf,
}

impl AssetItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete;
        let show_in_explorer;
        let open;
        let duplicate;
        let copy_path;
        let copy_file_name;
        let dependencies;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new()).with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            open = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Open"))
                                .build(ctx);
                            open
                        })
                        .with_child({
                            duplicate = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Duplicate"))
                                .build(ctx);
                            duplicate
                        })
                        .with_child({
                            copy_path = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Copy Full Path"))
                                .build(ctx);
                            copy_path
                        })
                        .with_child({
                            copy_file_name = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Copy File Name"))
                                .build(ctx);
                            copy_file_name
                        })
                        .with_child({
                            delete = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Delete"))
                                .build(ctx);
                            delete
                        })
                        .with_child({
                            show_in_explorer = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Show In Explorer"))
                                .build(ctx);
                            show_in_explorer
                        })
                        .with_child({
                            dependencies = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Dependencies"))
                                .build(ctx);
                            dependencies
                        }),
                )
                .build(ctx),
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            open,
            duplicate,
            copy_path,
            delete,
            show_in_explorer,
            placement_target: Default::default(),
            copy_file_name,
            dependencies,
            delete_confirmation_dialog: Default::default(),
            path_to_delete: Default::default(),
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) -> bool {
        let ui = engine.user_interfaces.first_mut();
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
                if let Some(item) = ui.try_get_of_type::<AssetItem>(self.placement_target) {
                    for handle in [self.dependencies, self.duplicate] {
                        ui.send_message(WidgetMessage::enabled(
                            handle,
                            MessageDirection::ToWidget,
                            item.path.is_file(),
                        ));
                    }
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if let Some(item) = ui.try_get_mut_of_type::<AssetItem>(self.placement_target) {
                if message.destination() == self.delete {
                    let text = format!(
                        "Do you really want to delete {} asset? This \
                    is irreversible operation!",
                        item.path.display()
                    );

                    self.path_to_delete = item.path.clone();

                    self.delete_confirmation_dialog = MessageBoxBuilder::new(
                        WindowBuilder::new(
                            WidgetBuilder::new().with_width(250.0).with_height(100.0),
                        )
                        .open(false)
                        .with_remove_on_close(true)
                        .with_title(WindowTitle::text("Confirm Deletion")),
                    )
                    .with_text(&text)
                    .with_buttons(MessageBoxButtons::YesNo)
                    .build(&mut ui.build_ctx());

                    ui.send_message(WindowMessage::open_modal(
                        self.delete_confirmation_dialog,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));

                    return true;
                } else if message.destination() == self.show_in_explorer {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        asset::show_in_explorer(canonical_path)
                    }
                } else if message.destination() == self.open {
                    item.open();
                } else if message.destination() == self.duplicate {
                    if let Some(resource) = item.untyped_resource() {
                        if let Some(path) = engine.resource_manager.resource_path(&resource) {
                            if let Some(built_in) = engine
                                .resource_manager
                                .state()
                                .built_in_resources
                                .get(&path)
                            {
                                if let Some(data_source) = built_in.data_source.as_ref() {
                                    let final_copy_path = asset::make_unique_path(
                                        Path::new("."),
                                        path.to_str().unwrap(),
                                        &data_source.extension,
                                    );

                                    match File::create(&final_copy_path) {
                                        Ok(mut file) => {
                                            Log::verify(file.write_all(&data_source.bytes));
                                        }
                                        Err(err) => Log::err(format!(
                                            "Failed to create a file for resource at path {}. \
                                                Reason: {:?}",
                                            final_copy_path.display(),
                                            err
                                        )),
                                    }
                                }
                            } else if let Ok(canonical_path) = path.canonicalize() {
                                if let (Some(parent), Some(stem), Some(ext)) = (
                                    canonical_path.parent(),
                                    canonical_path.file_stem(),
                                    canonical_path.extension(),
                                ) {
                                    let stem = stem.to_string_lossy().to_string();
                                    let ext = ext.to_string_lossy().to_string();
                                    let final_copy_path =
                                        asset::make_unique_path(parent, &stem, &ext);
                                    Log::verify(std::fs::copy(canonical_path, final_copy_path));
                                }
                            }
                        }
                    }
                } else if message.destination() == self.copy_path {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        asset::put_path_to_clipboard(engine, canonical_path.as_os_str())
                    }
                } else if message.destination() == self.copy_file_name {
                    if let Some(file_name) = item.path.clone().file_name() {
                        asset::put_path_to_clipboard(engine, file_name)
                    }
                }
            }
        } else if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.delete_confirmation_dialog {
                if *result == MessageBoxResult::Yes {
                    Log::verify(std::fs::remove_file(&self.path_to_delete));
                }

                self.delete_confirmation_dialog = Handle::NONE;
            }
        }

        false
    }
}
