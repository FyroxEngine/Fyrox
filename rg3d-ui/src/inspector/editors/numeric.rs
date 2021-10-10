use crate::numeric::{NumericType, NumericUpDownMessage};
use crate::{
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        InspectorError,
    },
    message::{FieldKind, MessageDirection, PropertyChanged, UiMessage, UiMessageData},
    numeric::NumericUpDownBuilder,
    widget::WidgetBuilder,
    Thickness,
};
use std::any::TypeId;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct NumericPropertyEditorDefinition<T>
where
    T: NumericType,
{
    phantom: PhantomData<T>,
}

impl<T> Default for NumericPropertyEditorDefinition<T>
where
    T: NumericType,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for NumericPropertyEditorDefinition<T>
where
    T: NumericType,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;
        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: NumericUpDownBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_value(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;
        Ok(Some(NumericUpDownMessage::value(
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
            if let UiMessageData::User(msg) = message.data() {
                if let Some(NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<T>>()
                {
                    return Some(PropertyChanged {
                        name: name.to_string(),
                        owner_type_id,
                        value: FieldKind::object(*value),
                    });
                }
            }
        }

        None
    }

    fn layout(&self) -> Layout {
        Layout::Horizontal
    }
}
