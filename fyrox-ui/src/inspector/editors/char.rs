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
    core::algebra::Vector2,
    formatted_text::WrapMode,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    text::TextMessage,
    text_box::{TextBoxBuilder, TextCommitMode},
    widget::WidgetBuilder,
    Thickness, VerticalAlignment,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct CharPropertyEditorDefinition;

impl PropertyEditorDefinition for CharPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<char>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<char>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: TextBoxBuilder::new(
                WidgetBuilder::new()
                    .with_min_size(Vector2::new(0.0, 17.0))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_wrap(WrapMode::Word)
            .with_text_commit_mode(TextCommitMode::Changed)
            .with_text(value.to_string())
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<char>()?;
        Ok(Some(TextMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone().to_string(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(TextMessage::Text(value)) = ctx.message.data::<TextMessage>() {
                if let Some(first_char) = value.chars().next() {
                    return Some(PropertyChanged {
                        name: ctx.name.to_string(),
                        value: FieldKind::object(first_char),
                    });
                }
            }
        }
        None
    }
}
