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

//! This is an editor for the tile inspector within the tileset editor.
//! The purpose of this editor is to allow the user to modify the `TileData::alternates`
//! field of each tile.

use fyrox::{
    core::{ImmutableString, pool::Handle, some_or_return},
    fxhash::FxHashMap,
    gui::{
        BuildContext, UiNode, UserInterface,
        button::Button,
        expander::ExpanderBuilder,
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        stack_panel::StackPanelBuilder,
        text::{Text, TextBuilder},
        widget::{WidgetBuilder, WidgetMessage},
    },
    scene::tilemap::{TileBook, TileDefinitionHandle},
};

use crate::plugins::tilemap::{
    TileDefinitionHandleEditor, TileDefinitionHandleEditorBuilder,
    TileDefinitionHandleEditorMessage, commands::SetAlternateCommand, tile_editor::TileEditor,
    tile_inspector::TileEditorState,
};

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<Text> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

#[derive(Clone, Copy, Debug)]
enum Value {
    One(TileDefinitionHandle),
    None,
    Multiple,
    Any,
}

impl From<Value> for Option<TileDefinitionHandle> {
    fn from(value: Value) -> Self {
        match value {
            Value::One(h) => Some(h),
            _ => None,
        }
    }
}

impl Value {
    fn is_multiple(&self) -> bool {
        matches!(self, Value::Multiple | Value::Any)
    }
    fn combined_with(self, other: Option<TileDefinitionHandle>) -> Self {
        match (self, other) {
            (Value::Any, Some(b)) => Value::One(b),
            (Value::Any, None) => Value::None,
            (Value::One(a), Some(b)) if a == b => Value::One(a),
            (Value::None, None) => Value::None,
            _ => Value::Multiple,
        }
    }
}

#[derive(Debug, Clone)]
struct AlternateRow {
    name: ImmutableString,
    handle: Handle<UiNode>,
    field: Handle<TileDefinitionHandleEditor>,
}

fn build_rows(names: &[ImmutableString], rows: &mut Vec<AlternateRow>, ctx: &mut BuildContext) {
    rows.clear();
    rows.reserve(names.len());
    for (i, name) in names.iter().enumerate() {
        let label = if name.is_empty() {
            make_label(&i.to_string(), ctx)
        } else {
            make_label(name, ctx)
        };
        let field =
            TileDefinitionHandleEditorBuilder::new(WidgetBuilder::new().on_column(1)).build(ctx);
        let handle = GridBuilder::new(WidgetBuilder::new().with_child(label).with_child(field))
            .add_column(Column::strict(100.0))
            .add_column(Column::stretch())
            .add_row(Row::auto())
            .build(ctx)
            .to_base();
        let row = AlternateRow {
            name: name.clone(),
            handle,
            field,
        };
        rows.push(row);
    }
}

#[derive(Debug, Clone)]
pub struct AlternatesEditor {
    pub handle: Handle<UiNode>,
    content: Handle<UiNode>,
    rows: Vec<AlternateRow>,
}

impl AlternatesEditor {
    pub fn new(names: &[ImmutableString], ctx: &mut BuildContext) -> Self {
        let mut rows = Vec::new();
        build_rows(names, &mut rows, ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(rows.iter().map(|r| r.handle)),
        )
        .build(ctx)
        .to_base();
        let expander = ExpanderBuilder::new(WidgetBuilder::new())
            .with_header(make_label("Alternates", ctx))
            .with_content(content)
            .build(ctx);
        Self {
            handle: expander.to_base(),
            content,
            rows,
        }
    }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.rows.iter().map(|r| r.name.as_str())
    }
    pub fn needs_rebuild(&self, state: &TileEditorState) -> bool {
        !state.alternates().map(|n| n.as_str()).eq(self.names())
    }
    pub fn rebuild(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        let names: Vec<_> = state.alternates().cloned().collect();
        build_rows(&names, &mut self.rows, &mut ui.build_ctx());
        let children = self.rows.iter().map(|r| r.handle).collect();
        ui.send_sync(self.content, WidgetMessage::ReplaceChildren(children));
    }
}

impl TileEditor for AlternatesEditor {
    fn handle(&self) -> Handle<UiNode> {
        self.handle
    }

    fn draw_button(&self) -> Handle<Button> {
        Handle::NONE
    }

    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        if self.needs_rebuild(state) {
            self.rebuild(state, ui);
        }
    }

    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        for (i, row) in self.rows.iter().enumerate() {
            let mut value = Value::Any;
            for (h, d) in state.tile_data() {
                let handle = d.alternates().get(i).filter(|x| *x != h);
                value = value.combined_with(handle);
            }
            if value.is_multiple() {
                ui.send_sync(row.field, TileDefinitionHandleEditorMessage::Multiple);
            } else {
                ui.send_sync(
                    row.field,
                    TileDefinitionHandleEditorMessage::Value(value.into()),
                );
            }
        }
        ui.send(
            self.handle,
            WidgetMessage::Visibility(state.tile_data().next().is_some() && !self.rows.is_empty()),
        );
    }

    fn draw_tile(
        &self,
        _handle: fyrox::scene::tilemap::TileDefinitionHandle,
        _subposition: fyrox::core::algebra::Vector2<usize>,
        _state: &super::TileDrawState,
        _update: &mut fyrox::scene::tilemap::TileSetUpdate,
        _tile_book: &fyrox::scene::tilemap::TileBook,
    ) {
        // Do nothing: alternates cannot be drawn
    }

    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &fyrox::gui::message::UiMessage,
        _ui: &mut UserInterface,
        tile_book: &TileBook,
        sender: &crate::message::MessageSender,
    ) {
        if let Some(TileDefinitionHandleEditorMessage::Value(value)) = message.data()
            && message.direction == MessageDirection::FromWidget
            && let TileBook::TileSet(tileset) = tile_book
            && let Some(index) = self
                .rows
                .iter()
                .position(|r| r.field.to_base() == message.destination)
        {
            let page = some_or_return!(state.page());
            let handles: Vec<_> = state
                .selected_positions()
                .filter_map(|pos| TileDefinitionHandle::try_new(page, pos))
                .collect();
            if !handles.is_empty() {
                sender.do_command(SetAlternateCommand {
                    tile_set: tileset.clone(),
                    modified_tiles: handles,
                    index,
                    value: *value,
                    old_values: FxHashMap::default(),
                });
            }
        }
    }
}
