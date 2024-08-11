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
    core::{algebra::Matrix2, num_traits::NumCast},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    matrix2::{Matrix2EditorBuilder, Matrix2EditorMessage},
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    widget::WidgetBuilder,
    Thickness,
};
use std::{any::TypeId, marker::PhantomData};

#[derive(Debug)]
pub struct Matrix2PropertyEditorDefinition<T: NumericType> {
    pub phantom: PhantomData<T>,
}

impl<T: NumericType> Default for Matrix2PropertyEditorDefinition<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T: NumericType> PropertyEditorDefinition for Matrix2PropertyEditorDefinition<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Matrix2<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Matrix2<T>>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: Matrix2EditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_min(Matrix2::repeat(
                ctx.property_info
                    .min_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::min_value),
            ))
            .with_max(Matrix2::repeat(
                ctx.property_info
                    .max_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::max_value),
            ))
            .with_step(Matrix2::repeat(
                ctx.property_info
                    .step
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::one),
            ))
            .with_value(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Matrix2<T>>()?;
        Ok(Some(Matrix2EditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(Matrix2EditorMessage::Value(value)) =
                ctx.message.data::<Matrix2EditorMessage<T>>()
            {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}
