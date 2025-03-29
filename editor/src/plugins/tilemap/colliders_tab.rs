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

//! This is the tab of the tile set editor that allows the user to modify the collider
//! layers stored within the tile set. Layers can be created, deleted, renamed
//! and their colors can be modified.

use fyrox::{
    fxhash::FxHashMap,
    gui::{
        button::ButtonMessage,
        color::{ColorFieldBuilder, ColorFieldMessage},
        grid::*,
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        scroll_viewer::ScrollViewerBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        utils::{make_arrow, ArrowDirection},
        HorizontalAlignment, VerticalAlignment,
    },
};

use fyrox::scene::tilemap::{tileset::*, *};

use crate::{send_sync_message, MSG_SYNC_FLAG};

use super::*;
use commands::*;

/// This is the tab of the tile set editor that allows the user to modify the collider
/// layers stored within the tile set. Layers can be created, deleted, renamed
/// and their colors can be modified.
pub struct CollidersTab {
    handle: Handle<UiNode>,
    list: Handle<UiNode>,
    up_button: Handle<UiNode>,
    down_button: Handle<UiNode>,
    remove_button: Handle<UiNode>,
    add_button: Handle<UiNode>,
    data_panel: Handle<UiNode>,
    name_field: Handle<UiNode>,
    color_field: Handle<UiNode>,
}

