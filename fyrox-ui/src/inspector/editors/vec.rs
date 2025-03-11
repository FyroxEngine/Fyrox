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
    core::{algebra::SVector, num_traits::NumCast},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    vec::{VecEditorBuilder, VecEditorMessage},
    widget::WidgetBuilder,
    Thickness,
};
use std::{any::TypeId, marker::PhantomData};

#[derive(Debug)]
pub struct VecPropertyEditorDefinition<T: NumericType, const D: usize> {
    pub phantom: PhantomData<T>,
}

impl<T: NumericType, const D: usize> Default for VecPropertyEditorDefinition<T, D> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T: NumericType, const D: usize> PropertyEditorDefinition
    for VecPropertyEditorDefinition<T, D>
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SVector<T, D>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SVector<T, D>>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: VecEditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_min(SVector::repeat(
                ctx.property_info
                    .min_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::min_value),
            ))
            .with_max(SVector::repeat(
                ctx.property_info
                    .max_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::max_value),
            ))
            .with_step(SVector::repeat(
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
        let value = ctx.property_info.cast_value::<SVector<T, D>>()?;
        Ok(Some(VecEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(VecEditorMessage::Value(value)) =
                ctx.message.data::<VecEditorMessage<T, D>>()
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

pub type Vec2PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 2>;
pub type Vec3PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 3>;
pub type Vec4PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 4>;
pub type Vec5PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 5>;
pub type Vec6PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 6>;
