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

use fyrox::{
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*, ImmutableString,
    },
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{Button, ButtonBuilder, ButtonMessage},
        decorator::{DecoratorBuilder, DecoratorMessage},
        define_constructor, define_widget_deref,
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::UiMessage,
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
        utils::make_simple_tooltip,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_BRIGHT_BLUE, BRUSH_DARKER, BRUSH_LIGHT, BRUSH_LIGHTER,
        BRUSH_LIGHTEST,
    },
    scene::tilemap::tileset::*,
};
use std::ops::{Deref, DerefMut};

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum TilePropertyMessage {
    Value(TileSetPropertyOptionValue),
    SetSlice { index: usize, value: Option<i8> },
    SyncToState,
    UpdateProperty,
    UpdateValue,
}

impl TilePropertyMessage {
    define_constructor!(TilePropertyMessage:Value => fn value(TileSetPropertyOptionValue), layout: false);
    define_constructor!(TilePropertyMessage:SetSlice => fn set_slice(index: usize, value: Option<i8>), layout: false);
    define_constructor!(TilePropertyMessage:SyncToState => fn sync_to_state(), layout: false);
    define_constructor!(TilePropertyMessage:UpdateProperty => fn update_property(), layout: false);
    define_constructor!(TilePropertyMessage:UpdateValue => fn update_value(), layout: false);
}

#[derive(Clone, Default, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "d4be71a2-a9fc-41b4-9fcf-a767beb769ea")]
pub struct TilePropertyEditor {
    widget: Widget,
    tile_set: TileSetResource,
    prop_type: TileSetPropertyType,
    #[reflect(hidden)]
    state: TileDrawStateRef,
    property_id: Uuid,
    draw_value: DrawValue,
    value: TileSetPropertyOptionValue,
    paint_button: Handle<UiNode>,
    name_field: Handle<UiNode>,
    value_field: Handle<UiNode>,
    list: Handle<UiNode>,
    nine: Handle<UiNode>,
    nine_buttons: Option<Box<[Handle<UiNode>; 9]>>,
}

define_widget_deref!(TilePropertyEditor);

