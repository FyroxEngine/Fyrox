use rg3d::gui::message::{MessageDirection, TextMessage};
use rg3d::{
    gui::{
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext,
            },
            InspectorError,
        },
        message::{PropertyChanged, UiMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
        VerticalAlignment,
    },
    resource::model::Model,
};
use std::any::TypeId;

#[derive(Debug)]
pub struct OptModelResourcePropertyEditorDefinition;

fn resource_path(resource: &Option<Model>) -> String {
    resource
        .as_ref()
        .map(|m| m.state().path().to_string_lossy().to_string())
        .unwrap_or_else(|| "None".to_string())
}

impl PropertyEditorDefinition for OptModelResourcePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Option<Model>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Model>>()?;

        Ok(PropertyEditorInstance {
            title: Default::default(),
            editor: TextBuilder::new(WidgetBuilder::new())
                .with_text(resource_path(value))
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Option<Model>>()?;

        Ok(Some(TextMessage::text(
            ctx.instance,
            MessageDirection::ToWidget,
            resource_path(value),
        )))
    }

    fn translate_message(
        &self,
        _name: &str,
        _owner_type_id: TypeId,
        _message: &UiMessage,
    ) -> Option<PropertyChanged> {
        None
    }
}
