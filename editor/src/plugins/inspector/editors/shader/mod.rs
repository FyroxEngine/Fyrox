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

use fyrox::{
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    gui::{
        control_trait_proxy_impls, define_widget_deref_proxy,
        grid::GridBuilder,
        message::{MessageData, UiMessage},
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{Window, WindowAlignment, WindowBuilder, WindowMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
    material::shader::ShaderSourceCode,
};

pub mod field;

#[derive(PartialEq, Debug, Clone)]
pub enum ShaderSourceCodeEditorMessage {
    Code(ShaderSourceCode),
}
impl MessageData for ShaderSourceCodeEditorMessage {}

#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "c2e0bdcc-28a6-4141-93b5-5dad50c8b29c")]
#[reflect(derived_type = "UiNode")]
pub struct ShaderSourceCodeEditor {
    window: Window,
    text_box: Handle<UiNode>,
}

define_widget_deref_proxy!(ShaderSourceCodeEditor, window);

impl Control for ShaderSourceCodeEditor {
    control_trait_proxy_impls!(window);

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message)
    }
}

pub struct ShaderSourceCodeEditorBuilder {
    window_builder: WindowBuilder,
    code: ShaderSourceCode,
}

impl ShaderSourceCodeEditorBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            code: Default::default(),
        }
    }

    pub fn with_code(mut self, code: ShaderSourceCode) -> Self {
        self.code = code;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<ShaderSourceCodeEditor> {
        let text_box = TextBoxBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_text(&self.code.0)
            .build(ctx);
        let content = GridBuilder::new(WidgetBuilder::new().with_child(text_box)).build(ctx);

        let editor = ShaderSourceCodeEditor {
            window: self
                .window_builder
                .with_remove_on_close(true)
                .with_content(content)
                .build_window(ctx),
            text_box,
        };

        let handle = ctx.add(editor);

        ctx.inner().send(
            handle,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: false,
                focus_content: true,
            },
        );

        handle
    }
}