impl TilePropertyEditor {
    #[inline]
    pub fn property_id(&self) -> Uuid {
        self.property_id
    }
    fn sync_to_state(&mut self, ui: &mut UserInterface) {
        let state = self.state.lock();
        let paint_active =
            state.drawing_mode == DrawingMode::Property && state.active_prop == Some(self.id);
        highlight_tool_button(self.paint_button, paint_active, ui);
    }
    fn apply_property_update(&mut self, ui: &mut UserInterface) {
        let tile_set = self.tile_set.data_ref();
        let layer = tile_set.find_property(self.property_id).unwrap();
        ui.send_message(TextMessage::text(
            self.name_field,
            MessageDirection::ToWidget,
            layer.name.to_string(),
        ));
        let list = build_list(layer, &mut ui.build_ctx());
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
    }
    fn apply_value_update(&mut self, ui: &mut UserInterface) {
        let tile_set = self.tile_set.data_ref();
        let layer = tile_set.find_property(self.property_id).unwrap();
        let default_value = layer.prop_type.default_value();
        let state = self.state.lock();
        let tiles = state.selection_tiles();
        let mut value = TileSetPropertyOptionValue::default();
        for handle in tiles.values().copied() {
            let Some(data) = tile_set.get_tile_data(TilePaletteStage::Tiles, handle) else {
                continue;
            };
            let tile_value = data.properties.get(&self.id).unwrap_or(&default_value);
            value.intersect(tile_value);
        }
        match value {
            TileSetPropertyOptionValue::I32(Some(v)) => {
                ui.send_message(NumericUpDownMessage::value(
                    self.value_field,
                    MessageDirection::ToWidget,
                    v,
                ));
                self.update_list(ui);
            }
            TileSetPropertyOptionValue::I32(None) => {
                ui.send_message(NumericUpDownMessage::value(
                    self.value_field,
                    MessageDirection::ToWidget,
                    0,
                ));
                self.update_list(ui);
            }
            TileSetPropertyOptionValue::F32(Some(v)) => {
                ui.send_message(NumericUpDownMessage::value(
                    self.value_field,
                    MessageDirection::ToWidget,
                    v,
                ));
                self.update_list(ui);
            }
            TileSetPropertyOptionValue::F32(None) => {
                ui.send_message(NumericUpDownMessage::value(
                    self.value_field,
                    MessageDirection::ToWidget,
                    0.0,
                ));
                self.update_list(ui);
            }
            TileSetPropertyOptionValue::String(Some(v)) => {
                ui.send_message(TextMessage::text(
                    self.value_field,
                    MessageDirection::ToWidget,
                    v.to_string(),
                ));
            }
            TileSetPropertyOptionValue::String(None) => {
                ui.send_message(TextMessage::text(
                    self.value_field,
                    MessageDirection::ToWidget,
                    String::default(),
                ));
            }
            TileSetPropertyOptionValue::NineSlice(_) => {
                let specs = create_nine_specs(&value, layer);
                for (i, specs) in specs.iter().enumerate() {
                    apply_specs_to_nine(specs, self.nine_buttons.as_ref().unwrap()[i], ui);
                }
            }
        }
    }
    fn update_list(&self, ui: &mut UserInterface) {
        let tile_set = self.tile_set.data_ref();
        let layer = tile_set.find_property(self.property_id).unwrap();
        let index = layer
            .find_value_index(&self.value)
            .map(|x| x + 1)
            .unwrap_or(0);
        ui.send_message(DropdownListMessage::selection(
            self.list,
            MessageDirection::ToWidget,
            Some(index),
        ));
    }
    fn set_value_from_text(&mut self, v: ImmutableString, ui: &mut UserInterface) {
        self.value = TileSetPropertyOptionValue::String(Some(v.clone()));
        self.draw_value = DrawValue::String(v);
        ui.send_message(TilePropertyMessage::value(
            self.handle,
            MessageDirection::FromWidget,
            self.value.clone(),
        ));
    }
    fn set_value_from_i32(&mut self, v: i32, ui: &mut UserInterface) {
        self.value = TileSetPropertyOptionValue::I32(Some(v));
        self.draw_value = DrawValue::I32(v);
        ui.send_message(TilePropertyMessage::value(
            self.handle,
            MessageDirection::FromWidget,
            self.value.clone(),
        ));
        self.update_list(ui);
    }
    fn set_value_from_f32(&mut self, v: f32, ui: &mut UserInterface) {
        self.value = TileSetPropertyOptionValue::F32(Some(v));
        self.draw_value = DrawValue::F32(v);
        ui.send_message(TilePropertyMessage::value(
            self.handle,
            MessageDirection::FromWidget,
            self.value.clone(),
        ));
        self.update_list(ui);
    }
    fn set_value_from_i8(&mut self, v: i8, _ui: &mut UserInterface) {
        self.draw_value = DrawValue::I8(v);
    }
    fn handle_nine_click(&mut self, handle: Handle<UiNode>, ui: &mut UserInterface) {
        let index = self
            .nine_buttons
            .as_ref()
            .unwrap()
            .iter()
            .position(|x| *x == handle);
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
        ui.send_message(TilePropertyMessage::set_slice(
            self.handle,
            MessageDirection::FromWidget,
            index,
            Some(slice_value),
        ));
        let tile_set = self.tile_set.data_ref();
        let layer = tile_set.find_property(self.property_id).unwrap();
        let specs = create_nine_specs(&self.value, layer);
        apply_specs_to_nine(&specs[index], handle, ui);
        ui.send_message(TilePropertyMessage::value(
            self.handle,
            MessageDirection::FromWidget,
            self.value.clone(),
        ));
    }
}

impl Control for TilePropertyEditor {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.paint_button {
                let mut state = self.state.lock_mut();
                let paint_active = state.drawing_mode == DrawingMode::Property
                    && state.active_prop == Some(self.id);
                let paint_active = !paint_active;
                if paint_active {
                    state.drawing_mode = DrawingMode::Property;
                    state.active_prop = Some(self.id);
                } else {
                    state.drawing_mode = DrawingMode::Pick;
                    state.active_prop = None;
                }
            } else {
                self.handle_nine_click(message.destination(), ui);
            }
        } else if let Some(TilePropertyMessage::SyncToState) = message.data() {
            self.sync_to_state(ui);
        } else if let Some(TilePropertyMessage::UpdateProperty) = message.data() {
            self.apply_property_update(ui);
        } else if let Some(TilePropertyMessage::UpdateValue) = message.data() {
            self.apply_value_update(ui);
        } else if let Some(TextMessage::Text(v)) = message.data() {
            if message.direction == MessageDirection::FromWidget
                && message.destination() == self.value_field
            {
                self.set_value_from_text(v.into(), ui);
            }
        } else if let Some(&NumericUpDownMessage::<i32>::Value(v)) = message.data() {
            if message.direction == MessageDirection::FromWidget
                && message.destination() == self.value_field
            {
                self.set_value_from_i32(v, ui);
            }
        } else if let Some(&NumericUpDownMessage::<i8>::Value(v)) = message.data() {
            if message.direction == MessageDirection::FromWidget
                && message.destination() == self.value_field
            {
                self.set_value_from_i8(v, ui);
            }
        } else if let Some(&NumericUpDownMessage::<f32>::Value(v)) = message.data() {
            if message.direction == MessageDirection::FromWidget
                && message.destination() == self.value_field
            {
                self.set_value_from_f32(v, ui);
            }
        }
    }
}

fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
    let decorator = *ui.try_get_of_type::<Button>(button).unwrap().decorator;
    ui.send_message(DecoratorMessage::select(
        decorator,
        MessageDirection::ToWidget,
        highlight,
    ));
}

pub fn make_named_value_list_option(
    ctx: &mut BuildContext,
    color: Color,
    name: &str,
) -> Handle<UiNode> {
    let icon = BorderBuilder::new(
        WidgetBuilder::new()
            .on_column(0)
            .with_background(Brush::Solid(color)),
    )
    .build(ctx);
    let text = TextBuilder::new(WidgetBuilder::new().on_column(1))
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_horizontal_text_alignment(HorizontalAlignment::Left)
        .with_text(name)
        .build(ctx);
    let grid = GridBuilder::new(WidgetBuilder::new().with_child(icon).with_child(text))
        .add_column(Column::strict(20.0))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);
    DecoratorBuilder::new(
        BorderBuilder::new(WidgetBuilder::new().with_child(grid))
            .with_corner_radius(4.0)
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

fn build_list(layer: &TileSetPropertyLayer, ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    let custom = make_named_value_list_option(ctx, Color::TRANSPARENT, "Custom");
    std::iter::once(custom)
        .chain(
            layer
                .named_values
                .iter()
                .map(|v| make_named_value_list_option(ctx, v.color, &v.name)),
        )
        .collect()
}

#[derive(Default, Clone)]
struct NineButtonSpec {
    name: String,
    color: Color,
}

impl NineButtonSpec {
    fn base_color(&self) -> Color {
        let color = self.color.to_opaque();
        if color.r > 230 && color.g > 230 && color.b > 230 {
            color.lerp(Color::BLACK, 0.3)
        } else {
            color
        }
    }
    fn selected_brush(&self) -> Brush {
        Brush::Solid(
            self.base_color()
                .lerp(Color::from_rgba(80, 118, 178, 255), 0.8),
        )
    }
    fn normal_brush(&self) -> Brush {
        Brush::Solid(self.base_color().lerp(Color::WHITE, 0.4))
    }
    fn hover_brush(&self) -> Brush {
        Brush::Solid(self.base_color().lerp(Color::WHITE, 0.6))
    }
    fn pressed_brush(&self) -> Brush {
        Brush::Solid(self.base_color().lerp(Color::WHITE, 0.7))
    }
}

const PAINT_BUTTON_WIDTH: f32 = 20.0;
const PAINT_BUTTON_HEIGHT: f32 = 20.0;

fn make_paint_button(tab_index: Option<usize>, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, "Paint with value."))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(WidgetBuilder::new().with_foreground(BRUSH_DARKER))
                .with_pad_by_corner_radius(false)
                .with_corner_radius(4.0)
                .with_stroke_thickness(Thickness::uniform(1.0)),
        )
        .with_selected_brush(BRUSH_BRIGHT_BLUE)
        .with_normal_brush(BRUSH_LIGHT)
        .with_hover_brush(BRUSH_LIGHTER)
        .with_pressed_brush(BRUSH_LIGHTEST)
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(180, 180, 180)))
                .with_margin(Thickness::uniform(2.0))
                .with_width(PAINT_BUTTON_WIDTH)
                .with_height(PAINT_BUTTON_HEIGHT),
        )
        .with_opt_texture(BRUSH_IMAGE.clone())
        .build(ctx),
    )
    .build(ctx)
}

fn create_nine_specs(
    value: &TileSetPropertyOptionValue,
    layer: &TileSetPropertyLayer,
) -> [NineButtonSpec; 9] {
    if let TileSetPropertyOptionValue::NineSlice(value) = value {
        value.map(|z| {
            z.map(|x| NineButtonSpec {
                name: layer.value_to_name(NamableValue::I8(x)),
                color: layer.value_to_color(NamableValue::I8(x)),
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

fn apply_specs_to_nine(specs: &NineButtonSpec, handle: Handle<UiNode>, ui: &mut UserInterface) {
    ui.send_message(TextMessage::text(
        handle,
        MessageDirection::ToWidget,
        specs.name.clone(),
    ));
    let decorator = *ui
        .try_get_of_type::<Button>(handle)
        .unwrap()
        .decorator
        .clone();
    ui.send_message(DecoratorMessage::selected_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.selected_brush(),
    ));
    ui.send_message(DecoratorMessage::normal_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.normal_brush(),
    ));
    ui.send_message(DecoratorMessage::pressed_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.pressed_brush(),
    ));
    ui.send_message(DecoratorMessage::hover_brush(
        decorator,
        MessageDirection::ToWidget,
        specs.hover_brush(),
    ));
}

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
            .on_row(y)
            .with_tab_index(tab_index),
    )
    .with_text(specs.name.as_str())
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(WidgetBuilder::new().with_foreground(BRUSH_DARKER))
                .with_pad_by_corner_radius(false)
                .with_corner_radius(4.0)
                .with_stroke_thickness(Thickness::uniform(1.0)),
        )
        .with_selected_brush(specs.selected_brush())
        .with_normal_brush(specs.normal_brush())
        .with_hover_brush(specs.hover_brush())
        .with_pressed_brush(specs.pressed_brush())
        .build(ctx),
    )
    .build(ctx)
}

