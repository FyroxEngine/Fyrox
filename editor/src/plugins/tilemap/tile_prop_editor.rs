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

//! [`TilePropertyEditor`] is the [`TileEditor`] for editing a tile's property layer. It shows
//! the name of the layer, the current value for the selected tile, and optionally
//! provides a dropdown list of pre-defined values.
//!
//! It includes a button to activate [`DrawingMode::Editor`] which allows this value
//! to be applied to other tiles by clicking in the tile palette.

use commands::SetTileSetTilesCommand;
use fyrox::{
    core::{
        algebra::Vector2, color::Color, pool::Handle, type_traits::prelude::*, ImmutableString,
    },
    fxhash::FxHashMap,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{Button, ButtonBuilder, ButtonMessage},
        decorator::{DecoratorBuilder, DecoratorMessage},
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::UiMessage,
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
        utils::make_simple_tooltip,
        widget::WidgetBuilder,
        BuildContext, Thickness, UiNode, UserInterface,
    },
    scene::tilemap::{tileset::*, TileDataUpdate, TileSetUpdate},
};
use palette::Subposition;

use crate::{send_sync_message, MSG_SYNC_FLAG};

use super::*;

#[derive(Debug, Clone, Visit, Reflect, PartialEq)]
enum DrawValue {
    I8(i8),
    I32(i32),
    F32(f32),
    String(ImmutableString),
    Color(Color),
    Material(TileMaterialBounds),
    Collider(TileCollider),
}

impl Default for DrawValue {
    fn default() -> Self {
        Self::I32(0)
    }
}

/// This is the [`TileEditor`] for editing a tile's property layer. It shows
/// the name of the layer, the current value for the selected tile, and optionally
/// provides a dropdown list of pre-defined values.
///
/// It includes a button to activate [`DrawingMode::Editor`] which allows this value
/// to be applied to other tiles by clicking in the tile palette.
///
/// When the property type is [`TileSetPropertyType::NineSlice`], the editor will show
/// the value as a grid of nine buttons, and the user my click on those buttons to set
/// the corresponding element to whatever number is currently in the text field.
pub struct TilePropertyEditor {
    handle: Handle<UiNode>,
    /// The data type we are editing.
    prop_type: TileSetPropertyType,
    /// The UUID of the property layer that we are responsible for editing.
    property_id: Uuid,
    /// The value that we will draw into other tiles if editor draw tool is active.
    draw_value: DrawValue,
    /// The value of the currently selected tiles.
    value: TileSetPropertyOptionValue,
    /// The button to activate the editor draw tool that allows this property value
    /// to be applied to other tiles.
    draw_button: Handle<UiNode>,
    /// The label showing the name of the layer.
    name_field: Handle<UiNode>,
    /// The field for editing the current value.
    /// This may be several different widgets, depending on the type of the layer.
    /// For [`TileSetPropertyType::I32`] it will be a [`NumericUpDown`](fyrox::gui::numeric::NumericUpDown) for i32.
    /// For [`TileSetPropertyType::String`] it will be a [`TextBox`](fyrox::gui::text_box::TextBox), and so on.
    value_field: Handle<UiNode>,
    /// The dropdown list of pre-defined values.
    list: Handle<UiNode>,
    /// The handles of the buttons in the 9-button grid.
    nine_buttons: Option<Box<[Handle<UiNode>; 9]>>,
}

