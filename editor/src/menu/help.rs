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
    gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode},
};
use crate::menu::{create_menu_item, create_root_menu_item};

pub struct HelpMenu {
    pub menu: Handle<UiNode>,
    pub open_book: Handle<UiNode>,
    pub open_api_reference: Handle<UiNode>,
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
