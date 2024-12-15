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
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    gui::{
        button::{Button, ButtonMessage},
        color::{ColorField, ColorFieldBuilder, ColorFieldMessage},
        decorator::DecoratorMessage,
        expander::ExpanderBuilder,
        grid::{Column, GridBuilder, Row},
        message::UiMessage,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        vec::{Vec2Editor, Vec2EditorBuilder, Vec2EditorMessage, VecEditorBuilder},
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, Orientation, UiNode, UserInterface,
    },
    material::{Material, MaterialResource, MaterialResourceExtension},
    scene::tilemap::{tileset::*, *},
};

use super::*;
use commands::*;
use palette::*;

pub const FIELD_LABEL_WIDTH: f32 = 100.0;

fn make_button(
    title: &str,
    tooltip: &str,
    row: usize,
    column: usize,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .on_column(column)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
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
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius((2.0).into())
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
                .with_width(16.0)
                .with_height(16.0),
        )
        .with_opt_texture(BRUSH_IMAGE.clone())
        .build(ctx),
    )
    .build(ctx)
}

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
    let decorator = *ui.try_get_of_type::<Button>(button).unwrap().decorator;
    ui.send_message(DecoratorMessage::select(
        decorator,
        MessageDirection::ToWidget,
        highlight,
    ));
}

fn send_visibility(ui: &UserInterface, destination: Handle<UiNode>, visible: bool) {
    ui.send_message(WidgetMessage::visibility(
        destination,
        MessageDirection::ToWidget,
        visible,
    ));
}

#[derive(Clone, Default, Debug, Visit, Reflect)]
struct DrawableField {
    handle: Handle<UiNode>,
    field: Handle<UiNode>,
    draw_button: Handle<UiNode>,
}

impl DrawableField {
    fn new(
        label: &str,
        draw_button: Handle<UiNode>,
        field: Handle<UiNode>,
        ctx: &mut BuildContext,
    ) -> Self {
        let label = make_label(label, ctx);
        Self {
            handle: GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(label)
                    .with_child(draw_button)
                    .with_child(field),
            )
            .add_row(Row::auto())
            .add_column(Column::strict(FIELD_LABEL_WIDTH))
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .build(ctx),
            draw_button,
            field,
        }
    }
}

#[derive(Clone, Default, Debug, Visit, Reflect)]
struct InspectorField {
    handle: Handle<UiNode>,
    field: Handle<UiNode>,
}

impl InspectorField {
    fn new(label: &str, field: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let label = make_label(label, ctx);
        Self {
            handle: GridBuilder::new(WidgetBuilder::new().with_child(label).with_child(field))
                .add_row(Row::auto())
                .add_column(Column::strict(FIELD_LABEL_WIDTH))
                .add_column(Column::stretch())
                .build(ctx),
            field,
        }
    }
}

#[derive(Clone, Debug, Visit, Reflect)]
pub struct TileInspector {
    handle: Handle<UiNode>,
    #[reflect(hidden)]
    state: TileDrawStateRef,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    pages_palette: Handle<UiNode>,
    tiles_palette: Handle<UiNode>,
    tile_resource: TileResource,
    tile_set_page_creator: Handle<UiNode>,
    brush_page_creator: Handle<UiNode>,
    tile_size_inspector: InspectorField,
    create_tile: Handle<UiNode>,
    create_page: Handle<UiNode>,
    create_atlas: Handle<UiNode>,
    create_free: Handle<UiNode>,
    create_transform: Handle<UiNode>,
    material_field: DrawableField,
    color_field: DrawableField,
    page_material_inspector: InspectorField,
    page_material_field: Handle<UiNode>,
    bounds_field: Handle<UiNode>,
    page_icon_field: Handle<UiNode>,
    brush_tile_field: Handle<UiNode>,
    colliders: Handle<UiNode>,
    properties: Handle<UiNode>,
}

