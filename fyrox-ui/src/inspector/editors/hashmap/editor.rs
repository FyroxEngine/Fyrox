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

use crate::inspector::editors::PropertyEditorBuildContext;
use crate::{
    button::{Button, ButtonBuilder, ButtonMessage},
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    grid::{Column, GridBuilder, Row},
    inspector::{
        editors::{
            hashmap::{
                dialog::{
                    SelectHashMapKeyDialogWindow, SelectHashMapKeyDialogWindowBuilder,
                    SelectHashMapKeyDialogWindowMessage,
                },
                HashMapKey, HashMapState, HashMapValue,
            },
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
        },
        InspectorEnvironmentContainer,
    },
    message::{MessageData, UiMessage},
    widget::{Widget, WidgetBuilder},
    window::{WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
    Control, UiNode, UserInterface, VerticalAlignment,
};
use fxhash::FxHashSet;
use fyrox_core::reflect;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, PartialEq, Clone)]
pub enum HashMapPropertyEditorMessage<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    Update { hash_map: HashMap<K, V, S> },
    ValueChanged { key: K, message: UiMessage },
    KeyChanged { key: K, message: UiMessage },
    Remove { key: K },
    InsertDefault { key: K },
}
impl<K, V, S> MessageData for HashMapPropertyEditorMessage<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
}

#[derive(Debug, Reflect, Visit, Clone, PartialEq)]
#[reflect(type_uuid = "1440dacb-19ae-425b-a1f4-9d73a1009e6a")]
pub struct Entry<V: HashMapValue> {
    pub key_editor: PropertyEditorInstance,
    pub value_editor: PropertyEditorInstance,
    pub remove: Handle<Button>,
    #[visit(skip)]
    pub value: V,
}

#[derive(Debug, Reflect, Visit, Clone)]
#[reflect(
    derived_type = "UiNode",
    type_uuid = "a36ed236-e6f6-4d98-a22e-73e6af38c29d",
    non_comparable,
    ignore_generics_type_uuid
)]
pub struct HashMapPropertyEditor<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    widget: Widget,
    add: Handle<Button>,
    #[visit(skip)]
    #[reflect(hidden)]
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    #[visit(skip)]
    #[reflect(hidden)]
    environment: Option<InspectorEnvironmentContainer>,
    dialog: Handle<SelectHashMapKeyDialogWindow<K>>,
    #[visit(skip)]
    #[reflect(hidden)]
    hash_map: HashMap<K, Entry<V>, S>,
}

impl<K, V, S> Deref for HashMapPropertyEditor<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<K, V, S> DerefMut for HashMapPropertyEditor<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<K, V, S> Control for HashMapPropertyEditor<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        for (key, entry) in self.hash_map.iter() {
            if message.destination() == entry.key_editor.editor() {
                ui.post(
                    self.handle(),
                    HashMapPropertyEditorMessage::<K, V, S>::KeyChanged {
                        key: key.clone(),
                        message: message.clone(),
                    },
                )
            } else if message.destination() == entry.value_editor.editor() {
                ui.post(
                    self.handle(),
                    HashMapPropertyEditorMessage::<K, V, S>::ValueChanged {
                        key: key.clone(),
                        message: message.clone(),
                    },
                )
            } else if let Some(ButtonMessage::Click) = message.data_from(entry.remove) {
                ui.post(
                    self.handle(),
                    HashMapPropertyEditorMessage::<K, V, S>::Remove { key: key.clone() },
                )
            }
        }

        if let Some(ButtonMessage::Click) = message.data_from(self.add) {
            self.dialog = SelectHashMapKeyDialogWindowBuilder::new(
                WindowBuilder::new(WidgetBuilder::new())
                    .with_remove_on_close(true)
                    .open(false)
                    .with_title(WindowTitle::text("Select Key Value")),
                self.property_editors.clone(),
                K::default(),
            )
            .with_existing_keys({
                self.hash_map
                    .keys()
                    .map(|e| self.hash_map.hasher().hash_one(e))
                    .collect::<FxHashSet<_>>()
            })
            .with_environment(self.environment.clone())
            .build(&mut ui.build_ctx());

            ui.send(
                self.dialog,
                WindowMessage::Open {
                    alignment: WindowAlignment::Center,
                    modal: true,
                    focus_content: true,
                },
            );
        } else if let Some(HashMapPropertyEditorMessage::<K, V, S>::Update { hash_map: _ }) =
            message.data_for(self.handle)
        {
            // TODO. Sync entries.
            // self.hash_map = hash_map.clone();
        }

        self.widget.handle_routed_message(ui, message)
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(SelectHashMapKeyDialogWindowMessage::Key(key)) =
            message.data_from::<SelectHashMapKeyDialogWindowMessage<K>>(self.dialog)
        {
            ui.post(
                self.handle(),
                HashMapPropertyEditorMessage::<K, V, S>::InsertDefault { key: key.clone() },
            )
        }
    }
}

