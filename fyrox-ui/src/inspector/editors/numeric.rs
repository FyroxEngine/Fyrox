use crate::{
    core::num_traits::NumCast,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::{NumericType, NumericUpDownBuilder, NumericUpDownMessage},
    widget::WidgetBuilder,
    Thickness,
};
use std::{any::TypeId, marker::PhantomData};

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
        Ok(PropertyEditorInstance::Simple {
            editor: NumericUpDownBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::top_bottom(1.0)),
            )
            .with_min_value(
                ctx.property_info
                    .min_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::min_value),
            )
            .with_max_value(
                ctx.property_info
                    .max_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::max_value),
            )
            .with_step(
                ctx.property_info
                    .step
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::one),
            )
            .with_precision(ctx.property_info.precision.unwrap_or(3))
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

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(NumericUpDownMessage::Value(value)) =
                ctx.message.data::<NumericUpDownMessage<T>>()
            {
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
