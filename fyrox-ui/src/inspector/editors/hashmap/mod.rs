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
    button::ButtonBuilder,
    core::{reflect::prelude::*, PhantomDataSendSync},
    inspector::{
        editors::{
            hashmap::editor::{
                Entry, HashMapPropertyEditor, HashMapPropertyEditorBuilder,
                HashMapPropertyEditorMessage,
            },
            PropertyEditorBuildContext, PropertyEditorDefinition,
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
            PropertyEditorMessageContext, PropertyEditorTranslationContext,
        },
        FieldAction, HashMapAction, InspectorEnvironmentContainer, InspectorError, ObjectValue,
        PropertyChanged, PropertyFilter,
    },
    message::{DeliveryMode, MessageDirection, UiMessage},
    widget::WidgetBuilder,
    BuildContext, VerticalAlignment,
};
use fyrox_core::reflect;
use fyrox_graph::SceneGraph;
use std::sync::Arc;
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

fn create_key_editor<'a, 'b, K>(
    key: &K,
    environment: Option<InspectorEnvironmentContainer>,
    definition_container: Arc<PropertyEditorDefinitionContainer>,
    property_info: &FieldRef<'a, 'b>,
    ctx: &mut BuildContext,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
    name_column_width: f32,
    hide_name_column: bool,
    base_path: String,
    has_parent_object: bool,
) -> Option<PropertyEditorInstance>
where
    K: HashMapKey,
{
    let key_property_editor = definition_container.get::<K>()?;

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
            build_context: ctx,
            property_info: &key_property_info,
            environment,
            definition_container: definition_container.clone(),
            layer_index,
            generate_property_string_values,
            filter,
            name_column_width,
            hide_name_column,
            base_path,
            has_parent_object,
        })
        .ok()
}

fn create_value_editor<'a, 'b, V>(
    key: &V,
    environment: Option<InspectorEnvironmentContainer>,
    definition_container: Arc<PropertyEditorDefinitionContainer>,
    property_info: &FieldRef<'a, 'b>,
    ctx: &mut BuildContext,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
    name_column_width: f32,
    hide_name_column: bool,
    base_path: String,
    has_parent_object: bool,
) -> Option<PropertyEditorInstance>
where
    V: HashMapValue,
{
    let value_property_editor = definition_container.get::<V>()?;

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
            build_context: ctx,
            property_info: &value_property_info,
            environment,
            definition_container: definition_container.clone(),
            layer_index,
            generate_property_string_values,
            filter,
            name_column_width,
            hide_name_column,
            base_path,
            has_parent_object,
        })
        .ok()
}

