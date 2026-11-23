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

//! This is the tab of the tile set editor that allows the user to modify the alternate
//! tiles stored within the tile set. Alternates can be created, deleted, renamed
//! and re-ordered.

use fyrox::{
    core::ImmutableString,
    fxhash::FxHashMap,
    gui::{
        HorizontalAlignment, VerticalAlignment,
        button::ButtonMessage,
        grid::*,
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        utils::{ArrowDirection, make_arrow},
    },
    scene::tilemap::tileset::{OptionTileSet, TileSetRef},
};

use super::*;
use commands::*;
use fyrox::core::pool::ObjectOrVariant;
use fyrox::gui::text_box::TextBox;

/// This is the tab of the tile set editor that allows the user to modify the
/// alternate tiles index names. Alternates can be created, deleted, renamed
/// and re-ordered.
pub struct AlternatesTab {
    handle: Handle<Grid>,
    list: Handle<ListView>,
    up_button: Handle<Button>,
    down_button: Handle<Button>,
    remove_button: Handle<Button>,
    add_button: Handle<Button>,
    name_field: Handle<TextBox>,
    data_panel: Handle<Grid>,
}

fn make_arrow_button(
    ctx: &mut BuildContext,
    dir: ArrowDirection,
    column: usize,
    row: usize,
) -> Handle<Button> {
    let arrow = make_arrow(ctx, dir, 16.0);
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(column)
            .on_row(row)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(arrow)
    .build(ctx)
}

fn make_button(
    title: &str,
    tooltip: &str,
    ctx: &mut BuildContext,
    column: usize,
    row: usize,
) -> Handle<Button> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(column)
            .on_row(row)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

fn make_list_item(ctx: &mut BuildContext, index: usize, name: &ImmutableString) -> Handle<UiNode> {
    let number = TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Right)
        .with_text(index.to_string())
        .build(ctx);
    let label = TextBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .with_margin(Thickness::left(10.0)),
    )
    .with_vertical_text_alignment(VerticalAlignment::Center)
    .with_horizontal_text_alignment(HorizontalAlignment::Left)
    .with_text(name.as_str())
    .build(ctx);
    let content = GridBuilder::new(WidgetBuilder::new().with_child(number).with_child(label))
        .add_column(Column::strict(50.0))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(content))
            .with_corner_radius((4.0).into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
    .to_base()
}

fn make_items(ctx: &mut BuildContext, tile_set: &OptionTileSet) -> Vec<Handle<UiNode>> {
    tile_set
        .alternates()
        .iter()
        .enumerate()
        .map(|(index, alt)| make_list_item(ctx, index, alt))
        .collect()
}

fn send_enabled(
    ui: &UserInterface,
    destination: Handle<impl ObjectOrVariant<UiNode>>,
    enabled: bool,
) {
    ui.send(destination, WidgetMessage::Enabled(enabled));
}

