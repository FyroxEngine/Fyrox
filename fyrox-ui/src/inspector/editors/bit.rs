use crate::{
    bit::{BitContainer, BitFieldBuilder, BitFieldMessage},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    MessageDirection, Thickness, UiMessage, WidgetBuilder,
};
use std::{any::TypeId, marker::PhantomData};

#[derive(Debug)]
pub struct BitFieldPropertyEditorDefinition<T: BitContainer> {
    phantom: PhantomData<T>,
}

impl<T: BitContainer> PropertyEditorDefinition for BitFieldPropertyEditorDefinition<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: BitFieldBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_value(*value)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<T>()?;
        Ok(Some(BitFieldMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(BitFieldMessage::Value(value)) = ctx.message.data::<BitFieldMessage<T>>() {
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
