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
    core::pool::Handle,
    gui::{
        formatted_text::WrapMode,
        message::MessageDirection,
        scroll_viewer::ScrollViewerBuilder,
        text::TextMessage,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, UserInterface,
    },
};

pub struct DocWindow {
    pub window: Handle<UiNode>,
    text: Handle<UiNode>,
}

impl DocWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let text;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("DocPanel")
                .with_width(400.0)
                .with_height(300.0),
        )
        .open(false)
        .with_content(
            ScrollViewerBuilder::new(WidgetBuilder::new())
                .with_content({
                    text = TextBoxBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(3.0)),
                    )
                    .with_editable(false)
                    .with_wrap(WrapMode::Word)
                    .build(ctx);
                    text
                })
                .build(ctx),
        )
        .with_title(WindowTitle::text("Documentation"))
        .build(ctx);
        Self { window, text }
    }

    pub fn open(&self, doc: String, ui: &UserInterface) {
        ui.send_message(TextMessage::text(
            self.text,
            MessageDirection::ToWidget,
            doc,
        ));
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }
}
