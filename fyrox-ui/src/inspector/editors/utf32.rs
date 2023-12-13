use crate::{
    core::algebra::Vector2,
    formatted_text::WrapMode,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    text::TextMessage,
    text_box::TextBoxBuilder,
    widget::WidgetBuilder,
    Thickness, VerticalAlignment,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct Utf32StringPropertyEditorDefinition;

impl PropertyEditorDefinition for Utf32StringPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Vec<char>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Vec<char>>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: TextBoxBuilder::new(
                WidgetBuilder::new()
                    .with_min_size(Vector2::new(0.0, 17.0))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_wrap(WrapMode::Word)
            .with_text(value.iter().collect::<String>())
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Vec<char>>()?;
        Ok(Some(TextMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            value.iter().collect::<String>(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(TextMessage::Text(value)) = ctx.message.data::<TextMessage>() {
                return Some(PropertyChanged {
                    owner_type_id: ctx.owner_type_id,
                    name: ctx.name.to_string(),
                    value: FieldKind::object(value.chars().collect::<Vec<_>>()),
                });
            }
        }
        None
    }
}
