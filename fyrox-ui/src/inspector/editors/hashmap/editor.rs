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

use crate::inspector::editors::hashmap::HashMapKey;
use crate::inspector::ObjectValue;
use crate::message::MessageData;
use crate::{
    core::pool::Handle,
    core::reflect::prelude::*,
    core::visitor::prelude::*,
    grid::{Column, GridBuilder, Row},
    inspector::editors::PropertyEditorInstance,
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Clone)]
pub enum HashMapPropertyEditorMessage {
    ValueChanged {
        key: ObjectValue,
        message: UiMessage,
    },
}
impl MessageData for HashMapPropertyEditorMessage {}

#[derive(Debug, Reflect, Visit, Clone, PartialEq)]
#[reflect(type_uuid = "1440dacb-19ae-425b-a1f4-9d73a1009e6a")]
pub struct Entry<K: HashMapKey> {
    pub key_hash: u64,
    #[visit(skip)]
    pub key: K,
    pub key_editor: PropertyEditorInstance,
    pub value_editor: PropertyEditorInstance,
}

#[derive(Debug, Reflect, Visit, Clone)]
#[reflect(type_uuid = "a36ed236-e6f6-4d98-a22e-73e6af38c29d", non_comparable)]
pub struct HashMapPropertyEditor<K: HashMapKey> {
    widget: Widget,
    #[visit(skip)]
    entries: Vec<Entry<K>>,
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
        if let Some(_key_editor) = self
            .entries
            .iter()
            .find(|e| message.destination() == e.key_editor.editor())
        {
            // TODO.
        } else if let Some(_value_editor) = self
            .entries
            .iter()
            .find(|e| message.destination() == e.value_editor.editor())
        {
            // TODO.
        }

        self.widget.handle_routed_message(ui, message)
    }
}

pub struct HashMapPropertyEditorBuilder<K: HashMapKey> {
    widget_builder: WidgetBuilder,
    entries: Vec<Entry<K>>,
}

impl<K: HashMapKey> HashMapPropertyEditorBuilder<K> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            entries: Default::default(),
        }
    }

    pub fn with_entries(mut self, entries: Vec<Entry<K>>) -> Self {
        self.entries = entries;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<HashMapPropertyEditor<K>> {
        let children = self
            .entries
            .iter()
            .enumerate()
            .flat_map(|(i, e)| {
                let key_editor = e.key_editor.editor();
                let key_editor_ref = &mut ctx[key_editor];
                key_editor_ref.set_row(i);
                key_editor_ref.set_column(0);
                let value_editor = e.value_editor.editor();
                let value_editor_ref = &mut ctx[value_editor];
                value_editor_ref.set_row(i);
                value_editor_ref.set_column(1);
                [key_editor, value_editor]
            })
            .collect::<Vec<_>>();

        let grid = GridBuilder::new(WidgetBuilder::new().with_children(children))
            .add_rows(self.entries.iter().map(|_| Row::auto()).collect::<Vec<_>>())
            .add_columns(vec![Column::auto(), Column::stretch()])
            .build(ctx);

        ctx.add(HashMapPropertyEditor {
            widget: self.widget_builder.with_child(grid).build(ctx),
            entries: self.entries,
        })
    }
}
