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
    plugins::material::editor::{MaterialFieldEditorBuilder, MaterialFieldMessage},
    send_sync_message, MSG_SYNC_FLAG,
};

use super::*;
use commands::*;
use fyrox::{
    fxhash::FxHashMap,
    gui::{
        color::{ColorFieldBuilder, ColorFieldMessage},
        grid::*,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
    },
    material::{MaterialResource, MaterialResourceExtension},
    scene::tilemap::{tileset::*, *},
};
use palette::Subposition;

pub trait TileEditor: Send {
    fn handle(&self) -> Handle<UiNode>;
    fn draw_button(&self) -> Handle<UiNode>;
    fn slice_mode(&self) -> bool {
        false
    }
    #[allow(unused_variables)]
    fn highlight(
        &self,
        highlight: &mut FxHashMap<Subposition, Color>,
        page: Vector2<i32>,
        tile_resource: &TileResource,
        update: &TileSetUpdate,
    ) {
    }
    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface);
    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface);
    fn draw_tile(
        &self,
        handle: TileDefinitionHandle,
        subposition: Vector2<usize>,
        state: &TileDrawState,
        update: &mut TileSetUpdate,
        tile_resource: &TileResource,
    );
    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &UiMessage,
        ui: &mut UserInterface,
        tile_resource: &TileResource,
        sender: &MessageSender,
    );
}

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

fn make_draw_button(
    tooltip: &str,
    ctx: &mut BuildContext,
    tab_index: Option<usize>,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.get_or_default(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(2.0)
            .with_stroke_thickness(Thickness::uniform(1.0)),
        )
        .with_selected_brush(ctx.style.get_or_default(Style::BRUSH_BRIGHT_BLUE))
        .with_normal_brush(ctx.style.get_or_default(Style::BRUSH_LIGHT))
        .with_hover_brush(ctx.style.get_or_default(Style::BRUSH_LIGHTER))
        .with_pressed_brush(ctx.style.get_or_default(Style::BRUSH_LIGHTEST))
        .build(ctx),
    )
    .with_content(
        ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(180, 180, 180)))
                .with_width(16.0)
                .with_height(16.0),
        )
        .with_opt_texture(BRUSH_IMAGE.clone())
        .build(ctx),
    )
    .build(ctx)
}

fn make_drawable_field(
    label: &str,
    draw_button: Handle<UiNode>,
    field: Handle<UiNode>,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let label = make_label(label, ctx);
    GridBuilder::new(
        WidgetBuilder::new()
            .with_child(label)
            .with_child(draw_button)
            .with_child(field),
    )
    .add_row(Row::auto())
    .add_column(Column::strict(FIELD_LABEL_WIDTH))
    .add_column(Column::auto())
    .add_column(Column::stretch())
    .build(ctx)
}

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

pub struct TileMaterialEditor {
    handle: Handle<UiNode>,
    material_line: Handle<UiNode>,
    material_field: Handle<UiNode>,
    bounds_field: Handle<UiNode>,
    draw_button: Handle<UiNode>,
    material_bounds: TileMaterialBounds,
}

impl TileMaterialEditor {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let draw_button = make_draw_button("Apply material to tiles", ctx, None);
        let material = DEFAULT_TILE_MATERIAL.deep_copy();
        let material_field = MaterialFieldEditorBuilder::new(WidgetBuilder::new().on_column(2))
            .build(ctx, sender, material.clone());
        let material_line = make_drawable_field("Material", draw_button, material_field, ctx);
        let bounds_field = TileBoundsEditorBuilder::new(WidgetBuilder::new()).build(ctx);
        Self {
            handle: StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(material_line)
                    .with_child(bounds_field),
            )
            .build(ctx),
            material_line,
            material_field,
            bounds_field,
            draw_button,
            material_bounds: TileMaterialBounds {
                material,
                ..TileMaterialBounds::default()
            },
        }
    }
    fn find_material(state: &TileEditorState) -> Option<MaterialResource> {
        let mut iter = state.tile_material_bounds().map(|(_, b)| &b.material);
        let value = iter.next()?.clone();
        if iter.all(|m| m == &value) {
            Some(value)
        } else {
            None
        }
    }
    fn find_bounds(state: &TileEditorState) -> Option<TileBounds> {
        let mut iter = state.tile_material_bounds().map(|(_, b)| &b.bounds);
        let value = iter.next()?.clone();
        if iter.all(|b| b == &value) {
            Some(value)
        } else {
            None
        }
    }
    fn bounds_visible(state: &TileEditorState) -> bool {
        state.tile_material_bounds().next().is_some()
    }
    fn apply_material(
        material: &MaterialResource,
        state: &TileEditorState,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        let tiles = state.selected_positions();
        let Some(page) = state.page() else {
            return;
        };
        let TileResource::TileSet(tile_set) = tile_resource else {
            return;
        };
        let iter = tiles
            .filter_map(|p| {
                let handle = TileDefinitionHandle::try_new(page, p)?;
                tile_set
                    .data_ref()
                    .tile_bounds(handle)
                    .map(|b| (handle, b.bounds.clone()))
            })
            .map(|(handle, bounds)| {
                (
                    handle,
                    TileDataUpdate::Material(TileMaterialBounds {
                        material: material.clone(),
                        bounds,
                    }),
                )
            });
        let mut update = TileSetUpdate::default();
        update.extend(iter);
        sender.do_command(SetTileSetTilesCommand {
            tile_set: tile_set.clone(),
            tiles: update,
        });
    }
    fn apply_bounds(
        bounds: &TileBounds,
        state: &TileEditorState,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        let tiles = state.selected_positions();
        let Some(page) = state.page() else {
            return;
        };
        let TileResource::TileSet(tile_set) = tile_resource else {
            return;
        };
        let iter = tiles
            .filter_map(|p| {
                let handle = TileDefinitionHandle::try_new(page, p)?;
                state
                    .tile_set()?
                    .tile_bounds(handle)
                    .map(|b| (handle, b.material.clone()))
            })
            .map(|(handle, material)| {
                (
                    handle,
                    TileDataUpdate::Material(TileMaterialBounds {
                        material,
                        bounds: bounds.clone(),
                    }),
                )
            });
        let mut update = TileSetUpdate::default();
        update.extend(iter);
        sender.do_command(SetTileSetTilesCommand {
            tile_set: tile_set.clone(),
            tiles: update,
        });
    }
    fn apply_transform(
        transformation: OrthoTransformation,
        state: &TileEditorState,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        let Some(page) = state.page() else {
            return;
        };
        let TileResource::TileSet(tile_set) = tile_resource else {
            return;
        };
        let tile_set = tile_set.clone();
        let tiles = state.selected_positions().collect();
        sender.do_command(TransformTilesCommand {
            tile_set,
            page,
            tiles,
            transformation,
        });
    }
}

