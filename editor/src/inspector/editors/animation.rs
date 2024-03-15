//! Animation selector for `Handle<Animation>` fields.

use crate::fyrox::generic_animation::EntityId;
use crate::fyrox::{
    core::pool::Handle,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        generic_animation::machine::Machine,
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
use crate::{gui::make_dropdown_list_option_universal, inspector::EditorEnvironment, Message};
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

pub struct AnimationPropertyEditorDefinition<T> {
    phantom: PhantomData<T>,
}

impl<T> Debug for AnimationPropertyEditorDefinition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnimationPropertyEditorDefinition")
    }
}

impl<T> Default for AnimationPropertyEditorDefinition<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for AnimationPropertyEditorDefinition<T>
where
    T: Send + Sync + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Handle<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;
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
                            .find_map(|(i, d)| {
                                if *value == d.handle.into() {
                                    Some(i)
                                } else {
                                    None
                                }
                            }),
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
        let value = ctx.property_info.cast_value::<Handle<T>>()?;
        if let Some(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
            if let Some(index) = environment
                .available_animations
                .iter()
                .position(|d| *value == d.handle.into())
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
                            value: FieldKind::object(Handle::<T>::from(definition.handle)),
                        });
                    }
                }
            }
        }
        None
    }
}

pub struct AnimationContainerPropertyEditorDefinition<T> {
    phantom: PhantomData<T>,
}

impl<T> Debug for AnimationContainerPropertyEditorDefinition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnimationContainerPropertyEditorDefinition")
    }
}

impl<T> Default for AnimationContainerPropertyEditorDefinition<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for AnimationContainerPropertyEditorDefinition<T>
where
    T: Send + Sync + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
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

pub struct MachinePropertyEditorDefinition<T> {
    phantom: PhantomData<T>,
}

impl<T> Debug for MachinePropertyEditorDefinition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MachinePropertyEditorDefinition")
    }
}

impl<T> Default for MachinePropertyEditorDefinition<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for MachinePropertyEditorDefinition<T>
where
    T: Send + Sync + EntityId,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Machine<T>>()
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