impl TilePropertyEditor {
    pub fn new(
        prop_layer: &TileSetPropertyLayer,
        value: &TileSetPropertyOptionValue,
        ctx: &mut BuildContext,
    ) -> Self {
        let draw_button = make_draw_button(Some(0), ctx);
        let name_field = TextBuilder::new(WidgetBuilder::new())
            .with_text(prop_layer.name.clone())
            .build(ctx);
        let value_field = match prop_layer.prop_type {
            TileSetPropertyType::I32 => {
                NumericUpDownBuilder::<i32>::new(WidgetBuilder::new().on_column(1)).build(ctx)
            }
            TileSetPropertyType::F32 => {
                NumericUpDownBuilder::<f32>::new(WidgetBuilder::new().on_column(1)).build(ctx)
            }
            TileSetPropertyType::String => {
                TextBoxBuilder::new(WidgetBuilder::new().on_column(1)).build(ctx)
            }
            TileSetPropertyType::NineSlice => {
                NumericUpDownBuilder::<i8>::new(WidgetBuilder::new().on_column(1)).build(ctx)
            }
        };
        let index = prop_layer
            .find_value_index_from_property(value)
            .map(|x| x + 1)
            .unwrap_or(0);
        let list = DropdownListBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .on_column(1)
                .with_visibility(!prop_layer.named_values.is_empty()),
        )
        .with_items(make_named_value_list_items(prop_layer, ctx))
        .with_selected(index)
        .build(ctx);
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_child(name_field)
                .with_child(value_field)
                .with_child(list),
        )
        .add_column(Column::strict(100.0))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        let pair = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(draw_button)
                .with_child(grid),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);
        let is_nine = matches!(prop_layer.prop_type, TileSetPropertyType::NineSlice);
        let (nine, nine_buttons) = if is_nine {
            let (nine, nine_buttons) = build_nine(create_nine_specs(value, prop_layer), ctx);
            (nine, Some(nine_buttons))
        } else {
            (Handle::NONE, None)
        };
        let handle = if is_nine {
            StackPanelBuilder::new(WidgetBuilder::new().with_child(pair).with_child(nine))
                .build(ctx)
        } else {
            pair
        };
        let draw_value = match prop_layer.prop_type {
            TileSetPropertyType::I32 => DrawValue::I32(0),
            TileSetPropertyType::F32 => DrawValue::F32(0.0),
            TileSetPropertyType::String => DrawValue::String(ImmutableString::default()),
            TileSetPropertyType::NineSlice => DrawValue::I8(0),
        };
        Self {
            handle,
            prop_type: prop_layer.prop_type,
            property_id: prop_layer.uuid,
            value: value.clone(),
            draw_value,
            draw_button,
            name_field,
            value_field,
            list,
            nine_buttons,
        }
    }
    /// The property layer may have changed, so update our widgets to reflect the current
    /// state of the layer.
    fn apply_property_update(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        let layer = state.find_property(self.property_id).unwrap();
        ui.send_message(TextMessage::text(
            self.name_field,
            MessageDirection::ToWidget,
            layer.name.to_string(),
        ));
        let list = make_named_value_list_items(layer, &mut ui.build_ctx());
        ui.send_message(DropdownListMessage::items(
            self.list,
            MessageDirection::ToWidget,
            list,
        ));
        ui.send_message(WidgetMessage::visibility(
            self.list,
            MessageDirection::ToWidget,
            !layer.named_values.is_empty(),
        ));
        self.sync_list_index(state, ui);
    }
    /// Scan the currently selected tiles to find the value that this editor should display.
    fn find_value(&self, state: &TileEditorState) -> TileSetPropertyOptionValue {
        let default_value = self.prop_type.default_value();
        let mut iter = state.tile_data().map(|(_, d)| {
            d.properties
                .get(&self.property_id)
                .unwrap_or(&default_value)
        });
        let Some(value) = iter.next() else {
            return self.prop_type.default_option_value();
        };
        let mut value = TileSetPropertyOptionValue::from(value.clone());
        for v in iter {
            value.intersect(v);
        }
        value
    }
    /// Update the value field to match the current value by sending sync messages.
    fn sync_value_to_field(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        match &self.value {
            &TileSetPropertyOptionValue::I32(Some(v)) => {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.value_field, MessageDirection::ToWidget, v),
                );
            }
            TileSetPropertyOptionValue::I32(None) => {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.value_field, MessageDirection::ToWidget, 0),
                );
            }
            &TileSetPropertyOptionValue::F32(Some(v)) => {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.value_field, MessageDirection::ToWidget, v),
                );
            }
            TileSetPropertyOptionValue::F32(None) => {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.value_field, MessageDirection::ToWidget, 0.0),
                );
            }
            TileSetPropertyOptionValue::String(Some(v)) => {
                send_sync_message(
                    ui,
                    TextMessage::text(self.value_field, MessageDirection::ToWidget, v.to_string()),
                );
            }
            TileSetPropertyOptionValue::String(None) => {
                send_sync_message(
                    ui,
                    TextMessage::text(
                        self.value_field,
                        MessageDirection::ToWidget,
                        String::default(),
                    ),
                );
            }
            TileSetPropertyOptionValue::NineSlice(_) => {
                if let DrawValue::I8(v) = self.draw_value {
                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.value_field,
                            MessageDirection::ToWidget,
                            v,
                        ),
                    );
                }
                if let Some(layer) = state.find_property(self.property_id) {
                    let specs = create_nine_specs(&self.value, layer);
                    for (i, specs) in specs.iter().enumerate() {
                        apply_specs_to_nine(specs, self.nine_buttons.as_ref().unwrap()[i], ui);
                    }
                }
            }
        }
    }
    /// Send a sync message to make the dropdown list selection match the current value.
    fn sync_list_index(&self, state: &TileEditorState, ui: &mut UserInterface) {
        let layer = state.find_property(self.property_id).unwrap();
        if layer.prop_type == TileSetPropertyType::String {
            return;
        }
        let index = if let DrawValue::I8(v) = self.draw_value {
            layer.find_value_index(NamableValue::I8(v))
        } else {
            layer.find_value_index_from_property(&self.value)
        }
        .map(|x| x + 1)
        .unwrap_or(0);
        send_sync_message(
            ui,
            DropdownListMessage::selection(self.list, MessageDirection::ToWidget, Some(index)),
        );
    }
    /// Update the value using an index from the dropdown list.
    fn set_value_from_list(
        &mut self,
        index: usize,
        state: &mut TileEditorState,
        ui: &mut UserInterface,
        sender: &MessageSender,
        tile_book: &TileBook,
    ) {
        if index == 0 {
            return;
        }
        let Some(layer) = state.find_property(self.property_id) else {
            return;
        };
        let Some(value) = layer.named_values.get(index.saturating_sub(1)) else {
            return;
        };
        match &value.value {
            NamableValue::I8(v) => {
                self.draw_value = DrawValue::I8(*v);
                self.sync_value_to_field(state, ui);
                state.touch();
            }
            NamableValue::I32(v) => {
                self.draw_value = DrawValue::I32(*v);
                self.value = TileSetPropertyOptionValue::I32(Some(*v));
                self.sync_value_to_field(state, ui);
                self.send_value(state, sender, tile_book);
            }
            NamableValue::F32(v) => {
                self.draw_value = DrawValue::F32(*v);
                self.value = TileSetPropertyOptionValue::F32(Some(*v));
                self.sync_value_to_field(state, ui);
                self.send_value(state, sender, tile_book);
            }
        }
    }
    fn set_value_from_text(&mut self, v: ImmutableString) {
        self.value = TileSetPropertyOptionValue::String(Some(v.clone()));
        self.draw_value = DrawValue::String(v);
    }
    fn set_value_from_i32(&mut self, v: i32, state: &TileEditorState, ui: &mut UserInterface) {
        self.value = TileSetPropertyOptionValue::I32(Some(v));
        self.draw_value = DrawValue::I32(v);
        self.sync_list_index(state, ui);
    }
    fn set_value_from_f32(&mut self, v: f32, state: &TileEditorState, ui: &mut UserInterface) {
        self.value = TileSetPropertyOptionValue::F32(Some(v));
        self.draw_value = DrawValue::F32(v);
        self.sync_list_index(state, ui);
    }
    fn set_value_from_i8(&mut self, v: i8) {
        self.draw_value = DrawValue::I8(v);
    }
    /// One of the nine buttons has been clicked, so set the corresponding element of the value.
    fn handle_nine_click(
        &mut self,
        handle: Handle<UiNode>,
        state: &TileEditorState,
        ui: &mut UserInterface,
        sender: &MessageSender,
        tile_book: &TileBook,
    ) {
        let Some(buttons) = self.nine_buttons.as_ref() else {
            return;
        };
        let index = buttons.iter().position(|x| *x == handle);
        let Some(index) = index else {
            return;
        };
        let DrawValue::I8(slice_value) = self.draw_value else {
            panic!();
        };
        let TileSetPropertyOptionValue::NineSlice(v) = &mut self.value else {
            panic!();
        };
        if v[index] == Some(slice_value) {
            return;
        }
        v[index] = Some(slice_value);
        let Some(layer) = state.find_property(self.property_id) else {
            return;
        };
        let specs = create_nine_specs(&self.value, layer);
        apply_specs_to_nine(&specs[index], handle, ui);
        let Some(tile_set) = tile_book.tile_set_ref().cloned() else {
            return;
        };
        let Some(page) = state.page() else {
            return;
        };
        let mut update = TileSetUpdate::default();
        for position in state.selected_positions() {
            update.set_property_slice(
                page,
                position,
                TileSetPropertyValue::index_to_nine_position(index),
                self.property_id,
                slice_value,
            );
        }
        sender.do_command(SetTileSetTilesCommand {
            tile_set,
            tiles: update,
        });
    }
    /// Use the sender to update the selected tiles with the current value.
    fn send_value(&self, state: &TileEditorState, sender: &MessageSender, tile_book: &TileBook) {
        let Some(tile_set) = tile_book.tile_set_ref().cloned() else {
            return;
        };
        let Some(page) = state.page() else {
            return;
        };
        let mut update = TileSetUpdate::default();
        for position in state.selected_positions() {
            update.set_property(
                page,
                position,
                self.property_id,
                Some(self.value.clone().into()),
            );
        }
        sender.do_command(SetTileSetTilesCommand {
            tile_set,
            tiles: update,
        });
    }
}

