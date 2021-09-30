use crate::{
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        InspectorError,
    },
    message::{
        FieldKind, MessageDirection, NumericUpDownMessage, PropertyChanged, UiMessage,
        UiMessageData,
    },
    numeric::NumericUpDownBuilder,
    widget::WidgetBuilder,
    Thickness,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct F32PropertyEditorDefinition;

impl PropertyEditorDefinition for F32PropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<f32>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<f32>()?;
        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: NumericUpDownBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_value(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<UiMessage, InspectorError> {
        let value = ctx.property_info.cast_value::<f32>()?;
        Ok(NumericUpDownMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
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
                    value: FieldKind::object(*value),
                });
            }
        }

        None
    }

    fn layout(&self) -> Layout {
        Layout::Horizontal
    }
}
