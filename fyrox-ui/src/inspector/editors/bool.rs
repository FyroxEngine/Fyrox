use crate::inspector::editors::PropertyEditorTranslationContext;
use crate::{
    check_box::{CheckBoxBuilder, CheckBoxMessage},
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext,
        },
        FieldKind, InspectorError, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    widget::WidgetBuilder,
    Thickness, VerticalAlignment,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct BoolPropertyEditorDefinition;

impl PropertyEditorDefinition for BoolPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<bool>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<bool>()?;
        Ok(PropertyEditorInstance::Simple {
            editor: CheckBoxBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::top_bottom(1.0))
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .checked(Some(*value))
            .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<bool>()?;
        Ok(Some(CheckBoxMessage::checked(
            ctx.instance,
            MessageDirection::ToWidget,
            Some(*value),
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(CheckBoxMessage::Check(Some(value))) = ctx.message.data::<CheckBoxMessage>()
            {
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
