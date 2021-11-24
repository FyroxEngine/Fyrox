use crate::{
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    text_box::{TextBoxBuilder, TextBoxMessage},
    widget::WidgetBuilder,
    Thickness, VerticalAlignment,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct StringPropertyEditorDefinition;

impl PropertyEditorDefinition for StringPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<String>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<String>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: TextBoxBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_text(value)
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<String>()?;
        Ok(Some(TextBoxMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let Some(TextBoxMessage::Text(value)) = message.data::<TextBoxMessage>() {
                return Some(PropertyChanged {
                    owner_type_id,
                    name: name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
