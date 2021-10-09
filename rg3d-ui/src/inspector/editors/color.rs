use crate::{
    color::ColorFieldBuilder,
    core::color::Color,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        InspectorError,
    },
    message::{
        ColorFieldMessage, FieldKind, MessageDirection, PropertyChanged, UiMessage, UiMessageData,
    },
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
        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: ColorFieldBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
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

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::ColorField(ColorFieldMessage::Color(value)) = message.data() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}
