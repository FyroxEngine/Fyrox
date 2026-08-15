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

use crate::grid::{Grid, GridMessage};
use crate::widget::WidgetMessage;
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
    BuildContext, Control, UiNode, UserInterface,
};
use fxhash::FxHashSet;
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
    Entries { hash_map: HashMap<K, Entry<V>, S> },
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
    container: Handle<Grid>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub hash_map: HashMap<K, Entry<V>, S>,
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
                WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(80.0))
                    .with_remove_on_close(true)
                    .open(false)
                    .with_title(WindowTitle::text("Enter New Key Value")),
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
        } else if let Some(HashMapPropertyEditorMessage::<K, V, S>::Entries { hash_map }) =
            message.data_for(self.handle)
        {
            self.hash_map = hash_map.clone();

            ui.send(
                self.container,
                WidgetMessage::ReplaceChildren(make_children_list(self.add, &self.hash_map)),
            );
            ui.send(self.container, GridMessage::Rows(make_rows(&self.hash_map)))
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

fn make_children_list<K, V, S>(
    add: Handle<Button>,
    hash_map: &HashMap<K, Entry<V>, S>,
) -> Vec<Handle<UiNode>>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    let mut children = vec![add.to_base()];
    children.extend(
        hash_map
            .values()
            .flat_map(|entry| {
                Some([
                    entry.key_editor.editor(),
                    entry.value_editor.editor(),
                    entry.remove.to_base(),
                ])
            })
            .flatten(),
    );
    children
}

fn make_rows<K, V, S>(hash_map: &HashMap<K, Entry<V>, S>) -> Vec<Row>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    let mut rows = hash_map.iter().map(|_| Row::auto()).collect::<Vec<_>>();
    rows.push(Row::auto());
    rows
}

pub struct HashMapPropertyEditorBuilder<K, V, S>
where
    K: HashMapKey,
    V: HashMapValue,
    S: HashMapState,
{
    widget_builder: WidgetBuilder,
    hash_map: HashMap<K, Entry<V>, S>,
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

    pub fn with_hash_map(mut self, hash_map: HashMap<K, Entry<V>, S>) -> Self {
        self.hash_map = hash_map;
        self
    }

    pub fn build(
        self,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
        environment: Option<InspectorEnvironmentContainer>,
        ctx: &mut BuildContext,
    ) -> Handle<HashMapPropertyEditor<K, V, S>> {
        let add = ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_text("Add...")
            .build(ctx);

        let children = make_children_list(add, &self.hash_map);

        let grid = GridBuilder::new(WidgetBuilder::new().with_child(add).with_children(children))
            .add_rows(make_rows(&self.hash_map))
            .add_columns(vec![Column::auto(), Column::stretch(), Column::auto()])
            .build(ctx);

        ctx.add(HashMapPropertyEditor {
            widget: self
                .widget_builder
                .with_child(grid)
                .with_preview_messages(true)
                .build(ctx),
            add,
            property_editors,
            environment,
            dialog: Default::default(),
            container: grid,
            hash_map: self.hash_map,
        })
    }
}
