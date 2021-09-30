use crate::{
    inspector::{
        editors::{
            Layout, PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        InspectorError,
    },
    message::{
        FieldKind, MessageDirection, PropertyChanged, TextBoxMessage, UiMessage, UiMessageData,
    },
    text_box::TextBoxBuilder,
    widget::WidgetBuilder,
    Thickness, VerticalAlignment,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct StringPropertyEditorDefinition;

impl PropertyEditorDefinition for StringPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<String>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<String>()?;
        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: TextBoxBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_text(value)
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<UiMessage, InspectorError> {
        let value = ctx.property_info.cast_value::<String>()?;
        Ok(TextBoxMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::TextBox(TextBoxMessage::Text(value)) = message.data() {
                return Some(PropertyChanged {
                    owner_type_id,
                    name: name.to_string(),
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }

    fn layout(&self) -> Layout {
        Layout::Horizontal
    }
}
