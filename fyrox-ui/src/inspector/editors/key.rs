use crate::{
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    key::{HotKey, HotKeyEditorBuilder, HotKeyEditorMessage},
    message::{MessageDirection, UiMessage},
    widget::WidgetBuilder,
    Thickness,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct HotKeyPropertyEditorDefinition;

impl PropertyEditorDefinition for HotKeyPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<HotKey>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<HotKey>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: HotKeyEditorBuilder::new(
                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            )
            .with_value(value.clone())
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<HotKey>()?;
        Ok(Some(HotKeyEditorMessage::value(
            ctx.instance,
            MessageDirection::ToWidget,
            value.clone(),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(HotKeyEditorMessage::Value(value)) = ctx.message.data() {
                return Some(PropertyChanged {
                    name: ctx.name.to_string(),
                    owner_type_id: ctx.owner_type_id,
                    value: FieldKind::object(value.clone()),
                });
            }
        }
        None
    }
}
