use crate::{
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    range::{RangeEditorBuilder, RangeEditorMessage},
    widget::WidgetBuilder,
};
use std::{any::TypeId, marker::PhantomData, ops::Range};

#[derive(Debug)]
pub struct RangePropertyEditorDefinition<T: NumericType> {
    phantom: PhantomData<T>,
}

impl<T: NumericType> RangePropertyEditorDefinition<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T: NumericType> PropertyEditorDefinition for RangePropertyEditorDefinition<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Range<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Range<T>>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: RangeEditorBuilder::new(WidgetBuilder::new())
                .with_value(value.clone())
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Range<T>>()?;

        Ok(Some(RangeEditorMessage::value(
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
            if let Some(RangeEditorMessage::Value(value)) = message.data::<RangeEditorMessage<T>>()
            {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: FieldKind::object(value.clone()),
                });
            }
        }

        None
    }
}