fn build_nine(
    specs: [NineButtonSpec; 9],
    ctx: &mut BuildContext,
) -> (Handle<UiNode>, Box<[Handle<UiNode>; 9]>) {
    let mut buttons: Box<[Handle<UiNode>; 9]> = [Handle::NONE; 9].into();
    let mut tab_index = 0;
    for y in (0..4).rev() {
        for x in 0..4 {
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

pub struct TilePropertyEditorBuilder {
    widget_builder: WidgetBuilder,
    tile_set: TileSetResource,
    state: TileDrawStateRef,
    prop_layer: TileSetPropertyLayer,
    value: TileSetPropertyOptionValue,
}

impl TilePropertyEditorBuilder {
    pub fn new(
        widget_builder: WidgetBuilder,
        tile_set: TileSetResource,
        state: TileDrawStateRef,
        prop_layer: TileSetPropertyLayer,
    ) -> Self {
        Self {
            widget_builder,
            tile_set,
            state,
            prop_layer,
            value: TileSetPropertyOptionValue::default(),
        }
    }
    pub fn with_value(mut self, value: TileSetPropertyOptionValue) -> Self {
        self.value = value;
        self
    }
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let paint_button = make_paint_button(Some(0), ctx);
        let name_field = TextBuilder::new(WidgetBuilder::new())
            .with_text(self.prop_layer.name.clone())
            .build(ctx);
        let value_field = match self.prop_layer.prop_type {
            TileSetPropertyType::I32 => {
                NumericUpDownBuilder::<i32>::new(WidgetBuilder::new()).build(ctx)
            }
            TileSetPropertyType::F32 => {
                NumericUpDownBuilder::<f32>::new(WidgetBuilder::new()).build(ctx)
            }
            TileSetPropertyType::String => TextBoxBuilder::new(WidgetBuilder::new()).build(ctx),
            TileSetPropertyType::NineSlice => {
                NumericUpDownBuilder::<i8>::new(WidgetBuilder::new()).build(ctx)
            }
        };
        let index = self
            .prop_layer
            .find_value_index(&self.value)
            .map(|x| x + 1)
            .unwrap_or(0);
        let list = DropdownListBuilder::new(
            WidgetBuilder::new().with_visibility(!self.prop_layer.named_values.is_empty()),
        )
        .with_items(build_list(&self.prop_layer, ctx))
        .with_selected(index)
        .build(ctx);
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_child(name_field)
                .with_child(value_field)
                .with_child(list),
        )
        .add_column(Column::strict(50.0))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        let pair = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(paint_button)
                .with_child(grid),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);
        let is_nine = matches!(self.prop_layer.prop_type, TileSetPropertyType::NineSlice);
        let (nine, nine_buttons) = if is_nine {
            let (nine, nine_buttons) =
                build_nine(create_nine_specs(&self.value, &self.prop_layer), ctx);
            (nine, Some(nine_buttons))
        } else {
            (Handle::NONE, None)
        };
        let content = if is_nine {
            StackPanelBuilder::new(WidgetBuilder::new().with_child(pair).with_child(nine))
                .build(ctx)
        } else {
            grid
        };
        let draw_value = match self.prop_layer.prop_type {
            TileSetPropertyType::I32 => DrawValue::I32(0),
            TileSetPropertyType::F32 => DrawValue::F32(0.0),
            TileSetPropertyType::String => DrawValue::String(ImmutableString::default()),
            TileSetPropertyType::NineSlice => DrawValue::I8(0),
        };
        ctx.add_node(UiNode::new(TilePropertyEditor {
            widget: self.widget_builder.with_child(content).build(),
            tile_set: self.tile_set,
            prop_type: self.prop_layer.prop_type,
            property_id: self.prop_layer.uuid,
            state: self.state,
            value: self.value,
            draw_value,
            paint_button,
            name_field,
            value_field,
            list,
            nine,
            nine_buttons,
        }))
    }
}