fn build_property_highlight_cell(
    position: Vector2<i32>,
    data: &TileData,
    data_update: Option<&TileDataUpdate>,
    layer: &TileSetPropertyLayer,
    draw_value: &DrawValue,
    highlight: &mut FxHashMap<Subposition, Color>,
) {
    use TileSetPropertyValueElement as Element;
    let value0 = data
        .properties
        .get(&layer.uuid)
        .cloned()
        .unwrap_or_else(|| layer.prop_type.default_value());
    let value = if let Some(update) = data_update {
        update.apply_to_property_value(layer.uuid, value0.clone())
    } else {
        value0.clone()
    };
    let element_value = match draw_value {
        DrawValue::I8(v) => Element::I8(*v),
        DrawValue::I32(v) => Element::I32(*v),
        DrawValue::F32(v) => Element::F32(*v),
        DrawValue::String(v) => Element::String(v.clone()),
        _ => return,
    };
    for x in 0..3 {
        for y in 0..3 {
            let subtile = Vector2::new(x, y);
            let pos = Subposition {
                tile: position,
                subtile,
            };
            let Some(color) = layer.highlight_color(subtile, &value, &element_value) else {
                continue;
            };
            let _ = highlight.insert(pos, color);
        }
    }
}

impl TileEditor for TilePropertyEditor {
    fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    fn draw_button(&self) -> Handle<UiNode> {
        self.draw_button
    }
    fn slice_mode(&self) -> bool {
        self.prop_type == TileSetPropertyType::NineSlice
    }