impl TileInspector {
    pub fn new(
        state: TileDrawStateRef,
        pages_palette: Handle<UiNode>,
        tiles_palette: Handle<UiNode>,
        tile_resource: TileResource,
        _resource_manager: ResourceManager,
        sender: MessageSender,
        ctx: &mut BuildContext,
    ) -> Self {
        let create_page;
        let create_atlas;
        let create_free;
        let create_transform;
        let bounds_field = TileBoundsEditorBuilder::new(WidgetBuilder::new()).build(ctx);
        let color_draw_button = make_draw_button("Apply color to tiles", ctx, None);
        let color_field = ColorFieldBuilder::new(WidgetBuilder::new().on_column(2)).build(ctx);
        let color_field = DrawableField::new("Color", color_draw_button, color_field, ctx);

        let material_draw_button = make_draw_button("Apply material to tiles", ctx, None);
        let material_field = MaterialFieldEditorBuilder::new(WidgetBuilder::new().on_column(2))
            .build(ctx, sender.clone(), DEFAULT_TILE_MATERIAL.deep_copy());
        let material_field =
            DrawableField::new("Material", material_draw_button, material_field, ctx);

        let creator_label_0 = make_label("Create New Page", ctx);
        let creator_label_1 = make_label("Create New Page", ctx);

        let brush_page_creator = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .on_row(1)
                .with_child(creator_label_0)
                .with_child({
                    create_page = make_button("Add Page", "Create a brush tile page.", 0, 0, ctx);
                    create_page
                }),
        )
        .build(ctx);
        let create_tile = make_button("Create Tile", "Add a tile to this page.", 0, 0, ctx);
        let tile_set_page_creator =
            GridBuilder::new(WidgetBuilder::new()
            .with_visibility(false)
            .with_child(creator_label_1)
            .with_child({
                create_atlas =
                    make_button("Tile Atlas", "Create a atlas texture tile page.", 1, 0, ctx);
                create_atlas
            })
            .with_child({
                create_free =
                    make_button("Free Tiles", "Create an arbitrary tile page, with no limits on material and uv coordinates.", 2, 0, ctx);
                create_free
            })
            .with_child({
                create_transform =
                    make_button("Transform", "Create a page that controls how tiles flip and rotate.", 3, 0, ctx);
                create_transform
            })
        ).add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        let page_material_field = MaterialFieldEditorBuilder::new(
            WidgetBuilder::new().on_column(1),
        )
        .build(ctx, sender.clone(), DEFAULT_TILE_MATERIAL.deep_copy());
        let page_material_inspector = InspectorField::new("Material", page_material_field, ctx);
        let tile_size_field =
            Vec2EditorBuilder::<u32>::new(WidgetBuilder::new().on_column(1)).build(ctx);
        let tile_size_inspector = InspectorField::new("Tile Size", tile_size_field, ctx);
        let page_icon_field = TileHandleEditorBuilder::new(WidgetBuilder::new())
            .with_label("Page Icon")
            .build(ctx);
        let brush_tile_field = TileHandleEditorBuilder::new(WidgetBuilder::new())
            .with_label("Handle")
            .build(ctx);
        let properties = ExpanderBuilder::new(WidgetBuilder::new()).build(ctx);
        let colliders = ExpanderBuilder::new(WidgetBuilder::new()).build(ctx);

