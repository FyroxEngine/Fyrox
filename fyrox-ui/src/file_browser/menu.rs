use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::Vector2, log::Log, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    menu::{ContextMenu, ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{MessageDirection, OsEvent, UiMessage},
    messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
    popup::{Placement, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    text::{TextBuilder, TextMessage},
    text_box::TextBoxBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::mpsc::Sender,
};

#[derive(Clone, Visit, Reflect, Default, Debug)]
pub struct FolderNameDialog {
    pub dialog: Handle<UiNode>,
    pub folder_name_tb: Handle<UiNode>,
    pub folder_name: String,
    pub ok: Handle<UiNode>,
    pub cancel: Handle<UiNode>,
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct ItemContextMenu {
    #[component(include)]
    pub base_menu: ContextMenu,
    pub delete: Handle<UiNode>,
    pub make_folder: Handle<UiNode>,
    pub delete_message_box: Cell<Handle<UiNode>>,
    pub folder_name_dialog: RefCell<Option<FolderNameDialog>>,
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

        if let Some(PopupMessage::Placement(Placement::Cursor(_))) = message.data() {
            if message.destination() == self.handle {
                if let Some(item_path) = self.item_path(ui) {
                    ui.send_message(WidgetMessage::enabled(
                        self.make_folder,
                        MessageDirection::ToWidget,
                        item_path.is_dir(),
                    ));
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.delete {
                if let Some(item_path) = self.item_path(ui) {
                    self.delete_message_box.set(
                        MessageBoxBuilder::new(
                            WindowBuilder::new(
                                WidgetBuilder::new().with_width(250.0).with_height(100.0),
                            )
                            .with_title(WindowTitle::text("Confirm Action"))
                            .open(false),
                        )
                        .with_text(format!("Delete {} file?", item_path.display()).as_str())
                        .with_buttons(MessageBoxButtons::YesNo)
                        .build(&mut ui.build_ctx()),
                    );

                    ui.send_message(MessageBoxMessage::open(
                        self.delete_message_box.get(),
                        MessageDirection::ToWidget,
                        None,
                        None,
                    ));
                }
            } else if message.destination() == self.make_folder {
                let ctx = &mut ui.build_ctx();
                let ok;
                let cancel;
                let folder_name_tb;
                let dialog =
                    WindowBuilder::new(WidgetBuilder::new().with_width(220.0).with_height(100.0))
                        .open(false)
                        .with_title(WindowTitle::text("New Folder Name"))
                        .with_content(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_child(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .on_row(0),
                                        )
                                        .with_text("Enter a new folder name:")
                                        .build(ctx),
                                    )
                                    .with_child({
                                        folder_name_tb = TextBoxBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(2.0))
                                                .with_height(25.0)
                                                .on_row(1),
                                        )
                                        .build(ctx);
                                        folder_name_tb
                                    })
                                    .with_child(
                                        StackPanelBuilder::new(
                                            WidgetBuilder::new()
                                                .with_horizontal_alignment(
                                                    HorizontalAlignment::Right,
                                                )
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_height(23.0)
                                                .on_row(3)
                                                .with_child({
                                                    ok = ButtonBuilder::new(
                                                        WidgetBuilder::new()
                                                            .with_margin(Thickness::uniform(1.0))
                                                            .with_width(80.0),
                                                    )
                                                    .with_text("OK")
                                                    .build(ctx);
                                                    ok
                                                })
                                                .with_child({
                                                    cancel = ButtonBuilder::new(
                                                        WidgetBuilder::new()
                                                            .with_margin(Thickness::uniform(1.0))
                                                            .with_width(80.0),
                                                    )
                                                    .with_text("Cancel")
                                                    .build(ctx);
                                                    cancel
                                                }),
                                        )
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx),
                                    ),
                            )
                            .add_row(Row::auto())
                            .add_row(Row::auto())
                            .add_row(Row::stretch())
                            .add_row(Row::auto())
                            .add_column(Column::stretch())
                            .build(ctx),
                        )
                        .build(ctx);

                ui.send_message(WindowMessage::open_modal(
                    dialog,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));

                self.folder_name_dialog = RefCell::new(Some(FolderNameDialog {
                    dialog,
                    ok,
                    cancel,
                    folder_name_tb,
                    folder_name: Default::default(),
                }));
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.base_menu.preview_message(ui, message);

        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.delete_message_box.get() {
                if let MessageBoxResult::Yes = *result {
                    if let Some(item_path) = self.item_path(ui) {
                        if item_path.is_dir() {
                            Log::verify(std::fs::remove_dir_all(item_path));
                        } else {
                            Log::verify(std::fs::remove_file(item_path));
                        }
                    }
                }

                ui.send_message(WidgetMessage::remove(
                    self.delete_message_box.get(),
                    MessageDirection::ToWidget,
                ));

                self.delete_message_box.set(Handle::NONE);
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            let dialog_ref = self.folder_name_dialog.borrow();
            if let Some(dialog) = dialog_ref.as_ref() {
                if message.destination() == dialog.dialog {
                    if !dialog.folder_name.is_empty() {
                        if let Some(item_path) = self.item_path(ui) {
                            Log::verify(std::fs::create_dir_all(
                                item_path.to_path_buf().join(&dialog.folder_name),
                            ));
                        }
                    }

                    ui.send_message(WidgetMessage::remove(
                        dialog.dialog,
                        MessageDirection::ToWidget,
                    ));

                    drop(dialog_ref);

                    *self.folder_name_dialog.borrow_mut() = None;
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            let mut dialog = self.folder_name_dialog.borrow_mut();
            if let Some(dialog) = dialog.as_mut() {
                if message.destination() == dialog.ok {
                    ui.send_message(WindowMessage::close(
                        dialog.dialog,
                        MessageDirection::ToWidget,
                    ));
                } else if message.destination() == dialog.cancel {
                    dialog.folder_name.clear();

                    ui.send_message(WindowMessage::close(
                        dialog.dialog,
                        MessageDirection::ToWidget,
                    ));
                }
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            let mut dialog = self.folder_name_dialog.borrow_mut();
            if let Some(dialog) = dialog.as_mut() {
                if message.destination() == dialog.folder_name_tb {
                    dialog.folder_name.clone_from(text);
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
    pub fn build(ctx: &mut BuildContext) -> Handle<UiNode> {
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
                        .with_width(120.0)
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
            ),
        )
        .build_context_menu(ctx);

        let menu = Self {
            base_menu,
            delete,
            make_folder,
            delete_message_box: Default::default(),
            folder_name_dialog: Default::default(),
        };

        ctx.add_node(UiNode::new(menu))
    }

    fn item_path(&self, ui: &UserInterface) -> Option<PathBuf> {
        ui.try_get(self.base_menu.popup.placement.target())
            .and_then(|n| n.user_data_cloned::<PathBuf>())
    }
}
