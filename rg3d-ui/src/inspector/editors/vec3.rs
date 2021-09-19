use crate::{
    core::{algebra::Vector3, pool::Handle},
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        property::PropertyInfo,
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
pub struct Vec3PropertyEditorDefinition;

impl<M: MessageData, C: Control<M, C>> PropertyEditorDefinition<M, C>
    for Vec3PropertyEditorDefinition
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Vector3<f32>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext<M, C>,
    ) -> Result<Handle<UINode<M, C>>, InspectorError> {
        let value = ctx.property_info.cast_value::<Vector3<f32>>()?;
        Ok(Vec3EditorBuilder::new(
            WidgetBuilder::new()
                .on_row(ctx.row)
                .on_column(ctx.column)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_value(*value)
        .build(ctx.build_context))
    }

    fn create_message(
        &self,
        instance: Handle<UINode<M, C>>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage<M, C>, InspectorError> {
        let value = property_info.cast_value::<Vector3<f32>>()?;
        Ok(Vec3EditorMessage::value(
            instance,
            MessageDirection::ToWidget,
            *value,
        ))
    }

    fn translate_message(&self, name: &str, message: &UiMessage<M, C>) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) = message.data() {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    value: Arc::new(*value),
                });
            }
        }
        None
    }
}