        let content = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(tile_set_page_creator)
                .with_child(brush_page_creator)
                .with_child(page_icon_field)
                .with_child(page_material_inspector.handle)
                .with_child(material_field.handle)
                .with_child(bounds_field)
                .with_child(tile_size_inspector.handle)
                .with_child(create_tile)
                .with_child(color_field.handle)
                .with_child(brush_tile_field)
                .with_child(properties)
                .with_child(colliders),
        )
        .build(ctx);
        Self {
            handle: content,
            state,
            sender,
            pages_palette,
            tiles_palette,
            tile_resource,
            bounds_field,
            color_field,
            material_field,
            brush_page_creator,
            tile_set_page_creator,
            page_material_inspector,
            page_material_field,
            tile_size_inspector,
            create_tile,
            create_page,
            create_atlas,
            create_free,
            create_transform,
            page_icon_field,
            brush_tile_field,
            properties,
            colliders,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    pub fn page(&self, ui: &UserInterface) -> Option<Vector2<i32>> {
        ui.node(self.tiles_palette)
            .cast::<PaletteWidget>()
            .unwrap()
            .page
    }
    pub fn selected_page_position(&self) -> Option<Vector2<i32>> {
        let state = self.state.lock();
        if state.selection_palette() != self.pages_palette {
            return None;
        }
        let sel = state.selection_positions();
        if sel.len() != 1 {
            return None;
        }
        sel.iter().next().copied()
    }
    pub fn empty_page_selected(&self) -> bool {
        let Some(page) = self.selected_page_position() else {
            return false;
        };
        !self.tile_resource.has_page_at(page)
    }
    pub fn material_page_selected(&self) -> bool {
        let Some(page) = self.selected_page_position() else {
            return false;
        };
        self.tile_resource.is_material_page(page)
    }
    pub fn free_page_selected(&self) -> bool {
        let Some(page) = self.selected_page_position() else {
            return false;
        };
        self.tile_resource.is_free_page(page)
    }
    pub fn transform_page_selected(&self) -> bool {
        let Some(page) = self.selected_page_position() else {
            return false;
        };
        self.tile_resource.is_transform_page(page)
    }
    pub fn brush_page_selected(&self) -> bool {
        let Some(page) = self.selected_page_position() else {
            return false;
        };
        self.tile_resource.is_brush_page(page)
    }
    pub fn any_page_selected(&self) -> bool {
        let Some(page) = self.selected_page_position() else {
            return false;
        };
        self.tile_resource.has_page_at(page)
    }
    pub fn single_tile_selected(&self) -> bool {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return false;
        }
        state.selection_positions().len() == 1
    }
    pub fn empty_tiles_selected(&self, ui: &UserInterface) -> bool {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return false;
        }
        let Some(page) = self.page(ui) else {
            return false;
        };
        if !self.tile_resource.has_page_at(page) {
            return false;
        }
        let sel = state.selection_positions();
        sel.iter()
            .any(|tile| !self.tile_resource.has_tile_at(page, *tile))
    }
    pub fn material_tiles_selected(&self, ui: &UserInterface) -> bool {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return false;
        }
        let Some(page) = self.page(ui) else {
            return false;
        };
        if !self.tile_resource.is_material_page(page) {
            return false;
        }
        let sel = state.selection_positions();
        sel.iter()
            .any(|tile| self.tile_resource.has_tile_at(page, *tile))
    }
    pub fn free_tiles_selected(&self, ui: &UserInterface) -> bool {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return false;
        }
        let Some(page) = self.page(ui) else {
            return false;
        };
        if !self.tile_resource.is_free_page(page) {
            return false;
        }
        let sel = state.selection_positions();
        sel.iter()
            .any(|tile| self.tile_resource.has_tile_at(page, *tile))
    }
    pub fn transform_tiles_selected(&self, ui: &UserInterface) -> bool {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return false;
        }
        let Some(page) = self.page(ui) else {
            return false;
        };
        if !self.tile_resource.is_transform_page(page) {
            return false;
        }
        let sel = state.selection_positions();
        sel.iter()
            .any(|tile| self.tile_resource.has_tile_at(page, *tile))
    }
    pub fn brush_tiles_selected(&self, ui: &UserInterface) -> bool {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return false;
        }
        let Some(page) = self.page(ui) else {
            return false;
        };
        if !self.tile_resource.is_brush_page(page) {
            return false;
        }
        let sel = state.selection_positions();
        sel.iter()
            .any(|tile| self.tile_resource.has_tile_at(page, *tile))
    }
    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        self.sync_to_state(ui);
    }
    pub fn sync_to_state(&mut self, ui: &mut UserInterface) {
        let single_tile = self.single_tile_selected();
        let empty_page = self.empty_page_selected();
        let free_tiles = self.free_tiles_selected(ui);
        let transform_tiles = self.transform_tiles_selected(ui);
        let brush_tiles = self.brush_tiles_selected(ui);
        let material_page = self.material_page_selected();
        let any_page = self.any_page_selected();
        let tile_data_selected = self.material_tiles_selected(ui) || free_tiles;
        let show_tile_mat = free_tiles && single_tile;
        send_visibility(
            ui,
            self.tile_set_page_creator,
            empty_page && self.tile_resource.is_tile_set(),
        );
        send_visibility(
            ui,
            self.brush_page_creator,
            empty_page && self.tile_resource.is_brush(),
        );
        send_visibility(ui, self.create_tile, self.empty_tiles_selected(ui));
        send_visibility(ui, self.tile_size_inspector.handle, material_page);
        send_visibility(ui, self.page_material_inspector.handle, material_page);
        send_visibility(ui, self.page_icon_field, any_page);
        send_visibility(ui, self.material_field.handle, show_tile_mat);
        send_visibility(ui, self.bounds_field, free_tiles);
        send_visibility(ui, self.color_field.handle, tile_data_selected);
        send_visibility(ui, self.brush_tile_field, transform_tiles || brush_tiles);
        send_visibility(ui, self.properties, tile_data_selected);
        send_visibility(ui, self.colliders, tile_data_selected);
        if let Some(position) = self.selected_page_position() {
            self.sync_to_page(position, ui);
        }
        let state = self.state.lock();
        highlight_tool_button(
            self.material_field.draw_button,
            state.drawing_mode == DrawingMode::Material,
            ui,
        );
        highlight_tool_button(
            self.color_field.draw_button,
            state.drawing_mode == DrawingMode::Color,
            ui,
        );
        highlight_tool_button(
            self.material_field.draw_button,
            state.drawing_mode == DrawingMode::Material,
            ui,
        );
        drop(state);
        let page_icon = self.find_page_icon(ui);
        send_sync_message(
            ui,
            TileHandleEditorMessage::value(
                self.page_icon_field,
                MessageDirection::ToWidget,
                page_icon,
            ),
        );
        if let Some(color) = self.find_color(ui) {
            send_sync_message(
                ui,
                ColorFieldMessage::color(self.color_field.field, MessageDirection::ToWidget, color),
            );
        }
        if show_tile_mat {
            self.sync_to_free_tile_material(ui);
        }
        if free_tiles {
            self.sync_to_bounds(ui);
        }
        // TODO
    }
    fn find_color(&self, ui: &mut UserInterface) -> Option<Color> {
        let state = self.state.lock();
        let handle = *state.selection_tiles().values().next()?;
        if let TileResource::TileSet(tile_set) = &self.tile_resource {
            tile_set.state().data()?.tile_color(handle)
        } else {
            None
        }
    }
    fn find_material(&self, ui: &mut UserInterface) -> Option<TileMaterialBounds> {
        let state = self.state.lock();
        let tiles = state.selection_tiles();
        let handle = tiles.values().next()?;
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return None;
        };
        tile_set
            .data_ref()
            .get_tile_bounds(TilePaletteStage::Tiles, *handle)
    }
    fn find_page_icon(&self, ui: &mut UserInterface) -> Option<TileDefinitionHandle> {
        let page = self.selected_page_position()?;
        self.tile_resource.page_icon(page)
    }
    fn sync_to_free_tile_material(&self, ui: &mut UserInterface) {
        let Some(material) = self.find_material(ui).map(|m| m.material) else {
            return;
        };
        send_sync_message(
            ui,
            MaterialFieldMessage::material(
                self.material_field.field,
                MessageDirection::ToWidget,
                material.clone(),
            ),
        );
    }
    fn get_bounds(&self) -> Option<TileBounds> {
        let state = self.state.lock();
        let tiles = state.selection_tiles();
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return None;
        };
        let mut iter = tiles.values().filter_map(|h| {
            tile_set
                .data_ref()
                .tile_bounds(*h)
                .map(|b| b.bounds.clone())
        });
        let value = iter.next()?;
        if iter.all(|b| b == value) {
            Some(value)
        } else {
            None
        }
    }
    fn sync_to_bounds(&self, ui: &mut UserInterface) {
        let bounds = self.get_bounds();
        send_sync_message(
            ui,
            TileBoundsMessage::value(self.bounds_field, MessageDirection::ToWidget, bounds),
        );
    }
    fn sync_to_page(&mut self, position: Vector2<i32>, ui: &mut UserInterface) {
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let tile_set = tile_set.data_ref();
        let Some(page) = tile_set.pages.get(&position) else {
            return;
        };
        send_sync_message(
            ui,
            TileHandleEditorMessage::value(
                self.page_icon_field,
                MessageDirection::ToWidget,
                page.icon,
            ),
        );
        if let TileSetPageSource::Material(mat) = &page.source {
            send_sync_message(
                ui,
                Vec2EditorMessage::value(
                    self.tile_size_inspector.field,
                    MessageDirection::ToWidget,
                    mat.tile_size,
                ),
            );
            send_sync_message(
                ui,
                MaterialFieldMessage::material(
                    self.page_material_inspector.field,
                    MessageDirection::ToWidget,
                    mat.material.clone(),
                ),
            );
        }
    }
    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if message.flags == MSG_SYNC_FLAG || message.direction() == MessageDirection::ToWidget {
            return;
        }
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create_atlas {
                self.create_tile_set_page(TileSetPageSource::new_material(), sender);
            } else if message.destination() == self.create_free {
                self.create_tile_set_page(TileSetPageSource::new_free(), sender);
            } else if message.destination() == self.create_transform {
                self.create_tile_set_page(TileSetPageSource::new_transform(), sender);
            } else if message.destination() == self.create_tile {
                self.create_tile(ui, sender);
            } else if message.destination() == self.color_field.draw_button {
                let color = self.find_color(ui);
                let mut state = self.state.lock_mut();
                if let Some(color) = color {
                    state.draw_value = DrawValue::Color(color);
                }
                state.drawing_mode = match state.drawing_mode {
                    DrawingMode::Color => DrawingMode::Pick,
                    _ => DrawingMode::Color,
                };
            } else if message.destination() == self.material_field.draw_button {
                let material = self.find_material(ui);
                let mut state = self.state.lock_mut();
                if let Some(material) = material {
                    state.draw_value = DrawValue::Material(material);
                }
                state.drawing_mode = match state.drawing_mode {
                    DrawingMode::Material => DrawingMode::Pick,
                    _ => DrawingMode::Material,
                };
            }
        } else if let Some(MaterialFieldMessage::Material(material)) = message.data() {
            if message.destination() == self.page_material_inspector.field {
                self.set_page_material(material.clone(), sender);
            } else if message.destination() == self.material_field.field {
                self.apply_tile_material(material.clone(), ui);
            }
        } else if let Some(Vec2EditorMessage::<u32>::Value(size)) = message.data() {
            if message.destination() == self.tile_size_inspector.field {
                self.set_page_tile_size(*size, sender);
            }
        } else if let Some(TileBoundsMessage::Value(Some(v))) = message.data() {
            self.apply_tile_bounds(v, ui);
        } else if let Some(TileBoundsMessage::Turn(amount)) = message.data() {
            self.apply_tile_transform(OrthoTransformation::new(false, *amount), ui);
        } else if let Some(TileBoundsMessage::FlipX) = message.data() {
            self.apply_tile_transform(OrthoTransformation::identity().x_flipped(), ui);
        } else if let Some(TileBoundsMessage::FlipY) = message.data() {
            self.apply_tile_transform(OrthoTransformation::identity().y_flipped(), ui);
        } else if let Some(TileHandleEditorMessage::Value(Some(handle))) = message.data() {
            if message.destination() == self.page_icon_field {
                self.apply_page_icon(*handle, ui);
            }
        } else if let Some(TileHandleEditorMessage::Goto) = message.data() {
            let editor: &TileHandleEditor = ui.node(message.destination()).cast().unwrap();
            if let Some(value) = editor.value() {
                self.goto_tile(value, ui);
            }
        } else if let Some(TileHandleEditorMessage::OpenPalette) = message.data() {
            let editor: &TileHandleEditor = ui.node(message.destination()).cast().unwrap();
            if let Some(value) = editor.value() {
                self.show_tile_in_palette(value, ui);
            }
        } else if let Some(ColorFieldMessage::Color(color)) = message.data() {
            self.apply_color(*color);
        }
        // TODO
    }
    fn apply_color(&self, color: Color) {
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let mut tiles = TileSetUpdate::default();
        for handle in self.state.lock().selection_tiles().values() {
            tiles.set_color(handle.page(), handle.tile(), color);
        }
        self.sender.do_command(SetTileSetTilesCommand {
            tile_set: tile_set.clone(),
            tiles,
        });
    }
    fn goto_tile(&self, handle: TileDefinitionHandle, ui: &mut UserInterface) {
        // TODO
    }
    fn show_tile_in_palette(&self, handle: TileDefinitionHandle, ui: &mut UserInterface) {
        // TODO
    }
    fn apply_page_icon(&self, icon: TileDefinitionHandle, ui: &mut UserInterface) {
        let state = self.state.lock();
        if state.selection_palette() != self.pages_palette {
            return;
        }
        let Some(page) = self.page(ui) else {
            return;
        };
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        self.sender.do_command(ModifyPageIconCommand {
            tile_set: tile_set.clone(),
            page,
            icon: Some(icon),
        });
    }
    fn apply_tile_material(&self, material: MaterialResource, ui: &UserInterface) {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return;
        }
        let Some(page) = self.page(ui) else {
            return;
        };
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let tile_set = tile_set.clone();
        let tile_set_data = tile_set.data_ref();
        let sel = state.selection_positions();
        let Some(original_bounds) = (match tile_set_data.pages.get(&page).map(|p| &p.source) {
            Some(TileSetPageSource::Freeform(map)) => sel
                .iter()
                .filter_map(|p| map.get(p))
                .map(|def| &def.material_bounds)
                .next(),
            _ => None,
        }) else {
            return;
        };
        let tile_update = TileDataUpdate::Material(TileMaterialBounds {
            material: material.clone(),
            bounds: original_bounds.bounds.clone(),
        });
        drop(tile_set_data);
        let mut update = TileSetUpdate::default();
        for p in sel.iter() {
            let Some(handle) = TileDefinitionHandle::try_new(page, *p) else {
                continue;
            };
            let _ = update.insert(handle, tile_update.clone());
        }
        self.sender.do_command(SetTileSetTilesCommand {
            tile_set,
            tiles: update,
        });
    }
    fn apply_tile_bounds(&self, new_bounds: &TileBounds, ui: &UserInterface) {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return;
        }
        let Some(page) = self.page(ui) else {
            return;
        };
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let tile_set = tile_set.clone();
        let tile_set_data = tile_set.data_ref();
        let sel = state.selection_positions();
        let Some(original_bounds) = (match tile_set_data.pages.get(&page).map(|p| &p.source) {
            Some(TileSetPageSource::Freeform(map)) => sel
                .iter()
                .filter_map(|p| map.get(p))
                .map(|def| &def.material_bounds)
                .next(),
            _ => None,
        }) else {
            return;
        };
        let tile_update = TileDataUpdate::Material(TileMaterialBounds {
            material: original_bounds.material.clone(),
            bounds: new_bounds.clone(),
        });
        drop(tile_set_data);
        let mut update = TileSetUpdate::default();
        for p in sel.iter() {
            let Some(handle) = TileDefinitionHandle::try_new(page, *p) else {
                continue;
            };
            let _ = update.insert(handle, tile_update.clone());
        }
        self.sender.do_command(SetTileSetTilesCommand {
            tile_set,
            tiles: update,
        });
    }
    fn apply_tile_transform(&self, transformation: OrthoTransformation, ui: &UserInterface) {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return;
        }
        let Some(page) = self.page(ui) else {
            return;
        };
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let tile_set = tile_set.clone();
        let sel = state.selection_positions();
        self.sender.do_command(TransformTilesCommand {
            tile_set,
            page,
            tiles: sel.iter().copied().collect(),
            transformation,
        });
    }
    fn create_tile(&self, ui: &UserInterface, sender: &MessageSender) {
        let state = self.state.lock();
        if state.selection_palette() != self.tiles_palette {
            return;
        }
        let Some(page) = self.page(ui) else {
            return;
        };
        let TileResource::TileSet(tile_set) = &self.tile_resource else {
            return;
        };
        let tile_set = tile_set.clone();
        let tile_set_data = tile_set.data_ref();
        let data = match tile_set_data.pages.get(&page).map(|p| &p.source) {
            Some(TileSetPageSource::Material(_)) => {
                TileDataUpdate::MaterialTile(TileData::default())
            }
            Some(TileSetPageSource::Freeform(_)) => {
                TileDataUpdate::FreeformTile(TileDefinition::default())
            }
            _ => return,
        };
        let sel = state.selection_positions();
        let empties = sel
            .iter()
            .filter(|tile| !tile_set_data.has_tile_at(page, **tile));
        let mut update = TileSetUpdate::default();
        for p in empties {
            let Some(h) = TileDefinitionHandle::try_new(page, *p) else {
                continue;
            };
            update.insert(h, data.clone());
        }
        drop(tile_set_data);
        sender.do_command(SetTileSetTilesCommand {
            tile_set,
            tiles: update,
        });
    }
    fn create_tile_set_page(&self, source: TileSetPageSource, sender: &MessageSender) {
        let Some(position) = self.selected_page_position() else {
            return;
        };
        let TileResource::TileSet(tile_set) = self.tile_resource.clone() else {
            return;
        };
        let page = TileSetPage { icon: None, source };
        sender.do_command(SetTileSetPageCommand {
            tile_set,
            position,
            page: Some(page),
        });
    }
    fn set_page_material(&self, material: MaterialResource, sender: &MessageSender) {
        let TileResource::TileSet(tile_set) = self.tile_resource.clone() else {
            return;
        };
        let Some(page) = self.selected_page_position() else {
            return;
        };
        if !matches!(
            tile_set.data_ref().pages.get(&page).map(|p| &p.source),
            Some(TileSetPageSource::Material(_))
        ) {
            return;
        }
        sender.do_command(ModifyPageMaterialCommand {
            tile_set,
            page,
            material,
        });
    }
    fn set_page_tile_size(&self, size: Vector2<u32>, sender: &MessageSender) {
        let TileResource::TileSet(tile_set) = self.tile_resource.clone() else {
            return;
        };
        let Some(page) = self.selected_page_position() else {
            return;
        };
        if !matches!(
            tile_set.data_ref().pages.get(&page).map(|p| &p.source),
            Some(TileSetPageSource::Material(_))
        ) {
            return;
        }
        sender.do_command(ModifyPageTileSizeCommand {
            tile_set,
            page,
            size,
        });
    }
}