    fn highlight(
        &self,
        highlight: &mut FxHashMap<palette::Subposition, Color>,
        page: Vector2<i32>,
        tile_book: &TileBook,
        update: &TileSetUpdate,
    ) {
        let Some(tile_set) = tile_book.tile_set_ref() else {
            return;
        };
        let tile_set = tile_set.data_ref();
        let property_id = self.property_id;
        let Some(layer) = tile_set.find_property(property_id) else {
            return;
        };
        let draw_value = &self.draw_value;
        let Some(page_source) = tile_set.get_page(page).map(|p| &p.source) else {
            return;
        };
        match page_source {
            TileSetPageSource::Atlas(source) => {
                for (position, data) in source.iter() {
                    let data_update =
                        TileDefinitionHandle::try_new(page, *position).and_then(|h| update.get(&h));
                    build_property_highlight_cell(
                        *position,
                        data,
                        data_update,
                        layer,
                        draw_value,
                        highlight,
                    );
                }
            }
            TileSetPageSource::Freeform(source) => {
                for (position, def) in source.iter() {
                    let data_update =
                        TileDefinitionHandle::try_new(page, *position).and_then(|h| update.get(&h));
                    build_property_highlight_cell(
                        *position,
                        &def.data,
                        data_update,
                        layer,
                        draw_value,
                        highlight,
                    );
                }
            }
            _ => (),
        }
    }

    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        self.apply_property_update(state, ui);
    }

    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        self.value = self.find_value(state);
        self.sync_value_to_field(state, ui);
        match &self.value {
            TileSetPropertyOptionValue::I32(v) => {
                self.draw_value = DrawValue::I32(v.unwrap_or_default())
            }
            TileSetPropertyOptionValue::F32(v) => {
                self.draw_value = DrawValue::F32(v.unwrap_or_default())
            }
            TileSetPropertyOptionValue::String(v) => {
                self.draw_value = DrawValue::String(v.as_ref().cloned().unwrap_or_default())
            }
            TileSetPropertyOptionValue::NineSlice(_) => (),
        };
        self.sync_list_index(state, ui);
    }

    fn draw_tile(
        &self,
        handle: TileDefinitionHandle,
        subposition: Vector2<usize>,
        _state: &TileDrawState,
        update: &mut TileSetUpdate,
        _tile_resource: &TileBook,
    ) {
        use TileSetPropertyValue as Value;
        let page = handle.page();
        let position = handle.tile();
        let property_id = self.property_id;
        match &self.draw_value {
            DrawValue::I8(v) => {
                update.set_property_slice(page, position, subposition, property_id, *v)
            }
            DrawValue::I32(v) => {
                update.set_property(page, position, property_id, Some(Value::I32(*v)))
            }
            DrawValue::F32(v) => {
                update.set_property(page, position, property_id, Some(Value::F32(*v)))
            }
            DrawValue::String(v) => {
                update.set_property(page, position, property_id, Some(Value::String(v.clone())))
            }
            _ => (),
        }
    }

    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &UiMessage,
        ui: &mut UserInterface,
        tile_book: &TileBook,
        sender: &MessageSender,
    ) {
        if message.flags == MSG_SYNC_FLAG || message.direction() == MessageDirection::ToWidget {
            return;
        }
        if let Some(ButtonMessage::Click) = message.data() {
            self.handle_nine_click(message.destination(), state, ui, sender, tile_book);
        } else if let Some(TextMessage::Text(v)) = message.data() {
            if message.destination() == self.value_field {
                self.set_value_from_text(v.into());
                self.send_value(state, sender, tile_book);
            }
        } else if let Some(&NumericUpDownMessage::<i32>::Value(v)) = message.data() {
            if message.destination() == self.value_field {
                self.set_value_from_i32(v, state, ui);
                self.send_value(state, sender, tile_book);
            }
        } else if let Some(&NumericUpDownMessage::<i8>::Value(v)) = message.data() {
            if message.destination() == self.value_field {
                self.set_value_from_i8(v);
                state.touch();
            }
        } else if let Some(&NumericUpDownMessage::<f32>::Value(v)) = message.data() {
            if message.destination() == self.value_field {
                self.set_value_from_f32(v, state, ui);
                self.send_value(state, sender, tile_book);
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.list {
                self.set_value_from_list(*index, state, ui, sender, tile_book);
            }
        }
    }
}

