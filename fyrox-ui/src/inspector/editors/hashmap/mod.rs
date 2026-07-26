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

mod dialog;
pub mod editor;

use crate::{
    core::{reflect::prelude::*, PhantomDataSendSync},
    inspector::{
        editors::{
            hashmap::editor::{HashMapPropertyEditorBuilder, HashMapPropertyEditorMessage},
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldAction, HashMapAction, InspectorError, ObjectValue, PropertyChanged,
    },
    message::{MessageDirection, UiMessage},
    widget::WidgetBuilder,
};
use std::{
    any::TypeId,
    collections::HashMap,
    fmt::{Debug, Formatter},
    hash::{BuildHasher, Hash},
};

pub trait HashMapKey: Reflect + Send + Eq + Hash + Clone + PartialEq + Default {}
impl<T: Reflect + Send + Eq + Hash + Clone + PartialEq + Default> HashMapKey for T {}

pub trait HashMapValue: Reflect + Clone + PartialEq + Default + Send {}
impl<T: Reflect + Clone + PartialEq + Default + Send> HashMapValue for T {}

pub trait HashMapState: BuildHasher + Clone + Debug + PartialEq + Default + Send + 'static {}
impl<T: BuildHasher + Clone + Debug + Send + PartialEq + Default + 'static> HashMapState for T {}

pub struct HashMapPropertyEditorDefinition<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    #[allow(unused)]
    key_placeholder: PhantomDataSendSync<K>,
    #[allow(unused)]
    value_placeholder: PhantomDataSendSync<V>,
    #[allow(unused)]
    state_placeholder: PhantomDataSendSync<S>,
}

impl<K, V, S> HashMapPropertyEditorDefinition<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    pub fn new() -> Self {
        Self {
            key_placeholder: Default::default(),
            value_placeholder: Default::default(),
            state_placeholder: Default::default(),
        }
    }
}

impl<K, V, S> Debug for HashMapPropertyEditorDefinition<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HashMapPropertyEditor")
    }
}

impl<K, V, S> PropertyEditorDefinition for HashMapPropertyEditorDefinition<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<HashMap<K, V, S>>()
    }

    fn create_instance(
        &self,
        mut ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let hash_map = ctx.property_info.cast_value::<HashMap<K, V, S>>()?;

        let editor = HashMapPropertyEditorBuilder::<K, V, S>::new(WidgetBuilder::new())
            .with_hash_map(hash_map.clone())
            .build(&mut ctx)
            .to_base();

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let hash_map = ctx.property_info.cast_value::<HashMap<K, V, S>>()?;

        // TODO: Very unoptimal solution. It is probably better to access the editor instance and
        //       see which entries it has and update/remove/add respective.
        Ok(Some(UiMessage::for_widget(
            ctx.instance,
            HashMapPropertyEditorMessage::Update {
                hash_map: hash_map.clone(),
            },
        )))
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(msg) = ctx.message.data::<HashMapPropertyEditorMessage<K, V, S>>() {
                match msg {
                    HashMapPropertyEditorMessage::ValueChanged { key, message } => {
                        if let Some(definition) = ctx.definition_container.get::<V>() {
                            return Some(PropertyChanged {
                                name: ctx.name.to_string(),
                                action: FieldAction::HashMapAction(Box::new(
                                    HashMapAction::ValueChanged {
                                        key: ObjectValue::new(key.clone()),
                                        action: definition
                                            .property_editor
                                            .translate_message(PropertyEditorTranslationContext {
                                                environment: ctx.environment.clone(),
                                                name: "",
                                                message,
                                                definition_container: ctx
                                                    .definition_container
                                                    .clone(),
                                            })?
                                            .action,
                                    },
                                )),
                            });
                        }
                    }
                    HashMapPropertyEditorMessage::KeyChanged { key, message } => {
                        if let Some(definition) = ctx.definition_container.get::<K>() {
                            return Some(PropertyChanged {
                                name: ctx.name.to_string(),
                                action: FieldAction::HashMapAction(Box::new(
                                    HashMapAction::KeyChanged {
                                        key: ObjectValue::new(key.clone()),
                                        action: definition
                                            .property_editor
                                            .translate_message(PropertyEditorTranslationContext {
                                                environment: ctx.environment.clone(),
                                                name: "",
                                                message,
                                                definition_container: ctx
                                                    .definition_container
                                                    .clone(),
                                            })?
                                            .action,
                                    },
                                )),
                            });
                        }
                    }
                    HashMapPropertyEditorMessage::Remove { key } => {
                        return Some(PropertyChanged {
                            name: ctx.name.to_string(),
                            action: FieldAction::HashMapAction(Box::new(HashMapAction::Remove {
                                key: ObjectValue::new(key.clone()),
                            })),
                        });
                    }
                    HashMapPropertyEditorMessage::InsertDefault { key } => {
                        return Some(PropertyChanged {
                            name: ctx.name.to_string(),
                            action: FieldAction::HashMapAction(Box::new(HashMapAction::Insert {
                                key: ObjectValue::new(key.clone()),
                                value: ObjectValue::new(V::default()),
                            })),
                        });
                    }
                    HashMapPropertyEditorMessage::Update { .. } => {
                        // Sync only.
                    }
                }
            }
        }

        None
    }
}
