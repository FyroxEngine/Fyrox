use crate::core::algebra::{RealField, SimdRealField, SimdValue};
use crate::core::num_traits::real::Real;
use crate::numeric::NumericType;
use crate::vec::vec3::Vec3EditorMessage;
use crate::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        math::{quat_from_euler, RotationOrder},
    },
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        InspectorError,
    },
    message::{FieldKind, MessageDirection, PropertyChanged, UiMessage},
    vec::vec3::Vec3EditorBuilder,
    widget::WidgetBuilder,
    Thickness,
};
use std::any::TypeId;
use std::marker::PhantomData;

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
        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: Vec3EditorBuilder::new(
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
        Ok(Some(Vec3EditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            euler_degrees,
        )))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let Some(Vec3EditorMessage::Value(value)) = message.data::<Vec3EditorMessage<T>>() {
                let euler = Vector3::new(
                    value.x.to_radians(),
                    value.y.to_radians(),
                    value.z.to_radians(),
                );
                let rotation = quat_from_euler(euler, RotationOrder::XYZ);
                return Some(PropertyChanged {
                    owner_type_id,
                    name: name.to_string(),
                    value: FieldKind::object(rotation),
                });
            }
        }

        None
    }

    fn layout(&self) -> Layout {
        Layout::Horizontal
    }
}
