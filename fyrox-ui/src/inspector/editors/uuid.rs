use crate::{
    core::uuid::Uuid,
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    uuid::{UuidEditorBuilder, UuidEditorMessage},
    widget::WidgetBuilder,
    Thickness, VerticalAlignment,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct UuidPropertyEditorDefinition;

impl PropertyEditorDefinition for UuidPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Uuid>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Uuid>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: UuidEditorBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .with_value(*value)
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Uuid>()?;
        Ok(Some(UuidEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            *value,
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(UuidEditorMessage::Value(value)) = ctx.message.data() {
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
