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

//! This is the tab of the tile set editor that allows the user to modify the property
//! layers stored within the tile set. Layers can be created, deleted, renamed
//! and pre-defined values can be edited.

use fyrox::{
    fxhash::FxHashMap,
    gui::{
        button::ButtonMessage,
        color::{ColorFieldBuilder, ColorFieldMessage},
        grid::*,
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        utils::{make_arrow, ArrowDirection},
        HorizontalAlignment, VerticalAlignment,
    },
    scene::tilemap::tileset::{
        NamableValue, NamedValue, OptionTileSet, TileSetPropertyLayer, TileSetPropertyType,
        TileSetRef,
    },
};

use crate::{send_sync_message, MSG_SYNC_FLAG};

use super::*;
use commands::*;

/// This is the tab of the tile set editor that allows the user to modify the property
/// layers stored within the tile set. Layers can be created, deleted, renamed
/// and pre-defined values can be edited.
pub struct PropertiesTab {
    handle: Handle<UiNode>,
    list: Handle<UiNode>,
    up_button: Handle<UiNode>,
    down_button: Handle<UiNode>,
    remove_button: Handle<UiNode>,
    add_int_button: Handle<UiNode>,
    add_float_button: Handle<UiNode>,
    add_string_button: Handle<UiNode>,
    add_nine_button: Handle<UiNode>,
    name_field: Handle<UiNode>,
    name_list: Handle<UiNode>,
    name_up: Handle<UiNode>,
    name_down: Handle<UiNode>,
    name_add: Handle<UiNode>,
    name_remove: Handle<UiNode>,
    data_panel: Handle<UiNode>,
    name_edit_panel: Handle<UiNode>,
    value_name_field: Handle<UiNode>,
    color_field: Handle<UiNode>,
    i8_field: Handle<UiNode>,
    i32_field: Handle<UiNode>,
    f32_field: Handle<UiNode>,
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

fn make_type_widget(ctx: &mut BuildContext, prop_type: TileSetPropertyType) -> Handle<UiNode> {
    let type_name = match prop_type {
        TileSetPropertyType::I32 => "INTEGER",
        TileSetPropertyType::F32 => "FLOAT",
        TileSetPropertyType::String => "STRING",
        TileSetPropertyType::NineSlice => "NINE",
    };
    TextBuilder::new(WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center))
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Right)
        .with_font_size((10.0).into())
        .with_text(type_name)
        .build(ctx)
}

fn make_list_item(ctx: &mut BuildContext, property: &TileSetPropertyLayer) -> Handle<UiNode> {
    let content = GridBuilder::new(
        WidgetBuilder::new()
            .with_child(make_type_widget(ctx, property.prop_type))
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::left(10.0))
                        .on_column(1),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Left)
                .with_text(property.name.clone())
                .build(ctx),
            ),
    )
    .add_row(Row::auto())
    .add_column(Column::strict(55.0))
    .add_column(Column::stretch())
    .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(content))
            .with_corner_radius((4.0).into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

fn make_items(ctx: &mut BuildContext, tile_set: &OptionTileSet) -> Vec<Handle<UiNode>> {
    tile_set
        .properties()
        .iter()
        .map(|p| make_list_item(ctx, p))
        .collect()
}

fn make_value_widget(ctx: &mut BuildContext, value: NamableValue) -> Handle<UiNode> {
    let text = match value {
        NamableValue::I8(x) => x.to_string(),
        NamableValue::I32(x) => x.to_string(),
        NamableValue::F32(x) => x.to_string(),
    };
    TextBuilder::new(WidgetBuilder::new().with_vertical_alignment(VerticalAlignment::Center))
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Right)
        .with_text(text)
        .build(ctx)
}