fn make_arrow_button(
    ctx: &mut BuildContext,
    dir: ArrowDirection,
    column: usize,
    row: usize,
) -> Handle<UiNode> {
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
) -> Handle<UiNode> {
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

pub fn make_list_item(ctx: &mut BuildContext, collider: &TileSetColliderLayer) -> Handle<UiNode> {
    let content = GridBuilder::new(
        WidgetBuilder::new()
            .with_child(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .with_horizontal_alignment(HorizontalAlignment::Center)
                        .with_vertical_alignment(VerticalAlignment::Center)
                        .with_width(16.0)
                        .with_height(16.0)
                        .with_background(Brush::Solid(collider.color).into()),
                )
                .build(ctx),
            )
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::left(5.0))
                        .on_column(1),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Left)
                .with_text(collider.name.clone())
                .build(ctx),
            ),
    )
    .add_row(Row::auto())
    .add_column(Column::strict(20.0))
    .add_column(Column::stretch())
    .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(content))
            .with_corner_radius(4.0.into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

fn make_items(ctx: &mut BuildContext, tile_set: &OptionTileSet) -> Vec<Handle<UiNode>> {
    tile_set
        .colliders()
        .iter()
        .map(|c| make_list_item(ctx, c))
        .collect()
}

impl CollidersTab {
    pub fn new(tile_book: TileBook, ctx: &mut BuildContext) -> Self {
        let items = if let TileBook::TileSet(t) = &tile_book {
            make_items(ctx, &TileSetRef::new(t).as_loaded())
        } else {
            Vec::default()
        };
        let properties_scroll = ScrollViewerBuilder::new(WidgetBuilder::new()).build(ctx);
        let list = ListViewBuilder::new(WidgetBuilder::new().on_row(1))
            .with_items(items)
            .with_scroll_viewer(properties_scroll)
            .build(ctx);
        let up_button = make_arrow_button(ctx, ArrowDirection::Top, 0, 0);
        let down_button = make_arrow_button(ctx, ArrowDirection::Bottom, 1, 0);
        let add_button = make_button("New", "Create a new collider.", ctx, 2, 0);
        let up_down = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
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
        .with_text("Colliders:")
        .build(ctx);
        let left_side = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(left_label)
                .with_child(list)
                .with_child(up_down),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::strict(30.0))
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
            .with_text_commit_mode(TextCommitMode::Changed)
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .build(ctx);
        let remove_button = make_button(
            "Delete",
            "Delete this collider from every tile in the tile set.",
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
        let color_field = ColorFieldBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_row(1)
                .with_height(30.0),
        )
        .build(ctx);
        let data_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_column(1)
                .with_child(header)
                .with_child(color_field),
        )
        .add_row(Row::auto())
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
            data_panel,
            name_field,
            color_field,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    pub fn sync_to_model(&mut self, tile_set: &OptionTileSet, ui: &mut UserInterface) {
        let items = make_items(&mut ui.build_ctx(), tile_set);
        ui.send_message(ListViewMessage::items(
            self.list,
            MessageDirection::ToWidget,
            items,
        ));
        self.sync_data(tile_set, ui);
    }
    fn sync_data(&mut self, tile_set: &OptionTileSet, ui: &mut UserInterface) {
        let sel_index = self.selection_index(ui);
        let name = match sel_index {
            Some(index) => tile_set
                .colliders()
                .get(index)
                .map(|c| c.name.to_string())
                .unwrap_or_default(),
            None => String::default(),
        };
        let color = match sel_index {
            Some(index) => tile_set
                .colliders()
                .get(index)
                .map(|c| c.color)
                .unwrap_or(Color::BLACK),
            None => Color::BLACK,
        };
        ui.send_message(WidgetMessage::enabled(
            self.data_panel,
            MessageDirection::ToWidget,
            sel_index.is_some(),
        ));
        send_sync_message(
            ui,
            TextMessage::text(self.name_field, MessageDirection::ToWidget, name),
        );
        send_sync_message(
            ui,
            ColorFieldMessage::color(self.color_field, MessageDirection::ToWidget, color),
        );
    }
    pub fn handle_ui_message(
        &mut self,
        tile_set: TileSetResource,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if message.direction() == MessageDirection::ToWidget || message.flags == MSG_SYNC_FLAG {
            return;
        }
        if let Some(ListViewMessage::SelectionChanged(_)) = message.data() {
            if message.destination() == self.list {
                self.sync_data(&TileSetRef::new(&tile_set).as_loaded(), ui);
            }
        } else if let Some(TextMessage::Text(value)) = message.data() {
            if message.destination() == self.name_field {
                self.update_name(tile_set, value, ui, sender);
            }
        } else if let Some(&ColorFieldMessage::Color(value)) = message.data() {
            if message.destination() == self.color_field {
                self.update_color(tile_set, value, ui, sender);
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
        ui.node(self.list)
            .cast::<ListView>()?
            .selection
            .last()
            .copied()
    }
    fn update_name(
        &self,
        resource: TileSetResource,
        name: &str,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let mut tile_set = TileSetRef::new(&resource);
        let tile_set = tile_set.as_loaded();
        let colliders = tile_set.colliders();
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, colliders.len() - 1))
        else {
            return;
        };
        let Some(uuid) = colliders.get(sel_index).map(|l| l.uuid) else {
            return;
        };
        ui.send_message(WidgetMessage::focus(
            self.name_field,
            MessageDirection::ToWidget,
        ));
        sender.do_command(SetColliderLayerNameCommand {
            tile_set: resource.clone(),
            uuid,
            name: name.into(),
        });
    }
    fn update_color(
        &self,
        resource: TileSetResource,
        color: Color,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let tile_set = resource.data_ref();
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.colliders.len() - 1))
        else {
            return;
        };
        let Some(uuid) = tile_set.colliders.get(sel_index).map(|l| l.uuid) else {
            return;
        };
        sender.do_command(SetColliderLayerColorCommand {
            tile_set: resource.clone(),
            uuid,
            color,
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
            .clamp(0, tile_set.colliders.len() - 1);
        if sel_index == new_index {
            return;
        }
        ui.send_message(ListViewMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            vec![new_index],
        ));
        sender.do_command(MoveColliderLayerCommand {
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
            .clamp(0, tile_set.colliders.len());
        ui.send_message(ListViewMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            vec![index],
        ));
        sender.do_command(AddColliderLayerCommand {
            tile_set: resource.clone(),
            index,
            uuid: Uuid::new_v4(),
        });
    }
    fn remove_layer(&self, resource: TileSetResource, ui: &UserInterface, sender: &MessageSender) {
        let tile_set = resource.data_ref();
        let Some(index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.colliders.len() - 1))
        else {
            return;
        };
        sender.do_command(RemoveColliderLayerCommand {
            tile_set: resource.clone(),
            index,
            layer: None,
            values: FxHashMap::default(),
        });
    }
}
