use crate::{
    check_box::CheckBoxBuilder,
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{
        CheckBoxMessage, MessageData, MessageDirection, PropertyChanged, UiMessage, UiMessageData,
    },
    node::UINode,
    widget::WidgetBuilder,
    Control, Thickness,
};
use std::{any::TypeId, sync::Arc};

#[derive(Debug)]
pub struct BoolPropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for BoolPropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<bool>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<bool>()?;
        Ok(CheckBoxBuilder::new(
            WidgetBuilder::new()
                .on_row(ctx.row)
                .on_column(ctx.column)
                .with_margin(Thickness::uniform(1.0)),
        )
        .checked(Some(*value))
        .build(ctx.build_context))
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<bool>()?;
        Ok(CheckBoxMessage::checked(
            instance,
            MessageDirection::ToWidget,
            Some(*value),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage<M, C>,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value))) = message.data() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: Arc::new(*value),
                });
            }
        }
        None
    }
}
