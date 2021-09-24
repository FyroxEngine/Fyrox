use crate::core::inspect::PropertyInfo;
use crate::{
    core::pool::Handle,
    inspector::{
        editors::{PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{MessageDirection, NumericUpDownMessage, PropertyChanged, UiMessage, UiMessageData},
    numeric::NumericUpDownBuilder,
    widget::WidgetBuilder,
    Thickness, UiNode,
};
use std::{any::TypeId, sync::Arc};

#[derive(Debug)]
pub struct I32PropertyEditorDefinition;

impl PropertyEditorDefinition for I32PropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<i32>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<Handle<UiNode>, InspectorError> {
        let value = ctx.property_info.cast_value::<i32>()?;
        Ok(NumericUpDownBuilder::new(
            WidgetBuilder::new()
                .on_row(ctx.row)
                .on_column(ctx.column)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_precision(0)
        .with_step(1.0)
        .with_min_value(-i32::MAX as f32)
        .with_max_value(i32::MAX as f32)
        .with_value(*value as f32)
        .build(ctx.build_context))
    }

    fn create_message(
        &self,
        instance: Handle<UiNode>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError> {
        let value = property_info.cast_value::<i32>()?;
        Ok(NumericUpDownMessage::value(
            instance,
            MessageDirection::ToWidget,
            *value as f32,
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = message.data()
            {
                return Some(PropertyChanged {
                    name: name.to_string(),
                    owner_type_id,
                    value: Arc::new(*value as i32),
                });
            }
        }
        None
    }
}
