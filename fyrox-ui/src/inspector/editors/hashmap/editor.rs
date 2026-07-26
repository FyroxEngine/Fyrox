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
                HashMapKey,
            },
            PropertyEditorDefinitionContainer, PropertyEditorInstance,
        },
        InspectorEnvironmentContainer, ObjectValue,
    },
    message::{MessageData, UiMessage},
    widget::{Widget, WidgetBuilder},
    window::{WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, UiNode, UserInterface,
};
use fxhash::FxHashSet;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, PartialEq, Clone)]
pub enum HashMapPropertyEditorMessage {
    ValueChanged {
        key: ObjectValue,
        message: UiMessage,
    },
    KeyChanged {
        key: ObjectValue,
        message: UiMessage,
    },
    Remove {
        key: ObjectValue,
    },
    InsertDefault {
        key: ObjectValue,
    },
}
impl MessageData for HashMapPropertyEditorMessage {}

#[derive(Debug, Reflect, Visit, Clone, PartialEq)]
#[reflect(
    derived_type = "UiNode",
    type_uuid = "1440dacb-19ae-425b-a1f4-9d73a1009e6a"
)]
pub struct Entry<K: HashMapKey> {
    #[visit(skip)]
    pub key: K,
    pub key_hash: u64,
    pub key_editor: PropertyEditorInstance,
    pub value_editor: PropertyEditorInstance,
    pub remove: Handle<Button>,
}

#[derive(Debug, Reflect, Visit, Clone)]
#[reflect(type_uuid = "a36ed236-e6f6-4d98-a22e-73e6af38c29d", non_comparable)]
pub struct HashMapPropertyEditor<K: HashMapKey> {
    widget: Widget,
    #[visit(skip)]
    entries: Vec<Entry<K>>,
    add: Handle<Button>,
    #[visit(skip)]
    #[reflect(hidden)]
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    #[visit(skip)]
    #[reflect(hidden)]
    environment: Option<InspectorEnvironmentContainer>,
    dialog: Handle<SelectHashMapKeyDialogWindow<K>>,
}

impl<K: HashMapKey> Deref for HashMapPropertyEditor<K> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<K: HashMapKey> DerefMut for HashMapPropertyEditor<K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<K: HashMapKey> Control for HashMapPropertyEditor<K> {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        for entry in self.entries.iter() {
            if message.destination() == entry.key_editor.editor() {
                ui.post(
                    self.handle(),
                    HashMapPropertyEditorMessage::KeyChanged {
                        key: ObjectValue::new(entry.key.clone()),
                        message: message.clone(),
                    },
                )
            } else if message.destination() == entry.value_editor.editor() {
                ui.post(
                    self.handle(),
                    HashMapPropertyEditorMessage::ValueChanged {
                        key: ObjectValue::new(entry.key.clone()),
                        message: message.clone(),
                    },
                )
            } else if let Some(ButtonMessage::Click) = message.data_from(entry.remove) {
                ui.post(
                    self.handle(),
                    HashMapPropertyEditorMessage::Remove {
                        key: ObjectValue::new(entry.key.clone()),
                    },
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
            .with_existing_keys(
                self.entries
                    .iter()
                    .map(|e| e.key_hash)
                    .collect::<FxHashSet<_>>(),
            )
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
        }

        self.widget.handle_routed_message(ui, message)
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(SelectHashMapKeyDialogWindowMessage::Key(key)) =
            message.data_from::<SelectHashMapKeyDialogWindowMessage<K>>(self.dialog)
        {
            ui.post(
                self.handle(),
                HashMapPropertyEditorMessage::InsertDefault {
                    key: ObjectValue::new(key.clone()),
                },
            )
        }
    }
}

pub struct HashMapPropertyEditorBuilder<K: HashMapKey> {
    widget_builder: WidgetBuilder,
    entries: Vec<Entry<K>>,
    property_editors: Arc<PropertyEditorDefinitionContainer>,
    environment: Option<InspectorEnvironmentContainer>,
}

impl<K: HashMapKey> HashMapPropertyEditorBuilder<K> {
    pub fn new(
        widget_builder: WidgetBuilder,
        property_editors: Arc<PropertyEditorDefinitionContainer>,
    ) -> Self {
        Self {
            widget_builder,
            entries: Default::default(),
            property_editors,
            environment: None,
        }
    }

    pub fn with_entries(mut self, entries: Vec<Entry<K>>) -> Self {
        self.entries = entries;
        self
    }

    pub fn with_environment(mut self, environment: InspectorEnvironmentContainer) -> Self {
        self.environment = Some(environment);
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<HashMapPropertyEditor<K>> {
        let add = ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
            .with_text("Add...")
            .build(ctx);

        let children = self
            .entries
            .iter()
            .enumerate()
            .flat_map(|(i, e)| {
                let row = i + 1; // "add" button occupies the first row
                let key_editor = e.key_editor.editor();
                let key_editor_ref = &mut ctx[key_editor];
                key_editor_ref.set_row(row);
                key_editor_ref.set_column(0);
                let value_editor = e.value_editor.editor();
                let value_editor_ref = &mut ctx[value_editor];
                value_editor_ref.set_row(row);
                value_editor_ref.set_column(1);
                let remove_ref = &mut ctx[e.remove];
                remove_ref.set_row(row);
                remove_ref.set_column(2);
                [key_editor, value_editor, e.remove.to_base()]
            })
            .collect::<Vec<_>>();

        let grid = GridBuilder::new(WidgetBuilder::new().with_child(add).with_children(children))
            .add_row(Row::auto())
            .add_rows(self.entries.iter().map(|_| Row::auto()).collect::<Vec<_>>())
            .add_columns(vec![Column::auto(), Column::stretch(), Column::auto()])
            .build(ctx);

        ctx.add(HashMapPropertyEditor {
            widget: self
                .widget_builder
                .with_child(grid)
                .with_preview_messages(true)
                .build(ctx),
            entries: self.entries,
            add,
            property_editors: self.property_editors,
            environment: self.environment,
            dialog: Default::default(),
        })
    }
}
