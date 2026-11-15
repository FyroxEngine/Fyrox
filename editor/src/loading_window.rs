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
    fxhash::FxHashSet,
    gui::{
        BuildContext, Thickness, UserInterface,
        formatted_text::WrapMode,
        stack_panel::StackPanelBuilder,
        text::{Text, TextBuilder, TextMessage},
        widget::WidgetBuilder,
        window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
    },
};
use std::path::PathBuf;

pub struct SceneLoadingWindow {
    window: Handle<Window>,
    scene_list_text: Handle<Text>,
}

impl SceneLoadingWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let scene_list_text = TextBuilder::new(WidgetBuilder::new()).build(ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
                        .with_wrap(WrapMode::Word)
                        .with_text(
                            "Please wait until the following scene(s) are \
                                fully loaded.",
                        )
                        .build(ctx),
                )
                .with_child(scene_list_text),
        )
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(100.0))
            .with_title(WindowTitle::text("Please wait..."))
            .can_close(false)
            .can_minimize(false)
            .open(false)
            .can_maximize(false)
            .with_remove_on_close(true)
            .with_content(content)
            .build(ctx);

        ctx.inner().send(
            window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );

        Self {
            window,
            scene_list_text,
        }
    }

    pub fn update(self, set: &FxHashSet<PathBuf>, ui: &UserInterface) -> Option<Self> {
        if set.is_empty() {
            ui.send(self.window, WindowMessage::Close);
            return None;
        }

        let list = set.iter().fold(String::new(), |mut str, path| {
            str.push_str(&path.to_string_lossy());
            str.push('\n');
            str
        });
        ui.send(self.scene_list_text, TextMessage::Text(list));

        Some(self)
    }
}