impl AlternatesTab {
    pub fn new(tile_book: TileBook, ctx: &mut BuildContext) -> Self {
        let items = if let TileBook::TileSet(t) = &tile_book {
            make_items(ctx, &TileSetRef::new(t).as_loaded())
        } else {
            Vec::default()
        };
        let scroll = ScrollViewerBuilder::new(WidgetBuilder::new()).build(ctx);
        let list = ListViewBuilder::new(WidgetBuilder::new().on_row(1))
            .with_items(items)
            .with_scroll_viewer(scroll)
            .build(ctx);
        let up_button = make_arrow_button(ctx, ArrowDirection::Top, 0, 0);
        let down_button = make_arrow_button(ctx, ArrowDirection::Bottom, 1, 0);
        let add_button = make_button("New", "Create an alternate", ctx, 2, 0);
        let buttons = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(up_button)
                .with_child(down_button)
                .with_child(add_button),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);
        let left_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Alternates:")
        .build(ctx);
        let left_side = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(left_label)
                .with_child(list)
                .with_child(buttons),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);
        let name_text = TextBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_margin(Thickness::right(4.0)),
        )
        .with_text("Name:")
        .build(ctx);
        let name_field = TextBoxBuilder::new(WidgetBuilder::new().with_height(20.0).on_column(1))
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_text_commit_mode(TextCommitMode::Changed)
            .build(ctx);
        let remove_button = make_button(
            "Delete",
            "Delete this alternate from every tile in the tile set",
            ctx,
            2,
            0,
        );
        let header = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(name_text)
                .with_child(name_field)
                .with_child(remove_button),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::strict(100.0))
        .add_row(Row::auto())
        .build(ctx);
        let data_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_column(1)
                .with_child(header),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);
        Self {
            handle: GridBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(2.0))
                    .with_child(left_side)
                    .with_child(data_panel),
            )
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .build(ctx),
            list,
            up_button,
            down_button,
            add_button,
            remove_button,
            name_field,
            data_panel,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle.to_base()
    }
    pub fn sync_to_model(&mut self, tile_set: &OptionTileSet, ui: &mut UserInterface) {
        let items = make_items(&mut ui.build_ctx(), tile_set);
        ui.send(self.list, ListViewMessage::Items(items));
        self.sync_data(tile_set, ui);
    }
    pub fn sync_data(&mut self, tile_set: &OptionTileSet, ui: &mut UserInterface) {
        let sel_index = self.selection_index(ui);
        let name = match sel_index {
            Some(index) => tile_set
                .alternates()
                .get(index)
                .map(|c| c.to_string())
                .unwrap_or_default(),
            None => String::default(),
        };
        send_enabled(ui, self.data_panel, sel_index.is_some());
        ui.send_sync(self.name_field, TextMessage::Text(name));
    }
    pub fn handle_ui_message(
        &mut self,
        tile_set: TileSetResource,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if message.direction() == MessageDirection::ToWidget {
            return;
        }
        if let Some(ListViewMessage::Selection(_)) = message.data() {
            if message.destination() == self.list {
                self.sync_data(&TileSetRef::new(&tile_set).as_loaded(), ui);
            }
        } else if let Some(TextMessage::Text(value)) = message.data() {
            if message.destination() == self.name_field {
                self.update_name(tile_set, value, ui, sender);
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.up_button {
                self.move_layer(tile_set, -1, ui, sender);
            } else if message.destination() == self.down_button {
                self.move_layer(tile_set, 1, ui, sender);
            } else if message.destination() == self.add_button {
                self.add_layer(tile_set, ui, sender);
            } else if message.destination() == self.remove_button {
                self.remove_layer(tile_set, ui, sender);
            }
        }
    }
    fn selection_index(&self, ui: &UserInterface) -> Option<usize> {
        ui.try_get(self.list).ok()?.selection.last().copied()
    }
    fn update_name(
        &self,
        resource: TileSetResource,
        name: &str,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let tile_set = resource.data_ref();
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.alternate_names.len() - 1))
        else {
            return;
        };
        sender.do_command(RenameAlternateCommand {
            tile_set: resource.clone(),
            index: sel_index,
            name: name.into(),
        });
    }
    fn move_layer(
        &self,
        resource: TileSetResource,
        amount: isize,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let tile_set = resource.data_ref();
        let Some(sel_index) = self.selection_index(ui) else {
            return;
        };
        let new_index = sel_index
            .saturating_add_signed(amount)
            .clamp(0, tile_set.alternate_names.len() - 1);
        if sel_index == new_index {
            return;
        }
        ui.send(self.list, ListViewMessage::Selection(vec![new_index]));
        sender.do_command(MoveAlternateCommand {
            tile_set: resource.clone(),
            start: sel_index,
            end: new_index,
        });
    }
    fn add_layer(&self, resource: TileSetResource, ui: &UserInterface, sender: &MessageSender) {
        let tile_set = resource.data_ref();
        let index = self
            .selection_index(ui)
            .map(|i| i + 1)
            .unwrap_or(0)
            .clamp(0, tile_set.alternate_names.len());
        ui.send(self.list, ListViewMessage::Selection(vec![index]));
        ui.send(self.name_field, WidgetMessage::Focus);
        sender.do_command(AddAlternateCommand {
            tile_set: resource.clone(),
            index,
        });
    }
    fn remove_layer(&self, resource: TileSetResource, ui: &UserInterface, sender: &MessageSender) {
        let tile_set = resource.data_ref();
        let Some(index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.properties.len() - 1))
        else {
            return;
        };
        sender.do_command(RemoveAlternateCommand {
            tile_set: resource.clone(),
            index,
            name: ImmutableString::default(),
            values: FxHashMap::default(),
        });
    }
}