impl TileEditor for TileMaterialEditor {
    fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    fn draw_button(&self) -> Handle<UiNode> {
        self.draw_button
    }
    fn sync_to_model(&mut self, _state: &TileEditorState, _ui: &mut UserInterface) {}
    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        let material = Self::find_material(state);
        let bounds = Self::find_bounds(state);
        if let Some(material) = &material {
            self.material_bounds.material = material.clone();
        }
        if let Some(bounds) = &bounds {
            self.material_bounds.bounds = bounds.clone();
        }
        send_visibility(
            ui,
            self.material_line,
            material.is_some() && bounds.is_some(),
        );
        if let Some(material) = material {
            send_sync_message(
                ui,
                MaterialFieldMessage::material(
                    self.material_field,
                    MessageDirection::ToWidget,
                    material.clone(),
                ),
            );
        }
        send_visibility(ui, self.bounds_field, Self::bounds_visible(state));
        send_sync_message(
            ui,
            TileBoundsMessage::value(self.bounds_field, MessageDirection::ToWidget, bounds),
        );
    }

    fn draw_tile(
        &self,
        handle: TileDefinitionHandle,
        _subposition: Vector2<usize>,
        _state: &TileDrawState,
        update: &mut TileSetUpdate,
        _tile_resource: &TileResource,
    ) {
        update.set_material(handle.page(), handle.tile(), self.material_bounds.clone());
    }

    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &UiMessage,
        ui: &mut UserInterface,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        if message.flags == MSG_SYNC_FLAG || message.direction() == MessageDirection::ToWidget {
            return;
        }
        if let Some(MaterialFieldMessage::Material(material)) = message.data() {
            if message.destination() == self.material_field {
                self.material_bounds.material = material.clone();
                Self::apply_material(material, state, tile_resource, sender);
            }
        } else if let Some(TileBoundsMessage::Value(Some(bounds))) = message.data() {
            if message.destination() == self.bounds_field {
                self.material_bounds.bounds = bounds.clone();
                Self::apply_bounds(bounds, state, tile_resource, sender);
            }
        } else if let Some(TileBoundsMessage::Turn(amount)) = message.data() {
            Self::apply_transform(
                OrthoTransformation::new(false, *amount),
                state,
                tile_resource,
                sender,
            );
        } else if let Some(TileBoundsMessage::FlipX) = message.data() {
            Self::apply_transform(
                OrthoTransformation::identity().x_flipped(),
                state,
                tile_resource,
                sender,
            );
        } else if let Some(TileBoundsMessage::FlipY) = message.data() {
            Self::apply_transform(
                OrthoTransformation::identity().y_flipped(),
                state,
                tile_resource,
                sender,
            );
        }
    }
}

pub struct TileColorEditor {
    handle: Handle<UiNode>,
    field: Handle<UiNode>,
    draw_button: Handle<UiNode>,
    color: Color,
}

