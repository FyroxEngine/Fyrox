use crate::{
    core::{log::Log, pool::Handle},
    menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{MessageDirection, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    stack_panel::StackPanelBuilder,
    widget::{WidgetBuilder, WidgetMessage},
    BuildContext, RcUiNodeHandle, Thickness, UiNode, UserInterface,
};
use std::{cell::Cell, path::Path, path::PathBuf};

#[derive(Clone)]
pub struct ItemContextMenu {
    pub menu: RcUiNodeHandle,
    pub delete: Handle<UiNode>,
    pub make_folder: Handle<UiNode>,
    pub placement_target: Cell<Handle<UiNode>>,
    pub delete_message_box: Handle<UiNode>,
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete;
        let make_folder;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
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
            )
            .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            delete,
            make_folder,
            placement_target: Default::default(),
            delete_message_box: Default::default(),
        }
    }

    fn item_path<'a>(&self, ui: &'a UserInterface) -> Option<&'a Path> {
        ui.try_get_node(self.placement_target.get())
            .and_then(|n| n.user_data_ref::<PathBuf>())
            .map(|p| p.as_path())
    }

    pub fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == *self.menu {
                self.placement_target.set(*target);

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
                    if item_path.is_dir() {
                        Log::verify(std::fs::remove_dir_all(item_path));
                    } else {
                        Log::verify(std::fs::remove_file(item_path));
                    }
                }
            } else if message.destination() == self.make_folder {
                // TODO
            }
        }
    }
}
