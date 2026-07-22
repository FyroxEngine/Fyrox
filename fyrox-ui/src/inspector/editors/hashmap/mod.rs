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

pub mod editor;

use crate::inspector::editors::hashmap::editor::{
    Entry, HashMapPropertyEditorBuilder, HashMapPropertyEditorMessage,
};
use crate::inspector::{FieldAction, HashMapAction};
use crate::message::MessageDirection;
use crate::widget::WidgetBuilder;
use crate::{
    core::{
        reflect::{self, prelude::*, FieldMetadata, FieldRef},
        PhantomDataSendSync,
    },
    inspector::{
        editors::{
            PropertyEditorBuildContext, PropertyEditorDefinition, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        InspectorError, PropertyChanged,
    },
    message::UiMessage,
};
use std::{
    any::TypeId,
    collections::HashMap,
    fmt::{Debug, Formatter},
    hash::{BuildHasher, Hash},
};

pub trait HashMapKey: Reflect + Send + Eq + Hash + Clone + PartialEq {}
impl<T: Reflect + Send + Eq + Hash + Clone + PartialEq> HashMapKey for T {}

pub trait HashMapValue: Reflect + Clone + PartialEq {}
impl<T: Reflect + Clone + PartialEq> HashMapValue for T {}

pub trait HashMapState: BuildHasher + Clone + Debug + 'static {}
impl<T: BuildHasher + Clone + Debug + 'static> HashMapState for T {}

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

fn create_key_editor<K>(
    key: &K,
    ctx: &mut PropertyEditorBuildContext,
) -> Option<PropertyEditorInstance>
where
    K: HashMapKey,
{
    let definitions = ctx.definition_container.definitions();
    let key_property_editor = definitions.get(&TypeId::of::<K>())?;

    let property_info = ctx.property_info;

    let key_name = reflect::make_hash_map_key(key);

    let key_property_info = FieldRef {
        metadata: &FieldMetadata {
            name: &key_name,
            display_name: &key_name,
            read_only: property_info.read_only,
            immutable_collection: property_info.immutable_collection,
            min_value: property_info.min_value,
            max_value: property_info.max_value,
            step: property_info.step,
            precision: property_info.precision,
            tag: property_info.tag,
            doc: property_info.doc,
        },
        value: key,
    };

    key_property_editor
        .property_editor
        .create_instance(PropertyEditorBuildContext {
            build_context: ctx.build_context,
            property_info: &key_property_info,
            environment: ctx.environment.clone(),
            definition_container: ctx.definition_container.clone(),
            layer_index: ctx.layer_index,
            generate_property_string_values: ctx.generate_property_string_values,
            filter: ctx.filter.clone(),
            name_column_width: ctx.name_column_width,
            hide_name_column: ctx.hide_name_column,
            base_path: ctx.base_path.clone(),
            has_parent_object: ctx.has_parent_object,
        })
        .ok()
}

fn create_value_editor<V>(
    key: &V,
    ctx: &mut PropertyEditorBuildContext,
) -> Option<PropertyEditorInstance>
where
    V: HashMapValue,
{
    let definitions = ctx.definition_container.definitions();
    let value_property_editor = definitions.get(&TypeId::of::<V>())?;

    let property_info = ctx.property_info;

    let value_property_info = FieldRef {
        metadata: &FieldMetadata {
            name: property_info.name,
            display_name: property_info.display_name,
            read_only: property_info.read_only,
            immutable_collection: property_info.immutable_collection,
            min_value: property_info.min_value,
            max_value: property_info.max_value,
            step: property_info.step,
            precision: property_info.precision,
            tag: property_info.tag,
            doc: property_info.doc,
        },
        value: key,
    };

    value_property_editor
        .property_editor
        .create_instance(PropertyEditorBuildContext {
            build_context: ctx.build_context,
            property_info: &value_property_info,
            environment: ctx.environment.clone(),
            definition_container: ctx.definition_container.clone(),
            layer_index: ctx.layer_index,
            generate_property_string_values: ctx.generate_property_string_values,
            filter: ctx.filter.clone(),
            name_column_width: ctx.name_column_width,
            hide_name_column: ctx.hide_name_column,
            base_path: ctx.base_path.clone(),
            has_parent_object: ctx.has_parent_object,
        })
        .ok()
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

        let entries = hash_map
            .iter()
            .filter_map(|(key, value)| {
                Some(Entry {
                    key: key.clone(),
                    key_editor: create_key_editor(key, &mut ctx)?,
                    value_editor: create_value_editor(value, &mut ctx)?,
                })
            })
            .collect::<Vec<_>>();

        let editor = HashMapPropertyEditorBuilder::new(WidgetBuilder::new())
            .with_entries(entries)
            .build(ctx.build_context)
            .to_base();

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        _ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        // TODO
        Ok(None)
    }

    fn translate_message(&self, ctx: PropertyEditorTranslationContext) -> Option<PropertyChanged> {
        if ctx.message.direction() == MessageDirection::FromWidget {
            if let Some(msg) = ctx.message.data::<HashMapPropertyEditorMessage>() {
                match msg {
                    HashMapPropertyEditorMessage::ValueChanged { key, message } => {
                        if let Some(definition) = ctx
                            .definition_container
                            .definitions()
                            .get(&TypeId::of::<V>())
                        {
                            return Some(PropertyChanged {
                                name: ctx.name.to_string(),
                                action: FieldAction::HashMapAction(Box::new(
                                    HashMapAction::ValueChanged {
                                        key: key.clone(),
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
                        if let Some(definition) = ctx
                            .definition_container
                            .definitions()
                            .get(&TypeId::of::<K>())
                        {
                            return Some(PropertyChanged {
                                name: ctx.name.to_string(),
                                action: FieldAction::HashMapAction(Box::new(
                                    HashMapAction::KeyChanged {
                                        key: key.clone(),
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
                }
            }
        }

        None
    }
}