impl TileColorEditor {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let draw_button = make_draw_button("Apply color to tiles", ctx, None);
        let color = Color::default();
        let field = ColorFieldBuilder::new(WidgetBuilder::new().on_column(2))
            .with_color(color)
            .build(ctx);
        Self {
            handle: make_drawable_field("Color", draw_button, field, ctx),
            field,
            draw_button,
            color,
        }
    }
    fn find_color(state: &TileEditorState) -> Option<Color> {
        let mut iter = state.tile_data().map(|(_, d)| d.color);
        let value = iter.next()?;
        if iter.all(|c| c == value) {
            Some(value)
        } else {
            None
        }
    }
    fn apply_color(
        color: Color,
        state: &TileEditorState,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        let TileResource::TileSet(tile_set) = tile_resource else {
            return;
        };
        let iter = state
            .tile_data()
            .map(|(h, _)| (h, TileDataUpdate::Color(color)));
        let mut update = TileSetUpdate::default();
        update.extend(iter);
        sender.do_command(SetTileSetTilesCommand {
            tile_set: tile_set.clone(),
            tiles: update,
        });
    }
}

impl TileEditor for TileColorEditor {
    fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    fn draw_button(&self) -> Handle<UiNode> {
        self.draw_button
    }
    fn sync_to_model(&mut self, _state: &TileEditorState, _ui: &mut UserInterface) {}
    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        let color = Self::find_color(state);
        if let Some(color) = color {
            self.color = color;
        }
        send_visibility(ui, self.handle, color.is_some());
        if let Some(color) = color {
            send_sync_message(
                ui,
                ColorFieldMessage::color(self.field, MessageDirection::ToWidget, color),
            );
        }
    }

    fn draw_tile(
        &self,
        handle: TileDefinitionHandle,
        _subposition: Vector2<usize>,
        _state: &TileDrawState,
        update: &mut TileSetUpdate,
        _tile_resource: &TileResource,
    ) {
        update.set_color(handle.page(), handle.tile(), self.color);
    }

    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &UiMessage,
        ui: &mut UserInterface,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        if message.direction() == MessageDirection::ToWidget || message.flags == MSG_SYNC_FLAG {
            return;
        }
        if let Some(&ColorFieldMessage::Color(color)) = message.data() {
            self.color = color;
            Self::apply_color(color, state, tile_resource, sender);
        }
    }
}

pub struct TileHandleEditor {
    handle: Handle<UiNode>,
    value: Option<TileDefinitionHandle>,
}

impl TileHandleEditor {
    pub fn new(value: Option<TileDefinitionHandle>, ctx: &mut BuildContext) -> Self {
        let handle = TileHandleFieldBuilder::new(WidgetBuilder::new().on_column(1))
            .with_label("Handle")
            .with_value(value)
            .build(ctx);
        Self { handle, value }
    }
    fn find_value(state: &TileEditorState) -> Option<TileDefinitionHandle> {
        let mut iter = state.tile_redirect().map(|(_, v)| v);
        let value = iter.next()?;
        if iter.all(|c| c == value) {
            Some(value)
        } else {
            None
        }
    }
    fn apply_value(
        value: Option<TileDefinitionHandle>,
        state: &TileEditorState,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        match tile_resource {
            TileResource::Empty => (),
            TileResource::TileSet(tile_set) => {
                let iter = state
                    .tile_handles()
                    .map(|h| (h, TileDataUpdate::TransformSet(value)));
                let mut update = TileSetUpdate::default();
                update.extend(iter);
                sender.do_command(SetTileSetTilesCommand {
                    tile_set: tile_set.clone(),
                    tiles: update,
                });
            }
            TileResource::Brush(resource) => {
                if let Some(page) = state.page() {
                    let iter = state.selected_positions().map(|position| (position, value));
                    let mut update = TilesUpdate::default();
                    update.extend(iter);
                    sender.do_command(SetBrushTilesCommand {
                        page,
                        brush: resource.clone(),
                        tiles: update,
                    });
                }
            }
        }
    }
}

impl TileEditor for TileHandleEditor {
    fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    fn draw_button(&self) -> Handle<UiNode> {
        Handle::NONE
    }
    fn sync_to_model(&mut self, _state: &TileEditorState, _ui: &mut UserInterface) {}
    fn sync_to_state(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        let value = Self::find_value(state);
        self.value = value;
        send_visibility(ui, self.handle, state.tile_redirect().next().is_some());
        send_sync_message(
            ui,
            TileHandleEditorMessage::value(self.handle, MessageDirection::ToWidget, value),
        );
    }

    fn draw_tile(
        &self,
        _handle: TileDefinitionHandle,
        _subposition: Vector2<usize>,
        _state: &TileDrawState,
        _update: &mut TileSetUpdate,
        _tile_resource: &TileResource,
    ) {
    }

    fn handle_ui_message(
        &mut self,
        state: &mut TileEditorState,
        message: &UiMessage,
        _ui: &mut UserInterface,
        tile_resource: &TileResource,
        sender: &MessageSender,
    ) {
        if message.direction() == MessageDirection::ToWidget || message.flags == MSG_SYNC_FLAG {
            return;
        }
        if let Some(&TileHandleEditorMessage::Value(value)) = message.data() {
            self.value = value;
            Self::apply_value(value, state, tile_resource, sender);
        }
    }
}
