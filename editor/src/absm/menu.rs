use crate::absm::message::AbsmMessage;
use fyrox::{
    core::pool::Handle,
    gui::{
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::UiMessage,
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
};
use std::sync::mpsc::Sender;

pub struct Menu {
    pub menu: Handle<UiNode>,
    pub edit_menu: EditMenu,
}

impl Menu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let edit_menu = EditMenu::new(ctx);

        let menu = MenuBuilder::new(WidgetBuilder::new())
            .with_items(vec![edit_menu.menu])
            .build(ctx);

        Self { menu, edit_menu }
    }

    pub fn handle_ui_message(&self, sender: &Sender<AbsmMessage>, message: &UiMessage) {
        self.edit_menu.handle_ui_message(sender, message)
    }
}

pub struct EditMenu {
    pub menu: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    clear_command_stack: Handle<UiNode>,
}

impl EditMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let undo;
        let redo;
        let clear_command_stack;
        let menu = MenuItemBuilder::new(WidgetBuilder::new())
            .with_content(MenuItemContent::text_no_arrow("Edit"))
            .with_items(vec![
                {
                    undo = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Undo"))
                        .build(ctx);
                    undo
                },
                {
                    redo = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Redo"))
                        .build(ctx);
                    redo
                },
                {
                    clear_command_stack = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Clear Command Stack"))
                        .build(ctx);
                    clear_command_stack
                },
            ])
            .build(ctx);

        Self {
            menu,
            undo,
            redo,
            clear_command_stack,
        }
    }

    pub fn handle_ui_message(&self, sender: &Sender<AbsmMessage>, message: &UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.undo {
                sender.send(AbsmMessage::Undo).unwrap();
            } else if message.destination() == self.redo {
                sender.send(AbsmMessage::Redo).unwrap();
            } else if message.destination() == self.clear_command_stack {
                sender.send(AbsmMessage::ClearCommandStack).unwrap();
            }
        }
    }
}
