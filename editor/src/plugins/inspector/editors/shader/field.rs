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
        core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            define_widget_deref_proxy,
            inspector::{
                editors::{
                    PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                    PropertyEditorMessageContext, PropertyEditorTranslationContext,
                },
                FieldAction, InspectorError, PropertyChanged,
            },
            message::{MessageDirection, UiMessage},
            style::resource::StyleResourceExt,
            widget::{Widget, WidgetBuilder},
            window::{WindowBuilder, WindowTitle},
            BuildContext, Control, UiNode, UserInterface,
        },
        material::shader::ShaderSourceCode,
    },
    plugins::inspector::editors::shader::{
        ShaderSourceCodeEditor, ShaderSourceCodeEditorBuilder, ShaderSourceCodeEditorMessage,
    },
    Editor,
};
use fyrox::gui::{
    button::Button,
    font::FontResource,
    style::{Style, StyledProperty},
};
use std::{any::TypeId, cell::RefCell};

#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "f2024683-812e-4e0d-8065-7d168a82cce6")]
#[reflect(derived_type = "UiNode")]
pub struct ShaderSourceCodeEditorField {
    widget: Widget,
    button: Handle<Button>,
    code: RefCell<ShaderSourceCode>,
    editor: Handle<ShaderSourceCodeEditor>,
}

define_widget_deref_proxy!(ShaderSourceCodeEditorField, widget);

impl Control for ShaderSourceCodeEditorField {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            self.editor = ShaderSourceCodeEditorBuilder::new(
                WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(600.0))
                    .with_title(WindowTitle::text_with_font_size(
                        "Edit Shader Source Code",
                        ui.default_font.clone(),
                        ui.style.property(Editor::UI_FONT_SIZE),
                    ))
                    .open(false),
            )
            .with_code(self.code.borrow().clone())
            .build(&mut ui.build_ctx());
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(ShaderSourceCodeEditorMessage::Code(code)) = message.data_from(self.editor) {
            *self.code.borrow_mut() = code.clone();

            ui.post(
                self.handle,
                ShaderSourceCodeEditorMessage::Code(code.clone()),
            );
        }
    }
}

pub struct ShaderSourceCodeEditorFieldBuilder {
    widget_builder: WidgetBuilder,
    code: ShaderSourceCode,
    font: Option<FontResource>,
    font_size: Option<StyledProperty<f32>>,
}

impl ShaderSourceCodeEditorFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            code: Default::default(),
            font: None,
            font_size: None,
        }
    }

    pub fn with_code(mut self, code: ShaderSourceCode) -> Self {
        self.code = code;
        self
    }

    /// Sets the desired font of the editor.
    pub fn with_font(mut self, font: FontResource) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets a desired font size property of the editor.
    pub fn with_font_size(mut self, font_size: StyledProperty<f32>) -> Self {
        self.font_size = Some(font_size);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<ShaderSourceCodeEditorField> {
        let font = self.font.clone().unwrap_or_else(|| ctx.default_font());
        let font_size = self
            .font_size
            .clone()
            .unwrap_or_else(|| ctx.style.property(Style::FONT_SIZE));

        let button = ButtonBuilder::new(WidgetBuilder::new())
            .with_text_and_font_size("Edit Source Code...", font.clone(), font_size.clone())
            .build(ctx);

        let editor = ShaderSourceCodeEditorField {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(button)
                .build(ctx),
            button,
            code: RefCell::new(self.code),
            editor: Default::default(),
        };

        ctx.add(editor)
    }
}

#[derive(Debug)]
pub struct ShaderSourceCodeEditorDefinition;

impl PropertyEditorDefinition for ShaderSourceCodeEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<ShaderSourceCode>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<ShaderSourceCode>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: ShaderSourceCodeEditorFieldBuilder::new(WidgetBuilder::new())
                .with_code(value.clone())
                .with_font(ctx.font.unwrap_or_else(|| ctx.build_context.default_font()))
                .with_font_size(
                    ctx.font_size
                        .unwrap_or_else(|| ctx.build_context.style.property(Style::FONT_SIZE)),
                )
                .build(ctx.build_context)
                .transmute(),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<ShaderSourceCode>()?;
        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            ShaderSourceCodeEditorMessage::Code(value.clone()),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ShaderSourceCodeEditorMessage::Code(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    action: FieldAction::object(value.clone()),
                });
            }
        }
        None
    }
}