/// Each of the nine buttons has a label and a color to indicate
/// which number is stored in the corresponding element of the value.
/// When the number matches one of the pre-defined values, then the
/// name and color of the pre-defined value will be used.
#[derive(Default, Clone)]
struct NineButtonSpec {
    name: String,
    color: Color,
}

/// If the color of the button is too bright for white text,
/// then the text will be rendered as black.
const BRIGHTNESS_LIMIT: usize = 500;

impl NineButtonSpec {
    fn base_color(&self) -> Color {
        self.color.to_opaque()
    }
    /// True if the color of the button is too bright for white text.
    fn is_bright(&self) -> bool {
        let r = self.color.r as usize;
        let b = self.color.b as usize;
        let g = self.color.g as usize;
        let brightness = 2 * r + b + 3 * g;
        brightness > BRIGHTNESS_LIMIT
    }
    /// The text color for the button.
    fn foreground_brush(&self) -> Brush {
        if self.is_bright() {
            Brush::Solid(Color::BLACK)
        } else {
            Brush::Solid(Color::WHITE)
        }
    }
    /// The color of the button when selected.
    fn selected_brush(&self) -> Brush {
        Brush::Solid(
            self.base_color()
                .lerp(Color::from_rgba(80, 118, 178, 255), 0.8),
        )
    }
    /// The color of the button.
    fn normal_brush(&self) -> Brush {
        Brush::Solid(self.base_color())
    }
    /// The color of the botton when the mouse is over.
    fn hover_brush(&self) -> Brush {
        Brush::Solid(self.base_color().lerp(Color::WHITE, 0.6))
    }
    /// The color of the button when pressed.
    fn pressed_brush(&self) -> Brush {
        Brush::Solid(self.base_color().lerp(Color::WHITE, 0.7))
    }
}

const DRAW_BUTTON_WIDTH: f32 = 20.0;
const DRAW_BUTTON_HEIGHT: f32 = 20.0;

