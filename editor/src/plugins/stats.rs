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
        core::pool::Handle,
        gui::{
            menu::MenuItemMessage,
            message::{MessageDirection, UiMessage},
            stack_panel::StackPanelBuilder,
            text::{TextBuilder, TextMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage},
            HorizontalAlignment, Thickness, UiNode, VerticalAlignment,
        },
    },
    menu::create_menu_item,
    plugin::EditorPlugin,
    Editor,
};
use fyrox::gui::window::WindowTitle;

/// Editor UI statistics, useful to track number of active widgets and memory consumption.

#[derive(Default)]
pub struct UiStatisticsPlugin {
    window: Handle<UiNode>,
    text: Handle<UiNode>,
    open_ui_stats: Handle<UiNode>,
}

impl EditorPlugin for UiStatisticsPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        let ctx = &mut ui.build_ctx();
        self.open_ui_stats = create_menu_item("Editor UI Statistics", vec![], ctx);
        ui.send_message(MenuItemMessage::add_item(
            editor.menu.utils_menu.menu,
            MessageDirection::ToWidget,
            self.open_ui_stats,
        ));
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.open_ui_stats && self.window.is_none() {
                let ctx = &mut ui.build_ctx();
                self.text =
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                        .build(ctx);
                self.window =
                    WindowBuilder::new(WidgetBuilder::new().with_width(200.0).with_height(100.0))
                        .with_title(WindowTitle::text("Editor UI Statistics"))
                        .with_content(
                            StackPanelBuilder::new(WidgetBuilder::new().with_child(self.text))
                                .build(ctx),
                        )
                        .open(false)
                        .build(ctx);

                ui.send_message(WindowMessage::open_and_align(
                    self.window,
                    MessageDirection::ToWidget,
                    editor.scene_viewer.frame(),
                    HorizontalAlignment::Right,
                    VerticalAlignment::Bottom,
                    Thickness::uniform(1.0),
                    false,
                    true,
                ));
            }
        }

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                ui.send_message(WidgetMessage::remove(
                    self.window,
                    MessageDirection::ToWidget,
                ));
                self.window = Handle::NONE;
            }
        }
    }

    fn on_update(&mut self, editor: &mut Editor) {
        if self.window.is_none() {
            return;
        }

        let ui = editor.engine.user_interfaces.first();

        let total_memory = ui
            .nodes()
            .iter()
            .fold(0, |acc, node| acc + node.self_size());

        ui.send_message(TextMessage::text(
            self.text,
            MessageDirection::ToWidget,
            format!(
                "Widget Count: {}\nMemory Used: {:.3} Mb.\n\
                Drawing Commands: {}\nProcessed Messages: {}\n",
                ui.nodes().alive_count(),
                total_memory as f32 / (1024.0 * 1024.0),
                ui.get_drawing_context().get_commands().len(),
                editor.processed_ui_messages
            ),
        ));
    }
}
