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

use crate::fyrox::{
    core::{log::Log, pool::Handle},
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext},
};
use crate::menu::{create_menu_item, create_root_menu_item};
use fyrox::core::{uuid, Uuid};
use fyrox::gui::menu::MenuItem;

pub struct HelpMenu {
    pub menu: Handle<MenuItem>,
    pub open_book: Handle<MenuItem>,
    pub open_api_reference: Handle<MenuItem>,
}

impl HelpMenu {
    pub const HELP: Uuid = uuid!("ac39d24b-16e0-4150-b33b-a990d0c8b7bb");
    pub const OPEN_BOOK: Uuid = uuid!("ad6133b9-a3b3-4fd4-9fbf-491d220ffe0a");
    pub const OPEN_API: Uuid = uuid!("974533d3-7c20-460d-91a6-31aa91d35511");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let open_book;
        let open_api_reference;
        let menu = create_root_menu_item(
            "Help",
            Self::HELP,
            vec![
                {
                    open_book = create_menu_item("Open Book", Self::OPEN_BOOK, vec![], ctx);
                    open_book
                },
                {
                    open_api_reference =
                        create_menu_item("Open API Reference", Self::OPEN_API, vec![], ctx);
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
