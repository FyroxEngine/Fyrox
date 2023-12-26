//! Animation selector for `Handle<Animation>` fields.

use crate::{gui::make_dropdown_list_option_universal, inspector::EditorEnvironment, Message};
use fyrox::scene::node::Node;
use fyrox::{
    animation::{machine::Machine, Animation, AnimationContainer},
    core::pool::Handle,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        inspector::{
            editors::{
                PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
                PropertyEditorMessageContext, PropertyEditorTranslationContext,
            },
            FieldKind, InspectorError, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
    },
};
use std::any::TypeId;

#[derive(Debug)]
pub struct AnimationPropertyEditorDefinition;

impl PropertyEditorDefinition for AnimationPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Handle<Animation<Handle<Node>>>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx
            .property_info
            .cast_value::<Handle<Animation<Handle<Node>>>>()?;
        if let Some(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
            Ok(PropertyEditorInstance::Simple {
                editor: DropdownListBuilder::new(WidgetBuilder::new())
                    .with_items(
                        environment
                            .available_animations
                            .iter()
                            .map(|d| {
                                make_dropdown_list_option_universal(
                                    ctx.build_context,
                                    &d.name,
                                    22.0,
                                    *value,
                                )
                            })
                            .collect(),
                    )
                    .with_opt_selected(
                        environment
                            .available_animations
                            .iter()
                            .enumerate()
                            .find_map(|(i, d)| if d.handle == *value { Some(i) } else { None }),
                    )
                    .build(ctx.build_context),
            })
        } else {
            Err(InspectorError::Custom("No environment!".to_string()))
        }
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx
            .property_info
            .cast_value::<Handle<Animation<Handle<Node>>>>()?;
        if let Some(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
            if let Some(index) = environment
                .available_animations
                .iter()
                .position(|d| d.handle == *value)
            {
                Ok(Some(DropdownListMessage::selection(
                    ctx.instance,
                    MessageDirection::ToWidget,
                    Some(index),
                )))
            } else {
                Err(InspectorError::Custom(
                    "Animation list desync has occurred!".to_string(),
                ))
            }
        } else {
            Err(InspectorError::Custom("No environment!".to_string()))
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(DropdownListMessage::SelectionChanged(Some(value))) = ctx.message.data() {
                if let Some(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
                    if let Some(definition) = environment.available_animations.get(*value) {
                        return Some(PropertyChanged {
                            name: ctx.name.to_string(),
                            owner_type_id: ctx.owner_type_id,
                            value: FieldKind::object(definition.handle),
                        });
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct AnimationContainerPropertyEditorDefinition;

impl PropertyEditorDefinition for AnimationContainerPropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<AnimationContainer<Handle<Node>>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        Ok(PropertyEditorInstance::Simple {
            editor: ButtonBuilder::new(WidgetBuilder::new())
                .with_text("Open Animation Editor...")
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        _ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        Ok(None)
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ButtonMessage::Click) = ctx.message.data() {
                if let Some(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
                    environment.sender.send(Message::OpenAnimationEditor);
                }
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct MachinePropertyEditorDefinition;

impl PropertyEditorDefinition for MachinePropertyEditorDefinition {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Machine<Handle<Node>>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        Ok(PropertyEditorInstance::Simple {
            editor: ButtonBuilder::new(WidgetBuilder::new())
                .with_text("Open ABSM Editor...")
                .build(ctx.build_context),
        })
    }

    fn create_message(
        &self,
        _ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        Ok(None)
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(ButtonMessage::Click) = ctx.message.data() {
                if let Some(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
                    environment.sender.send(Message::OpenAbsmEditor);
                }
            }
        }
        None
    }
}