fn make_name_list_item(ctx: &mut BuildContext, named_value: &NamedValue) -> Handle<UiNode> {
    let content = GridBuilder::new(
        WidgetBuilder::new()
            .with_child(
                BorderBuilder::new(
                    WidgetBuilder::new()
                        .on_column(1)
                        .with_horizontal_alignment(HorizontalAlignment::Center)
                        .with_vertical_alignment(VerticalAlignment::Center)
                        .with_width(16.0)
                        .with_height(16.0)
                        .with_background(Brush::Solid(named_value.color).into()),
                )
                .build(ctx),
            )
            .with_child(make_value_widget(ctx, named_value.value))
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::left(10.0))
                        .on_column(2),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Left)
                .with_text(&named_value.name)
                .build(ctx),
            ),
    )
    .add_row(Row::auto())
    .add_column(Column::stretch())
    .add_column(Column::strict(24.0))
    .add_column(Column::generic(SizeMode::Stretch, 180.0))
    .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(content))
            .with_corner_radius((4.0).into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

fn make_name_items(ctx: &mut BuildContext, property: &TileSetPropertyLayer) -> Vec<Handle<UiNode>> {
    property
        .named_values
        .iter()
        .map(|p| make_name_list_item(ctx, p))
        .collect()
}

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

fn send_enabled(ui: &UserInterface, destination: Handle<UiNode>, enabled: bool) {
    ui.send_message(WidgetMessage::enabled(
        destination,
        MessageDirection::ToWidget,
        enabled,
    ));
}

impl PropertiesTab {
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
        let up_down = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(up_button)
                .with_child(down_button),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);
        let add_int_button = make_button("New Int", "Create an integer property.", ctx, 0, 0);
        let add_float_button =
            make_button("New Float", "Create a floating point property.", ctx, 1, 0);
        let add_string_button = make_button("New String", "Create a string property.", ctx, 2, 0);
        let add_nine_button = make_button("New Nine", "Create a nine-slice property.", ctx, 3, 0);
        let creation_buttons = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(3)
                .with_margin(Thickness::uniform(2.0))
                .with_child(add_int_button)
                .with_child(add_float_button)
                .with_child(add_string_button)
                .with_child(add_nine_button),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);
        let left_label = TextBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Properties:")
        .build(ctx);
        let left_side = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(left_label)
                .with_child(list)
                .with_child(up_down)
                .with_child(creation_buttons),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);
        let name_text_0 = TextBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_margin(Thickness::right(4.0)),
        )
        .with_text("Name:")
        .build(ctx);
        let name_text_1 = TextBuilder::new(
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
            "Delete this property from every tile in the tile set.",
            ctx,
            2,
            0,
        );
        let header = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(name_text_0)
                .with_child(name_field)
                .with_child(remove_button),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::strict(100.0))
        .add_row(Row::auto())
        .build(ctx);
        let name_list = ListViewBuilder::new(WidgetBuilder::new().on_row(1))
            .with_scroll_viewer(ScrollViewerBuilder::new(WidgetBuilder::new()).build(ctx))
            .build(ctx);
        let name_up = make_arrow_button(ctx, ArrowDirection::Top, 0, 0);
        let name_down = make_arrow_button(ctx, ArrowDirection::Bottom, 1, 0);
        let name_add = make_button(
            "Add",
            "Add a new name to the list of named values.",
            ctx,
            2,
            0,
        );
        let name_remove = make_button(
            "Remove",
            "Remove this name from the list of named values.",
            ctx,
            2,
            0,
        );
        let name_up_down = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(name_up)
                .with_child(name_down)
                .with_child(name_add),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);
        let value_name_field =
            TextBoxBuilder::new(WidgetBuilder::new().with_height(20.0).on_column(1))
                .with_text_commit_mode(TextCommitMode::Changed)
                .build(ctx);
        let edit_header = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(name_text_1)
                .with_child(value_name_field)
                .with_child(name_remove),
        )
        .add_row(Row::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::strict(100.0))
        .build(ctx);
        let color_field = ColorFieldBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(4.0))
                .on_row(1)
                .with_height(30.0),
        )
        .build(ctx);
        let i8_field = NumericUpDownBuilder::<i8>::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(8.0))
                .with_visibility(false),
        )
        .build(ctx);
        let i32_field = NumericUpDownBuilder::<i32>::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(8.0))
                .with_visibility(true),
        )
        .build(ctx);
        let f32_field = NumericUpDownBuilder::<f32>::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(8.0))
                .with_visibility(false),
        )
        .build(ctx);
        let edit_content = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(4.0))
                .with_child(i8_field)
                .with_child(i32_field)
                .with_child(f32_field)
                .with_child(color_field),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::strict(24.0))
        .build(ctx);
        let name_edit_panel = BorderBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_row(3)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(4.0))
                            .with_child(edit_header)
                            .with_child(edit_content),
                    )
                    .build(ctx),
                ),
        )
        .build(ctx);
        let data_panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .on_column(1)
                .with_child(header)
                .with_child(name_list)
                .with_child(name_up_down)
                .with_child(name_edit_panel),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
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
            add_int_button,
            add_float_button,
            add_string_button,
            add_nine_button,
            remove_button,
            name_field,
            name_list,
            name_up,
            name_down,
            name_remove,
            name_add,
            data_panel,
            name_edit_panel,
            value_name_field,
            color_field,
            i8_field,
            i32_field,
            f32_field,
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
    pub fn sync_data(&mut self, tile_set: &OptionTileSet, ui: &mut UserInterface) {
        let sel_index = self.selection_index(ui);
        let name = match sel_index {
            Some(index) => tile_set
                .properties()
                .get(index)
                .map(|c| c.name.to_string())
                .unwrap_or_default(),
            None => String::default(),
        };
        let prop = self.property(tile_set, ui);
        let named_values = match prop {
            Some(prop) => make_name_items(&mut ui.build_ctx(), prop),
            None => Vec::default(),
        };
        ui.send_message(ListViewMessage::items(
            self.name_list,
            MessageDirection::ToWidget,
            named_values,
        ));
        send_enabled(ui, self.data_panel, sel_index.is_some());
        send_sync_message(
            ui,
            TextMessage::text(self.name_field, MessageDirection::ToWidget, name),
        );
        send_enabled(ui, self.name_list, sel_index.is_some());
        self.sync_name_edit(sel_index.is_some(), tile_set, ui);
    }
    fn sync_name_edit(&mut self, enabled: bool, tile_set: &OptionTileSet, ui: &mut UserInterface) {
        let prop = self.property(tile_set, ui);
        let name_index = self.name_selection_index(ui);
        let value_name = match (name_index, prop) {
            (Some(index), Some(prop)) => prop
                .named_values
                .get(index)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "".into()),
            _ => "".into(),
        };
        let color = match (name_index, prop) {
            (Some(index), Some(prop)) => prop
                .named_values
                .get(index)
                .map(|c| c.color)
                .unwrap_or(Color::BLACK),
            _ => Color::BLACK,
        };
        send_sync_message(
            ui,
            TextMessage::text(
                self.value_name_field,
                MessageDirection::ToWidget,
                value_name,
            ),
        );
        send_sync_message(
            ui,
            ColorFieldMessage::color(self.color_field, MessageDirection::ToWidget, color),
        );
        let named_value = name_index.and_then(|i| prop.and_then(|p| p.named_values.get(i)));
        let value = named_value.map(|v| v.value);
        match value {
            Some(NamableValue::I8(value)) => send_sync_message(
                ui,
                NumericUpDownMessage::value(self.i8_field, MessageDirection::ToWidget, value),
            ),
            Some(NamableValue::I32(value)) => send_sync_message(
                ui,
                NumericUpDownMessage::value(self.i32_field, MessageDirection::ToWidget, value),
            ),
            Some(NamableValue::F32(value)) => send_sync_message(
                ui,
                NumericUpDownMessage::value(self.f32_field, MessageDirection::ToWidget, value),
            ),
            None => (),
        }
        let show_i8 = matches!(value, Some(NamableValue::I8(_))) && enabled;
        let show_i32 = matches!(value, Some(NamableValue::I32(_))) && enabled;
        let show_f32 = matches!(value, Some(NamableValue::F32(_))) && enabled;
        let show_any = show_i8 || show_i32 || show_f32;
        send_visibility(ui, self.i8_field, show_i8);
        send_visibility(ui, self.i32_field, show_i32);
        send_visibility(ui, self.f32_field, show_f32 || !show_any);
        send_enabled(ui, self.name_up, show_any);
        send_enabled(ui, self.name_down, show_any);
        send_enabled(
            ui,
            self.name_add,
            prop.is_some() && prop.unwrap().prop_type != TileSetPropertyType::String,
        );
        send_enabled(ui, self.name_edit_panel, show_any);
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
            } else if message.destination() == self.name_list {
                self.sync_name_edit(
                    self.selection_index(ui).is_some(),
                    &TileSetRef::new(&tile_set).as_loaded(),
                    ui,
                );
            }
        } else if let Some(TextMessage::Text(value)) = message.data() {
            if message.destination() == self.name_field {
                self.update_name(tile_set, value, ui, sender);
            } else if message.destination() == self.value_name_field {
                self.update_value_name(tile_set, value, ui, sender);
            }
        } else if let Some(&NumericUpDownMessage::<i8>::Value(value)) = message.data() {
            if message.destination() == self.i8_field {
                self.update_value(tile_set, NamableValue::I8(value), ui, sender);
            }
        } else if let Some(&NumericUpDownMessage::<i32>::Value(value)) = message.data() {
            if message.destination() == self.i32_field {
                self.update_value(tile_set, NamableValue::I32(value), ui, sender);
            }
        } else if let Some(&NumericUpDownMessage::<f32>::Value(value)) = message.data() {
            if message.destination() == self.f32_field {
                self.update_value(tile_set, NamableValue::F32(value), ui, sender);
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
            } else if message.destination() == self.add_int_button {
                self.add_layer(tile_set, TileSetPropertyType::I32, ui, sender);
            } else if message.destination() == self.add_float_button {
                self.add_layer(tile_set, TileSetPropertyType::F32, ui, sender);
            } else if message.destination() == self.add_string_button {
                self.add_layer(tile_set, TileSetPropertyType::String, ui, sender);
            } else if message.destination() == self.add_nine_button {
                self.add_layer(tile_set, TileSetPropertyType::NineSlice, ui, sender);
            } else if message.destination() == self.remove_button {
                self.remove_layer(tile_set, ui, sender);
            } else if message.destination() == self.name_add {
                self.add_name(tile_set, ui, sender);
            } else if message.destination() == self.name_remove {
                self.remove_name(tile_set, ui, sender);
            } else if message.destination() == self.name_up {
                self.move_name(tile_set, -1, ui, sender);
            } else if message.destination() == self.name_down {
                self.move_name(tile_set, 1, ui, sender);
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
    fn property<'a>(
        &self,
        tile_set: &'a OptionTileSet,
        ui: &UserInterface,
    ) -> Option<&'a TileSetPropertyLayer> {
        let sel_index = self.selection_index(ui)?;
        tile_set.properties().get(sel_index)
    }
    fn name_selection_index(&self, ui: &UserInterface) -> Option<usize> {
        ui.node(self.name_list)
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
        let tile_set = resource.data_ref();
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.properties.len() - 1))
        else {
            return;
        };
        let Some(uuid) = tile_set.properties.get(sel_index).map(|l| l.uuid) else {
            return;
        };
        sender.do_command(SetPropertyLayerNameCommand {
            tile_set: resource.clone(),
            uuid,
            name: name.into(),
        });
    }
    fn update_value_name(
        &self,
        resource: TileSetResource,
        name: &str,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let tile_set = resource.data_ref();
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.properties.len() - 1))
        else {
            return;
        };
        let Some(prop) = tile_set.properties.get(sel_index) else {
            return;
        };
        let Some(name_index) = self
            .name_selection_index(ui)
            .map(|i| i.clamp(0, prop.named_values.len() - 1))
        else {
            return;
        };
        let Some(uuid) = tile_set.properties.get(sel_index).map(|l| l.uuid) else {
            return;
        };
        sender.do_command(SetPropertyValueNameCommand {
            tile_set: resource.clone(),
            uuid,
            name_index,
            name: name.into(),
        });
    }
    fn update_value(
        &self,
        resource: TileSetResource,
        value: NamableValue,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let tile_set = resource.data_ref();
        let Some(sel_index) = self
            .selection_index(ui)
            .map(|i| i.clamp(0, tile_set.properties.len() - 1))
        else {
            return;
        };
        let Some(prop) = tile_set.properties.get(sel_index) else {
            return;
        };
        let Some(name_index) = self
            .name_selection_index(ui)
            .map(|i| i.clamp(0, prop.named_values.len() - 1))
        else {
            return;
        };
        let Some(uuid) = tile_set.properties.get(sel_index).map(|l| l.uuid) else {
            return;
        };
        sender.do_command(SetPropertyValueCommand {
            tile_set: resource.clone(),
            uuid,
            name_index,
            value,
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
            .map(|i| i.clamp(0, tile_set.properties.len() - 1))
        else {
            return;
        };
        let Some(prop) = tile_set.properties.get(sel_index) else {
            return;
        };
        let Some(name_index) = self
            .name_selection_index(ui)
            .map(|i| i.clamp(0, prop.named_values.len() - 1))
        else {
            return;
        };
        let Some(uuid) = tile_set.properties.get(sel_index).map(|l| l.uuid) else {
            return;
        };
        sender.do_command(SetPropertyValueColorCommand {
            tile_set: resource.clone(),
            uuid,
            name_index,
            color,
        });
    }
    fn move_name(
        &self,
        resource: TileSetResource,
        amount: isize,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let mut tile_set_guard = TileSetRef::new(&resource);
        let tile_set = tile_set_guard.as_loaded();
        let Some(prop) = self.property(&tile_set, ui) else {
            return;
        };
        let Some(sel_index) = self.name_selection_index(ui) else {
            return;
        };
        let new_index = sel_index
            .saturating_add_signed(amount)
            .clamp(0, prop.named_values.len() - 1);
        if sel_index == new_index {
            return;
        }
        ui.send_message(ListViewMessage::selection(
            self.name_list,
            MessageDirection::ToWidget,
            vec![new_index],
        ));
        let uuid = prop.uuid;
        drop(tile_set_guard);
        sender.do_command(MovePropertyValueCommand {
            tile_set: resource,
            uuid,
            start: sel_index,
            end: new_index,
        });
    }
    fn add_name(&self, resource: TileSetResource, ui: &UserInterface, sender: &MessageSender) {
        let mut tile_set_guard = TileSetRef::new(&resource);
        let tile_set = tile_set_guard.as_loaded();
        let Some(prop) = self.property(&tile_set, ui) else {
            return;
        };
        if prop.prop_type == TileSetPropertyType::String {
            return;
        }
        let index = self
            .name_selection_index(ui)
            .map(|i| i + 1)
            .unwrap_or(0)
            .clamp(0, prop.named_values.len());
        ui.send_message(ListViewMessage::selection(
            self.name_list,
            MessageDirection::ToWidget,
            vec![index],
        ));
        ui.send_message(WidgetMessage::focus(
            self.value_name_field,
            MessageDirection::ToWidget,
        ));
        let uuid = prop.uuid;
        let value_type = prop.prop_type;
        drop(tile_set_guard);
        sender.do_command(AddPropertyValueCommand {
            tile_set: resource,
            uuid,
            value_type,
            index,
        });
    }
    fn remove_name(&self, resource: TileSetResource, ui: &UserInterface, sender: &MessageSender) {
        let mut tile_set_guard = TileSetRef::new(&resource);
        let tile_set = tile_set_guard.as_loaded();
        let Some(prop) = self.property(&tile_set, ui) else {
            return;
        };
        let Some(index) = self
            .name_selection_index(ui)
            .map(|i| i.clamp(0, prop.named_values.len()))
        else {
            return;
        };
        let uuid = prop.uuid;
        drop(tile_set_guard);
        sender.do_command(RemovePropertyValueCommand {
            tile_set: resource,
            uuid,
            value: None,
            index,
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
            .clamp(0, tile_set.properties.len() - 1);
        if sel_index == new_index {
            return;
        }
        ui.send_message(ListViewMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            vec![new_index],
        ));
        sender.do_command(MovePropertyLayerCommand {
            tile_set: resource.clone(),
            start: sel_index,
            end: new_index,
        });
    }
    fn add_layer(
        &self,
        resource: TileSetResource,
        prop_type: TileSetPropertyType,
        ui: &UserInterface,
        sender: &MessageSender,
    ) {
        let tile_set = resource.data_ref();
        let index = self
            .selection_index(ui)
            .map(|i| i + 1)
            .unwrap_or(0)
            .clamp(0, tile_set.properties.len());
        ui.send_message(ListViewMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            vec![index],
        ));
        ui.send_message(WidgetMessage::focus(
            self.name_field,
            MessageDirection::ToWidget,
        ));
        sender.do_command(AddPropertyLayerCommand {
            tile_set: resource.clone(),
            index,
            uuid: Uuid::new_v4(),
            prop_type,
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
        sender.do_command(RemovePropertyLayerCommand {
            tile_set: resource.clone(),
            index,
            layer: None,
            values: FxHashMap::default(),
        });
    }
}
