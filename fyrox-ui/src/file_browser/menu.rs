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

use crate::menu::MenuItem;
use crate::messagebox::MessageBox;
use crate::{
    core::{
        algebra::Vector2, log::Log, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*,
    },
    draw::DrawingContext,
    file_browser::{
        dialog::{FolderNameDialog, FolderNameDialogMessage},
        fs_tree::TreeItemPath,
    },
    menu::{ContextMenu, ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{OsEvent, UiMessage},
    messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
    popup::{Placement, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{WindowBuilder, WindowTitle},
    BuildContext, Control, Thickness, UiNode, UserInterface,
};
use fyrox_graph::SceneGraph;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct ItemContextMenu {
    #[component(include)]
    pub base_menu: ContextMenu,
    pub delete: Handle<MenuItem>,
    pub make_folder: Handle<MenuItem>,
    pub delete_message_box: Cell<Handle<MessageBox>>,
    pub folder_name_dialog: Handle<FolderNameDialog>,
}

impl Deref for ItemContextMenu {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.base_menu.popup.widget
    }
}

impl DerefMut for ItemContextMenu {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_menu.popup.widget
    }
}

uuid_provider!(ItemContextMenu = "6a9d597f-6a9f-4bad-b569-4cff1a6deff7");

impl Control for ItemContextMenu {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.base_menu.on_remove(sender)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.base_menu.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.base_menu.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.base_menu.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.base_menu.update(dt, ui)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.base_menu.handle_routed_message(ui, message);

        if let Some(PopupMessage::Placement(Placement::Cursor(_))) = message.data_from(self.handle)
        {
            if let Some(tree_item_path) = self.item_path(ui) {
                ui.send(
                    self.make_folder,
                    WidgetMessage::Enabled(tree_item_path.path().is_dir()),
                );
                ui.send(
                    self.delete,
                    WidgetMessage::Enabled(!tree_item_path.is_root()),
                );
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.delete {
                if let Some(tree_item_path) = self.item_path(ui) {
                    self.delete_message_box.set(
                        MessageBoxBuilder::new(
                            WindowBuilder::new(
                                WidgetBuilder::new().with_width(250.0).with_height(100.0),
                            )
                            .with_title(WindowTitle::text("Confirm Action"))
                            .open(false),
                        )
                        .with_text(
                            format!("Delete {} file?", tree_item_path.path().display()).as_str(),
                        )
                        .with_buttons(MessageBoxButtons::YesNo)
                        .build(&mut ui.build_ctx()),
                    );

                    ui.send(
                        self.delete_message_box.get(),
                        MessageBoxMessage::Open {
                            title: None,
                            text: None,
                        },
                    );
                }
            } else if message.destination() == self.make_folder {
                self.folder_name_dialog = FolderNameDialog::build_and_open(ui);
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.base_menu.preview_message(ui, message);

        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.delete_message_box.get() {
                if let MessageBoxResult::Yes = *result {
                    if let Some(item_path) = self.item_path(ui).map(|p| p.into_path()) {
                        if item_path.is_dir() {
                            Log::verify(std::fs::remove_dir_all(item_path));
                        } else {
                            Log::verify(std::fs::remove_file(item_path));
                        }
                    }
                }

                ui.send(self.delete_message_box.get(), WidgetMessage::Remove);

                self.delete_message_box.set(Handle::NONE);
            }
        } else if let Some(FolderNameDialogMessage::Name(folder_name)) =
            message.data_from(self.folder_name_dialog)
        {
            if !folder_name.is_empty() {
                if let Some(item_path) = self.item_path(ui).map(|p| p.into_path()) {
                    Log::verify(std::fs::create_dir_all(
                        item_path.to_path_buf().join(folder_name),
                    ));
                }
            }
        }
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.base_menu.handle_os_event(self_handle, ui, event)
    }
}

impl ItemContextMenu {
    pub fn build(ctx: &mut BuildContext) -> Handle<ItemContextMenu> {
        let delete;
        let make_folder;
        let base_menu = ContextMenuBuilder::new(
            PopupBuilder::new(
                WidgetBuilder::new()
                    .with_preview_messages(true)
                    .with_visibility(false),
            )
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            delete = MenuItemBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                            )
                            .with_content(MenuItemContent::text("Delete"))
                            .build(ctx);
                            delete
                        })
                        .with_child({
                            make_folder = MenuItemBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                            )
                            .with_content(MenuItemContent::text("Make Folder"))
                            .build(ctx);
                            make_folder
                        }),
                )
                .build(ctx),
            )
            .with_restrict_picking(true),
        )
        .build_context_menu(ctx);

        let menu = Self {
            base_menu,
            delete,
            make_folder,
            delete_message_box: Default::default(),
            folder_name_dialog: Default::default(),
        };

        ctx.add(menu)
    }

    fn item_path(&self, ui: &UserInterface) -> Option<TreeItemPath> {
        ui.try_get_node(self.base_menu.popup.placement.target())
            .ok()
            .and_then(|n| n.user_data_cloned::<TreeItemPath>())
    }
}