fn make_draw_button(tab_index: Option<usize>, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, "Apply property value to tiles"))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius((4.0).into())
            .with_stroke_thickness(Thickness::uniform(1.0).into()),
        )
        .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
        .with_normal_brush(ctx.style.property(Style::BRUSH_LIGHT))
        .with_hover_brush(ctx.style.property(Style::BRUSH_LIGHTER))
        .with_pressed_brush(ctx.style.property(Style::BRUSH_LIGHTEST))
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(180, 180, 180)).into())
                .with_margin(Thickness::uniform(2.0))
                .with_width(DRAW_BUTTON_WIDTH)
                .with_height(DRAW_BUTTON_HEIGHT),
        )
        .with_opt_texture(BRUSH_IMAGE.clone())
        .build(ctx),
    )
    .build(ctx)
}

/// Use the given property value to construct nine NineButtonSpec objects,
/// one for each of the elements of the value.
fn create_nine_specs(
    value: &TileSetPropertyOptionValue,
    layer: &TileSetPropertyLayer,
) -> [NineButtonSpec; 9] {
    if let TileSetPropertyOptionValue::NineSlice(value) = value {
        value.map(|z| {
            z.map(|x| NineButtonSpec {
                name: layer.value_to_name(NamableValue::I8(x)),
                color: layer
                    .value_to_color(NamableValue::I8(x))
                    .unwrap_or(ELEMENT_MATCH_HIGHLIGHT_COLOR),
            })
            .unwrap_or_else(|| NineButtonSpec {
                name: "?".into(),
                color: Color::LIGHT_GRAY,
            })
        })
    } else {
        std::array::from_fn(|_| NineButtonSpec {
            name: String::default(),
            color: Color::default(),
        })
    }
}

/// Send UI messages to update the one of nine buttons to match the given [`NineButtonSpec`].
fn apply_specs_to_nine(specs: &NineButtonSpec, handle: Handle<UiNode>, ui: &mut UserInterface) {
    let button = ui.try_get_of_type::<Button>(handle).unwrap();
    let text = *button.content.clone();
    let decorator = *button.decorator.clone();
    ui.send_message(TextMessage::text(
        text,
        MessageDirection::ToWidget,
        specs.name.clone(),
    ));
    ui.send_message(WidgetMessage::foreground(
        text,
        MessageDirection::ToWidget,
        specs.foreground_brush().into(),
    ));
    ui.send_message(DecoratorMessage::selected_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.selected_brush().into(),
    ));
    ui.send_message(DecoratorMessage::normal_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.normal_brush().into(),
    ));
    ui.send_message(DecoratorMessage::pressed_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.pressed_brush().into(),
    ));
    ui.send_message(DecoratorMessage::hover_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.hover_brush().into(),
    ));
}

/// Use [`ButtonBuilder`] to create a buttons to represent one of the nine buttons
/// that will represent the elements of a nine-slice property value.
fn build_nine_button(
    specs: &NineButtonSpec,
    x: usize,
    y: usize,
    tab_index: Option<usize>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(x)
            .on_row(2 - y)
            .with_tab_index(tab_index),
    )
    .with_text(specs.name.as_str())
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius((4.0).into())
            .with_stroke_thickness(Thickness::uniform(1.0).into()),
        )
        .with_selected_brush(specs.selected_brush().into())
        .with_normal_brush(specs.normal_brush().into())
        .with_hover_brush(specs.hover_brush().into())
        .with_pressed_brush(specs.pressed_brush().into())
        .build(ctx),
    )
    .build(ctx)
}

/// Build the grid of nine buttons using the given list of [`NineButtonSpec`] to control the color and label
/// of each button.
fn build_nine(
    specs: [NineButtonSpec; 9],
    ctx: &mut BuildContext,
) -> (Handle<UiNode>, Box<[Handle<UiNode>; 9]>) {
    let mut buttons: Box<[Handle<UiNode>; 9]> = [Handle::NONE; 9].into();
    let mut tab_index = 0;
    for y in 0..3 {
        for x in 0..3 {
            let i = TileSetPropertyValue::nine_position_to_index(Vector2::new(x, y));
            buttons[i] = build_nine_button(&specs[i], x, y, Some(tab_index), ctx);
            tab_index += 1;
        }
    }
    let nine = GridBuilder::new(WidgetBuilder::new().with_children(buttons.iter().copied()))
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
    (nine, buttons)
}
