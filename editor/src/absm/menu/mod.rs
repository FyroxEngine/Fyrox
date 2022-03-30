use crate::absm::message::MessageSender;
use fyrox::{
    core::pool::Handle,
    gui::{
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::UiMessage,
        widget::WidgetBuilder,
        BuildContext, UiNode,
    },
};

pub mod context;

pub struct Menu {
    pub menu: Handle<UiNode>,
    pub file_menu: FileMenu,
    pub edit_menu: EditMenu,
}

impl Menu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let file_menu = FileMenu::new(ctx);
        let edit_menu = EditMenu::new(ctx);

        let menu = MenuBuilder::new(WidgetBuilder::new())
            .with_items(vec![file_menu.menu, edit_menu.menu])
            .build(ctx);

        Self {
            menu,
            edit_menu,
            file_menu,
        }
    }

    pub fn handle_ui_message(&self, sender: &MessageSender, message: &UiMessage) {
        self.file_menu.handle_ui_message(sender, message);
        self.edit_menu.handle_ui_message(sender, message);
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

    pub fn handle_ui_message(&self, sender: &MessageSender, message: &UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.undo {
                sender.undo();
            } else if message.destination() == self.redo {
                sender.redo();
            } else if message.destination() == self.clear_command_stack {
                sender.clear_command_stack();
            }
        }
    }
}

pub struct FileMenu {
    menu: Handle<UiNode>,
    new: Handle<UiNode>,
    save: Handle<UiNode>,
    load: Handle<UiNode>,
}

impl FileMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let new;
        let save;
        let load;
        let menu = MenuItemBuilder::new(WidgetBuilder::new())
            .with_content(MenuItemContent::text_no_arrow("File"))
            .with_items(vec![
                {
                    new = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("New"))
                        .build(ctx);
                    new
                },
                {
                    save = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Save"))
                        .build(ctx);
                    save
                },
                {
                    load = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Load"))
                        .build(ctx);
                    load
                },
            ])
            .build(ctx);

        Self {
            menu,
            new,
            save,
            load,
        }
    }

    pub fn handle_ui_message(&self, sender: &MessageSender, message: &UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.new {
                sender.create_new_absm();
            } else if message.destination() == self.save {
                sender.save_current_absm();
            } else if message.destination() == self.load {
                sender.load_absm();
            }
        }
    }
}
