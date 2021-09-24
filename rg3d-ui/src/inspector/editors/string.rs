use crate::core::inspect::PropertyInfo;
use crate::{
    core::pool::Handle,
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{MessageDirection, PropertyChanged, TextBoxMessage, UiMessage, UiMessageData},
    text_box::TextBoxBuilder,
    widget::WidgetBuilder,
    Thickness, UiNode,
};
use std::{any::TypeId, sync::Arc};

#[derive(Debug)]
pub struct StringPropertyEditorDefinition;

impl PropertyEditorDefinition for StringPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<String>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<Handle<UiNode>, InspectorError> {
        let value = ctx.property_info.cast_value::<String>()?;
        Ok(TextBoxBuilder::new(
            WidgetBuilder::new()
                .on_row(ctx.row)
                .on_column(ctx.column)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text(value)
        .build(ctx.build_context))
    }

    fn create_message(
        &self,
        instance: Handle<UiNode>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError> {
        let value = property_info.cast_value::<String>()?;
        Ok(TextBoxMessage::text(
            instance,
            MessageDirection::ToWidget,
            value.clone(),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::TextBox(TextBoxMessage::Text(value)) = message.data() {
                return Some(PropertyChanged {
                    owner_type_id,
                    name: name.to_string(),
                    value: Arc::new(value.clone()),
                });
            }
        }
        None
    }
}
