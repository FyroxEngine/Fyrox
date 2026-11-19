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
    Editor, Message,
    fyrox::{
        core::pool::Handle,
        gui::{
            BuildContext, UserInterface,
            dock::{DockingManager, DockingManagerMessage},
            markdown,
            message::UiMessage,
            scroll_viewer::{ScrollViewer, ScrollViewerBuilder, ScrollViewerMessage},
            widget::WidgetBuilder,
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
        },
    },
    plugin::EditorPlugin,
};

pub struct ShowDocumentation(pub String);

struct DocWindow {
    pub window: Handle<Window>,
    scroll_viewer: Handle<ScrollViewer>,
}

impl DocWindow {
    fn new(ctx: &mut BuildContext) -> Self {
        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new()).build(ctx);
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("DocPanel")
                .with_width(400.0)
                .with_height(300.0),
        )
        .with_remove_on_close(true)
        .open(false)
        .with_content(scroll_viewer)
        .with_title(WindowTitle::text("Documentation"))
        .build(ctx);
        Self {
            window,
            scroll_viewer,
        }
    }

    fn open(&self, doc: String, ui: &mut UserInterface) {
        let content = markdown::markdown_to_visual_tree(ui, doc);
        ui.send(self.scroll_viewer, ScrollViewerMessage::Content(content));
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: false,
                focus_content: true,
            },
        );
    }

    fn handle_ui_message(
        self,
        message: &UiMessage,
        ui: &UserInterface,
        docking_manager: Handle<DockingManager>,
    ) -> Option<Self> {
        if let Some(WindowMessage::Close) = message.data()
            && message.destination() == self.window
        {
            ui.send(
                docking_manager,
                DockingManagerMessage::RemoveFloatingWindow(self.window),
            );
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Default)]
pub struct DocPlugin {
    window: Option<DocWindow>,
}

impl EditorPlugin for DocPlugin {
    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(window) = self.window.take() {
            let ui = editor.engine.user_interfaces.first();
            self.window = window.handle_ui_message(message, ui, editor.docking_manager);
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        if let Some(data) = message.try_cast_any::<ShowDocumentation>() {
            let ui = editor.engine.user_interfaces.first_mut();
            let window = DocWindow::new(&mut ui.build_ctx());
            ui.send(
                editor.docking_manager,
                DockingManagerMessage::AddFloatingWindow(window.window),
            );
            window.open(data.0.clone(), ui);
            self.window = Some(window);
        }
    }
}