pub struct HashMapPropertyEditorBuilder<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    widget_builder: WidgetBuilder,
    hash_map: HashMap<K, V, S>,
}

fn create_key_editor<K>(
    key: &K,
    ctx: &mut PropertyEditorBuildContext,
) -> Option<PropertyEditorInstance>
where
    K: HashMapKey,
{
    let key_property_editor = ctx.definition_container.get::<K>()?;

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
    let value_property_editor = ctx.definition_container.get::<V>()?;

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

impl<K, V, S> HashMapPropertyEditorBuilder<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            hash_map: Default::default(),
        }
    }

    pub fn with_hash_map(mut self, hash_map: HashMap<K, V, S>) -> Self {
        self.hash_map = hash_map;
        self
    }

    pub fn build(
        self,
        ctx: &mut PropertyEditorBuildContext,
    ) -> Handle<HashMapPropertyEditor<K, V, S>> {
        let add = ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_text("Add...")
            .build(ctx.build_context);

        let hash_map = self
            .hash_map
            .into_iter()
            .enumerate()
            .filter_map(|(i, (key, value))| {
                let key_editor_instance = create_key_editor(&key, ctx)?;
                let value_editor_instance = create_value_editor(&value, ctx)?;
                let remove = ButtonBuilder::new(
                    WidgetBuilder::new()
                        .with_width(24.0)
                        .with_height(24.0)
                        .with_vertical_alignment(VerticalAlignment::Center),
                )
                .with_text("-")
                .build(ctx.build_context);

                let row = i + 1; // "add" button occupies the first row
                let key_editor = key_editor_instance.editor();
                let key_editor_ref = &mut ctx.build_context[key_editor];
                key_editor_ref.set_row(row);
                key_editor_ref.set_column(0);
                let value_editor = value_editor_instance.editor();
                let value_editor_ref = &mut ctx.build_context[value_editor];
                value_editor_ref.set_row(row);
                value_editor_ref.set_column(1);
                let remove_ref = &mut ctx.build_context[remove];
                remove_ref.set_row(row);
                remove_ref.set_column(2);
                Some((
                    key,
                    Entry {
                        key_editor: key_editor_instance,
                        value_editor: value_editor_instance,
                        remove,
                        value,
                    },
                ))
            })
            .collect::<HashMap<K, Entry<V>, S>>();

        let children = hash_map
            .values()
            .flat_map(|entry| {
                Some([
                    entry.key_editor.editor(),
                    entry.value_editor.editor(),
                    entry.remove.to_base(),
                ])
            })
            .flatten()
            .collect::<Vec<_>>();

        let grid = GridBuilder::new(WidgetBuilder::new().with_child(add).with_children(children))
            .add_row(Row::auto())
            .add_rows(hash_map.iter().map(|_| Row::auto()).collect::<Vec<_>>())
            .add_columns(vec![Column::auto(), Column::stretch(), Column::auto()])
            .build(ctx.build_context);

        ctx.build_context.add(HashMapPropertyEditor {
            widget: self
                .widget_builder
                .with_child(grid)
                .with_preview_messages(true)
                .build(ctx.build_context),
            add,
            property_editors: ctx.definition_container.clone(),
            environment: ctx.environment.clone(),
            dialog: Default::default(),
            hash_map,
        })
    }
}
