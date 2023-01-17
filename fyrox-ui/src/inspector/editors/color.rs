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
                    owner_type_id: ctx.owner_type_id,
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
                    owner_type_id: ctx.owner_type_id,
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
