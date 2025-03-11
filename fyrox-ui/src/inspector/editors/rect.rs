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
    core::math::Rect,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    rect::{RectEditorBuilder, RectEditorMessage},
    widget::WidgetBuilder,
};
use std::{any::TypeId, fmt::Debug, marker::PhantomData};

#[derive(Debug)]
pub struct RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    phantom: PhantomData<T>,
}

impl<T> RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Default for RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Rect<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Rect<T>>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: RectEditorBuilder::new(WidgetBuilder::new().with_height(36.0))
                .with_value(*value)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Rect<T>>()?;
        Ok(Some(RectEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(RectEditorMessage::Value(value)) =
                ctx.message.data::<RectEditorMessage<T>>()
            {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),

                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}