fn make_entries<'a, 'b, K, V, S>(
    hash_map: &HashMap<K, V, S>,
    environment: Option<InspectorEnvironmentContainer>,
    definition_container: Arc<PropertyEditorDefinitionContainer>,
    property_info: &FieldRef<'a, 'b>,
    ctx: &mut BuildContext,
    layer_index: usize,
    generate_property_string_values: bool,
    filter: PropertyFilter,
    name_column_width: f32,
    hide_name_column: bool,
    base_path: String,
    has_parent_object: bool,
) -> HashMap<K, Entry<V>, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    hash_map
        .iter()
        .enumerate()
        .filter_map(|(i, (key, value))| {
            let key_editor_instance = create_key_editor(
                key,
                environment.clone(),
                definition_container.clone(),
                property_info,
                ctx,
                layer_index,
                generate_property_string_values,
                filter.clone(),
                name_column_width,
                hide_name_column,
                base_path.clone(),
                has_parent_object,
            )?;
            let value_editor_instance = create_value_editor(
                value,
                environment.clone(),
                definition_container.clone(),
                property_info,
                ctx,
                layer_index,
                generate_property_string_values,
                filter.clone(),
                name_column_width,
                hide_name_column,
                base_path.clone(),
                has_parent_object,
            )?;
            let remove = ButtonBuilder::new(
                WidgetBuilder::new()
                    .with_width(24.0)
                    .with_height(24.0)
                    .with_vertical_alignment(VerticalAlignment::Center),
            )
            .with_text("-")
            .build(ctx);

            let row = i + 1; // "add" button occupies the first row
            let key_editor = key_editor_instance.editor();
            let key_editor_ref = &mut ctx[key_editor];
            key_editor_ref.set_row(row);
            key_editor_ref.set_column(0);
            let value_editor = value_editor_instance.editor();
            let value_editor_ref = &mut ctx[value_editor];
            value_editor_ref.set_row(row);
            value_editor_ref.set_column(1);
            let remove_ref = &mut ctx[remove];
            remove_ref.set_row(row);
            remove_ref.set_column(2);
            Some((
                key.clone(),
                Entry {
                    key_editor: key_editor_instance,
                    value_editor: value_editor_instance,
                    remove,
                    value: value.clone(),
                },
            ))
        })
        .collect::<HashMap<K, Entry<V>, S>>()
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
        ctx: PropertyEditorBuildContext,
    ) -> Result<PropertyEditorInstance, InspectorError> {
        let hash_map = ctx.property_info.cast_value::<HashMap<K, V, S>>()?;

        let editor = HashMapPropertyEditorBuilder::<K, V, S>::new(WidgetBuilder::new())
            .with_hash_map(make_entries(
                hash_map,
                ctx.environment.clone(),
                ctx.definition_container.clone(),
                ctx.property_info,
                ctx.build_context,
                ctx.layer_index + 1,
                ctx.generate_property_string_values,
                ctx.filter,
                ctx.name_column_width,
                ctx.hide_name_column,
                ctx.base_path,
                ctx.has_parent_object,
            ))
            .build(ctx.definition_container, ctx.environment, ctx.build_context)
            .to_base();

        Ok(PropertyEditorInstance::Simple { editor })
    }

    fn create_message(
        &self,
        ctx: PropertyEditorMessageContext,
    ) -> Result<Option<UiMessage>, InspectorError> {
        let PropertyEditorMessageContext {
            instance,
            ui,
            property_info,
            definition_container,
            layer_index,
            environment,
            generate_property_string_values,
            filter,
            name_column_width,
            hide_name_column,
            base_path,
            has_parent_object,
        } = ctx;

        let instance_ref =
            if let Some(instance) = ui.node(instance).cast::<HashMapPropertyEditor<K, V, S>>() {
                instance
            } else {
                return Err(InspectorError::Custom(
                    "Property editor is not HashMapPropertyEditor!".to_string(),
                ));
            };

        let hash_map = property_info.cast_value::<HashMap<K, V, S>>()?;

        if hash_map.len() != instance_ref.hash_map.len() {
            // Re-create items.
            let items = make_entries(
                hash_map,
                environment,
                definition_container,
                property_info,
                &mut ui.build_ctx(),
                layer_index + 1,
                generate_property_string_values,
                filter,
                name_column_width,
                hide_name_column,
                base_path,
                has_parent_object,
            );

            Ok(Some(UiMessage::for_widget(
                instance,
                HashMapPropertyEditorMessage::Entries { hash_map: items },
            )))
        } else {
            for ((_, entry), (key, value)) in
                instance_ref.hash_map.clone().iter().zip(hash_map.iter())
            {
                if let Some(key_property_editor_definition) = definition_container.get::<K>() {
                    let key_property_info = FieldRef {
                        metadata: &FieldMetadata {
                            name: "",
                            display_name: "",
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

                    if let Some(message) = key_property_editor_definition
                        .property_editor
                        .create_message(PropertyEditorMessageContext {
                            property_info: &key_property_info,
                            environment: environment.clone(),
                            definition_container: definition_container.clone(),
                            instance: entry.key_editor.editor(),
                            layer_index: layer_index + 1,
                            ui,
                            generate_property_string_values,
                            filter: filter.clone(),
                            name_column_width,
                            hide_name_column,
                            base_path: base_path.clone(),
                            has_parent_object,
                        })?
                    {
                        ui.send_message(message.with_delivery_mode(DeliveryMode::SyncOnly))
                    }
                }
                if let Some(value_property_editor_definition) = definition_container.get::<V>() {
                    let value_property_info = FieldRef {
                        metadata: &FieldMetadata {
                            name: "",
                            display_name: "",
                            read_only: property_info.read_only,
                            immutable_collection: property_info.immutable_collection,
                            min_value: property_info.min_value,
                            max_value: property_info.max_value,
                            step: property_info.step,
                            precision: property_info.precision,
                            tag: property_info.tag,
                            doc: property_info.doc,
                        },
                        value,
                    };

                    if let Some(message) = value_property_editor_definition
                        .property_editor
                        .create_message(PropertyEditorMessageContext {
                            property_info: &value_property_info,
                            environment: environment.clone(),
                            definition_container: definition_container.clone(),
                            instance: entry.value_editor.editor(),
                            layer_index: layer_index + 1,
                            ui,
                            generate_property_string_values,
                            filter: filter.clone(),
                            name_column_width,
                            hide_name_column,
                            base_path: base_path.clone(),
                            has_parent_object,
                        })?
                    {
                        ui.send_message(message.with_delivery_mode(DeliveryMode::SyncOnly))
                    }
                }
            }

            Ok(None)
        }
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
                    HashMapPropertyEditorMessage::Entries { .. } => {
                        // Sync only.
                    }
                }
            }
        }

        None
    }
}
