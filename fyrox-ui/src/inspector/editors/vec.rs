use crate::{
    core::{algebra::SVector, num_traits::NumCast},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    numeric::NumericType,
    vec::{VecEditorBuilder, VecEditorMessage},
    widget::WidgetBuilder,
    Thickness,
};
use std::{any::TypeId, marker::PhantomData};

#[derive(Debug)]
pub struct VecPropertyEditorDefinition<T: NumericType, const D: usize> {
    pub phantom: PhantomData<T>,
}

impl<T: NumericType, const D: usize> Default for VecPropertyEditorDefinition<T, D> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T: NumericType, const D: usize> PropertyEditorDefinition
    for VecPropertyEditorDefinition<T, D>
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<SVector<T, D>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<SVector<T, D>>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: VecEditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_min(SVector::repeat(
                ctx.property_info
                    .min_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::min_value),
            ))
            .with_max(SVector::repeat(
                ctx.property_info
                    .max_value
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::max_value),
            ))
            .with_step(SVector::repeat(
                ctx.property_info
                    .step
                    .and_then(NumCast::from)
                    .unwrap_or_else(T::one),
            ))
            .with_value(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<SVector<T, D>>()?;
        Ok(Some(VecEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(VecEditorMessage::Value(value)) =
                ctx.message.data::<VecEditorMessage<T, D>>()
            {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(*value),
                });
            }
        }
        None
    }
}

pub type Vec2PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 2>;
pub type Vec3PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 3>;
pub type Vec4PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 4>;
pub type Vec5PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 5>;
pub type Vec6PropertyEditorDefinition<T> = VecPropertyEditorDefinition<T, 6>;
