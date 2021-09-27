use crate::{
    check_box::CheckBoxBuilder,
    core::{inspect::PropertyInfo, pool::Handle},
    inspector::{
        editors::{Layout, PropertyEditorBuildContext, PropertyEditorDefinition},
        InspectorError,
    },
    message::{
        CheckBoxMessage, FieldKind, MessageDirection, PropertyChanged, UiMessage, UiMessageData,
    },
    widget::WidgetBuilder,
    Thickness, UiNode,
};
use std::{any::TypeId, sync::Arc};

#[derive(Debug)]
pub struct BoolPropertyEditorDefinition;

impl PropertyEditorDefinition for BoolPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<bool>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<Handle<UiNode>, InspectorError> {
        let value = ctx.property_info.cast_value::<bool>()?;
        Ok(
            CheckBoxBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .checked(Some(*value))
                .build(ctx.build_context),
        )
    }

    fn create_message(
        &self,
        instance: Handle<UiNode>,
        property_info: &PropertyInfo,
    ) -> Result<UiMessage, InspectorError> {
        let value = property_info.cast_value::<bool>()?;
        Ok(CheckBoxMessage::checked(
            instance,
            MessageDirection::ToWidget,
            Some(*value),
        ))
    }

    fn translate_message(
        &self,
        name: &str,
        owner_type_id: TypeId,
        message: &UiMessage,
    ) -> Option<PropertyChanged> {
        if message.direction() == MessageDirection::FromWidget {
            if let UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value))) = message.data() {
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
