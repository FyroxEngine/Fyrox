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
        gui::{menu::MenuItemMessage, message::UiMessage, BuildContext, UserInterface},
    },
    menu::{create_menu_item, create_root_menu_item, Panels},
    stats::StatisticsWindow,
};
use fyrox::core::{uuid, Uuid};
use fyrox::gui::menu::MenuItem;

pub struct UtilsMenu {
    pub menu: Handle<MenuItem>,
    pub rendering_statistics: Handle<MenuItem>,
}

impl UtilsMenu {
    pub const UTILS: Uuid = uuid!("f6a9a297-6efc-4b62-83b6-3955c0c43a00");
    pub const RENDERING_STATISTICS: Uuid = uuid!("ecf0bdb9-f97f-4df0-b17f-7ec07bdebd4d");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let rendering_statistics;
        let menu = create_root_menu_item(
            "Utils",
            Self::UTILS,
            vec![{
                rendering_statistics = create_menu_item(
                    "Rendering Statistics",
                    Self::RENDERING_STATISTICS,
                    vec![],
                    ctx,
                );
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
