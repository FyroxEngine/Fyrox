// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
use crate::plugins::inspector::EditorEnvironment;
use crate::Message;

use fyrox::core::reflect::Reflect;
use fyrox::gui::utils::make_dropdown_list_option_universal;
use std::{
    any::TypeId,
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

pub struct AnimationPropertyEditorDefinition<T: Reflect> {
    phantom: PhantomData<T>,
}

impl<T: Reflect> Debug for AnimationPropertyEditorDefinition<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnimationPropertyEditorDefinition")
    }
}

impl<T: Reflect> Default for AnimationPropertyEditorDefinition<T> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<T> PropertyEditorDefinition for AnimationPropertyEditorDefinition<T>
where
    T: Reflect + Send + Sync + 'static,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<Handle<T>>()
    }

    fn create_instance(
        &self,
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;
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
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let value = ctx.property_info.cast_value::<Handle<T>>()?;
        let environment = EditorEnvironment::try_get_from(&ctx.environment)?;
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
            Ok(None)
        }
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(DropdownListMessage::SelectionChanged(Some(value))) = ctx.message.data() {
                if let Ok(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
                    if let Some(definition) = environment.available_animations.get(*value) {
                        return Some(PropertyChanged {
                            name: ctx.name.to_string(),

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
                if let Ok(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
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
                if let Ok(environment) = EditorEnvironment::try_get_from(&ctx.environment) {
                    environment.sender.send(Message::OpenAbsmEditor);
                }
            }
        }
        None
    }
}
