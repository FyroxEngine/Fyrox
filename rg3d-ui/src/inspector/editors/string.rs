use crate::core::inspect::PropertyInfo;
use crate::{
    core::pool::Handle,
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{
        MessageData, MessageDirection, PropertyChanged, TextBoxMessage, UiMessage, UiMessageData,
    },
    node::UINode,
    text_box::TextBoxBuilder,
    widget::WidgetBuilder,
    Control, Thickness,
};
use std::{any::TypeId, sync::Arc};

#[derive(Debug)]
pub struct StringPropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for StringPropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<String>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
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
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<String>()?;
        Ok(TextBoxMessage::text(
            instance,
            MessageDirection::ToWidget,
            value.clone(),
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::TextBox(TextBoxMessage::Text(value)) = message.data() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(value.clone()),
                });
            }
        }
        None
    }
}
