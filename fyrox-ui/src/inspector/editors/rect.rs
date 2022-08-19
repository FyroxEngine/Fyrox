use crate::{
    core::math::Rect,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    rect::{RectEditorBuilder, RectEditorMessage},
    widget::WidgetBuilder,
};
use std::{any::TypeId, fmt::Debug, marker::PhantomData};

#[derive(Debug)]
pub struct RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    phantom: PhantomData<T>,
}

impl<T> RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T> Default for RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for RectPropertyEditorDefinition<T>
where
    T: NumericType,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Rect<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Rect<T>>()?;

        Ok(PropertyEditorInstance::Simple {
            editor: RectEditorBuilder::new(WidgetBuilder::new().with_height(36.0))
                .with_value(*value)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Rect<T>>()?;
        Ok(Some(RectEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(RectEditorMessage::Value(value)) =
                ctx.message.data::<RectEditorMessage<T>>()
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
