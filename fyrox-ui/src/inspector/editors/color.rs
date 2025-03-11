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
    color::{
        gradient::{ColorGradientEditorBuilder, ColorGradientEditorMessage},
        ColorFieldBuilder, ColorFieldMessage,
    },
    core::{algebra::Vector2, color::Color, color_gradient::ColorGradient},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    widget::WidgetBuilder,
    Thickness,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct ColorPropertyEditorDefinition;

impl PropertyEditorDefinition for ColorPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Color>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Color>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: ColorFieldBuilder::new(
                WidgetBuilder::new()
                    .with_min_size(Vector2::new(0.0, 17.0))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_color(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Color>()?;
        Ok(Some(ColorFieldMessage::color(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ColorFieldMessage::Color(value)) = ctx.message.data::<ColorFieldMessage>() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),

                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct ColorGradientPropertyEditorDefinition;

impl PropertyEditorDefinition for ColorGradientPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<ColorGradient>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<ColorGradient>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: ColorGradientEditorBuilder::new(
                WidgetBuilder::new()
                    .with_min_size(Vector2::new(0.0, 40.0))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_color_gradient(value.clone())
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<ColorGradient>()?;
        Ok(Some(ColorGradientEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ColorGradientEditorMessage::Value(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),

                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
