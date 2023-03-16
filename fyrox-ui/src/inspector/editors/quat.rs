use crate::{
    core::{
        algebra::{RealField, SimdRealField, SimdValue, UnitQuaternion, Vector3},
        math::{quat_from_euler, RotationOrder},
        num_traits::real::Real,
    },
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
pub struct QuatPropertyEditorDefinition<T>
where
    T: NumericType + Real + SimdValue + SimdRealField + RealField,
    T::Element: SimdRealField,
{
    phantom: PhantomData<T>,
}

impl<T> Default for QuatPropertyEditorDefinition<T>
where
    T: NumericType + Real + SimdValue + SimdRealField + RealField,
    T::Element: SimdRealField,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for QuatPropertyEditorDefinition<T>
where
    T: NumericType + Real + SimdValue + SimdRealField + RealField,
    T::Element: SimdRealField,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<UnitQuaternion<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<UnitQuaternion<T>>()?;
        let euler = value.euler_angles();
        Ok(PropertyEditorInstance::Simple {
            editor: VecEditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_value(Vector3::new(
                euler.0.to_degrees(),
                euler.1.to_degrees(),
                euler.2.to_degrees(),
            ))
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<UnitQuaternion<T>>()?;
        let euler = value.euler_angles();
        let euler_degrees = Vector3::new(
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees(),
        );
        Ok(Some(VecEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            euler_degrees,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(VecEditorMessage::Value(value)) =
                ctx.message.data::<VecEditorMessage<T, 3>>()
            {
                let euler = Vector3::new(
                    value[0].to_radians(),
                    value[1].to_radians(),
                    value[2].to_radians(),
                );
                let rotation = quat_from_euler(euler, RotationOrder::XYZ);
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(rotation),
                });
            }
        }

        None
    }
}
