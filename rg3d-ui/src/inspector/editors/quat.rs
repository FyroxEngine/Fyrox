use crate::core::algebra::Vector3;
use crate::core::math::{quat_from_euler, RotationOrder};
use crate::{
    core::{algebra::UnitQuaternion, inspect::PropertyInfo, math::UnitQuaternionExt, pool::Handle},
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{
        MessageData, MessageDirection, PropertyChanged, UiMessage, UiMessageData, Vec3EditorMessage,
    },
    node::UINode,
    vec::vec3::Vec3EditorBuilder,
    widget::WidgetBuilder,
    Control, Thickness,
};
use std::{any::TypeId, sync::Arc};

#[derive(Debug)]
pub struct QuatPropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for QuatPropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<UnitQuaternion<f32>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<UnitQuaternion<f32>>()?;
        let euler = value.to_euler();
        Ok(Vec3EditorBuilder::new(
            WidgetBuilder::new()
                .on_row(ctx.row)
                .on_column(ctx.column)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_value(Vector3::new(
            euler.x.to_degrees(),
            euler.y.to_degrees(),
            euler.z.to_degrees(),
        ))
        .build(ctx.build_context))
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<UnitQuaternion<f32>>()?;
        let euler = value.to_euler();
        Ok(Vec3EditorMessage::value(
            instance,
            MessageDirection::ToWidget,
            euler,
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) = message.data() {
                let euler = Vector3::new(
                    value.x.to_radians(),
                    value.y.to_radians(),
                    value.z.to_radians(),
                );
                let rotation = quat_from_euler(euler, RotationOrder::XYZ);
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(rotation),
                });
            }
        }
        None
    }
}
