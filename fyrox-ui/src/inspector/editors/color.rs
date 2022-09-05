use crate::{
    color::{ColorFieldBuilder, ColorFieldMessage},
    core::{algebra::Vector2, color::Color},
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
