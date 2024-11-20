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
    fyrox::{
        asset::core::pool::Handle,
        gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UiNode, UserInterface},
    },
    menu::{create_menu_item, create_root_menu_item, Panels},
    stats::StatisticsWindow,
};

pub struct UtilsMenu {
    pub menu: Handle<UiNode>,
    pub rendering_statistics: Handle<UiNode>,
}

impl UtilsMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let rendering_statistics;
        let menu = create_root_menu_item(
            "Utils",
            vec![{
                rendering_statistics = create_menu_item("Rendering Statistics", vec![], ctx);
                rendering_statistics
            }],
            ctx,
        );

        Self {
            menu,
            rendering_statistics,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        panels: &mut Panels,
        ui: &mut UserInterface,
    ) {
        if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
            if message.destination() == self.rendering_statistics {
                *panels.statistics_window = Some(StatisticsWindow::new(
                    &mut ui.build_ctx(),
                    panels.scene_frame,
                ))
            }
        }
    }
}
