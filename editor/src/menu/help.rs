use crate::fyrox::{
    core::{log::Log, pool::Handle},
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
};
use crate::menu::{create_menu_item, create_root_menu_item};

pub struct HelpMenu {
    pub menu: Handle<UiNode>,
    open_book: Handle<UiNode>,
    open_api_reference: Handle<UiNode>,
}

impl HelpMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let open_book;
        let open_api_reference;
        let menu = create_root_menu_item(
            "Help",
            vec![
                {
                    open_book = create_menu_item("Open Book", vec![], ctx);
                    open_book
                },
                {
                    open_api_reference = create_menu_item("Open API Reference", vec![], ctx);
                    open_api_reference
                },
            ],
            ctx,
        );

        Self {
            menu,
            open_book,
            open_api_reference,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.open_book {
                Log::verify(open::that("https://fyrox-book.github.io"));
            } else if message.destination() == self.open_api_reference {
                Log::verify(open::that("https://docs.rs/fyrox/latest"));
            }
        }
    }
}
