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

use fyrox::fxhash::FxHashSet;
use fyrox::scene::tilemap::tileset::{TileSetPageSource, TileSetPropertyValue};
use fyrox::scene::tilemap::TileSource;

use crate::asset::item::AssetItem;
use crate::fyrox::{
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        math::Rect,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    fxhash::FxHashMap,
    graph::BaseSceneGraph,
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        formatted_text::{FormattedText, FormattedTextBuilder},
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
    material::{Material, MaterialResource},
    resource::texture::TextureKind,
    scene::tilemap::{
        TilePaletteStage, TileRect, TileRenderData, TileResource, TileSetUpdate, TransTilesUpdate,
    },
};
use std::ops::{Deref, DerefMut};

use super::{commands::*, *};

pub const DEFAULT_MATERIAL_COLOR: Color = Color::from_rgba(255, 255, 255, 125);

#[derive(Debug, PartialEq, Clone)]
pub enum PaletteMessage {
    SetPage {
        source: TileResource,
        page: Option<Vector2<i32>>,
    },
    Center(Vector2<i32>),
    SelectAll,
    Delete,
    MaterialColor(Color),
    SyncToState,
}

impl PaletteMessage {
    define_constructor!(PaletteMessage:SetPage => fn set_page(source: TileResource, page: Option<Vector2<i32>>), layout: false);
    define_constructor!(PaletteMessage:Center => fn center(Vector2<i32>), layout: false);
    define_constructor!(PaletteMessage:SelectAll => fn select_all(), layout: false);
    define_constructor!(PaletteMessage:Delete => fn delete(), layout: false);
    define_constructor!(PaletteMessage:MaterialColor => fn material_color(Color), layout: false);
    define_constructor!(PaletteMessage:SyncToState => fn sync_to_state(), layout: false);
}

#[derive(Clone, Default, Debug, PartialEq)]
enum MouseMode {
    #[default]
    None,
    Panning {
        initial_view_position: Vector2<f32>,
        click_position: Vector2<f32>,
    },
    Dragging {
        initial_position: Vector2<f32>,
        offset: Vector2<i32>,
    },
    Drawing {
        start_tile: Vector2<i32>,
        end: MousePos,
    },
}

#[derive(Clone, Default, Debug, PartialEq)]
struct MousePos {
    fine: Vector2<f32>,
    grid: Vector2<i32>,
    subgrid: Vector2<usize>,
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Visit, Reflect)]
pub enum PaletteKind {
    #[default]
    Tiles,
    Pages,
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Hash)]
pub struct Subposition {
    pub tile: Vector2<i32>,
    pub subtile: Vector2<usize>,
}

fn calc_slice_coord(position: f32, step: f32) -> usize {
    let p = position / step;
    let p = (p - p.floor()) * 3.0;
    (p.floor() as i32).clamp(0, 2) as usize
}

#[derive(Clone, Debug, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5356a864-c026-4bd7-a4b1-30bacf77d8fa")]
pub struct PaletteWidget {
    widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    pub content: TileResource,
    pub page: Option<Vector2<i32>>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub update: TransTilesUpdate,
    #[visit(skip)]
    #[reflect(hidden)]
    pub tile_set_update: TileSetUpdate,
    #[reflect(hidden)]
    pub state: TileDrawStateRef,
    pub kind: TilePaletteStage,
    pub editable: bool,
    pub slice_mode: bool,
    material_color: Color,
    #[visit(skip)]
    #[reflect(hidden)]
    selecting_tiles: FxHashSet<Vector2<i32>>,
    #[visit(skip)]
    #[reflect(hidden)]
    highlight: FxHashMap<Subposition, Color>,
    #[visit(skip)]
    #[reflect(hidden)]
    cursor_position: Option<Vector2<i32>>,
    #[visit(skip)]
    #[reflect(hidden)]
    slice_position: Vector2<usize>,
    #[visit(skip)]
    #[reflect(hidden)]
    position_text: FormattedText,
    #[visit(skip)]
    #[reflect(hidden)]
    overlay: PaletteOverlay,
    view_position: Vector2<f32>,
    zoom: f32,
    tile_size: Vector2<f32>,
    #[visit(skip)]
    #[reflect(hidden)]
    mode: MouseMode,
}

define_widget_deref!(PaletteWidget);

#[derive(Default, Clone, Debug)]
struct PaletteOverlay {
    active: bool,
    movable_position: Vector2<i32>,
    movable_tiles: FxHashMap<Vector2<i32>, TileRenderData>,
    erased_tiles: FxHashSet<Vector2<i32>>,
}

impl PaletteOverlay {
    pub fn covers(&self, position: Vector2<i32>) -> bool {
        self.active
            && (self.erased_tiles.contains(&position)
                || self
                    .movable_tiles
                    .contains_key(&(position - self.movable_position)))
    }
    pub fn iter(&self) -> impl Iterator<Item = (Vector2<i32>, &TileRenderData)> {
        let offset = self.movable_position;
        self.movable_tiles
            .iter()
            .filter(|_| self.active)
            .map(move |(p, d)| (*p + offset, d))
    }
    pub fn clear(&mut self) {
        self.movable_tiles.clear();
        self.erased_tiles.clear();
    }
    pub fn set_to_stamp(&mut self, stamp: &Stamp, tile_set: &TileSet) {
        self.movable_tiles.clear();
        self.erased_tiles.clear();
        for (pos, handle) in stamp.iter() {
            let data = tile_set
                .get_transformed_render_data(stamp.transformation(), *handle)
                .unwrap_or_else(TileRenderData::missing_data);
            let _ = self.movable_tiles.insert(pos, data);
        }
    }
}

fn apply_transform(trans: &Matrix3<f32>, point: Vector2<f32>) -> Vector2<f32> {
    trans.transform_point(&Point2::from(point)).coords
}

fn invert_transform(trans: &Matrix3<f32>) -> Matrix3<f32> {
    trans.try_inverse().unwrap_or(Matrix3::identity())
}

impl PaletteWidget {
    pub fn stage(&self) -> TilePaletteStage {
        match &self.kind {
            TilePaletteStage::Pages => TilePaletteStage::Pages,
            _ => TilePaletteStage::Tiles,
        }
    }

    fn sync_to_state(&mut self) {
        let state = self.state.lock();
        if state.selection_palette() != self.handle {
            self.selecting_tiles.clear();
        }
        self.slice_mode = state.drawing_mode == DrawingMode::Property
            && matches!(state.draw_value, DrawValue::I8(_));
        if self.editable && self.kind == TilePaletteStage::Tiles {
            if state.drawing_mode == DrawingMode::Draw
                || state.drawing_mode == DrawingMode::FloodFill
            {
                if let Some(tile_set) = self.content.get_tile_set() {
                    self.overlay
                        .set_to_stamp(&state.stamp, &tile_set.data_ref());
                } else {
                    self.overlay.clear();
                }
            } else {
                self.overlay.clear();
            }
        }
    }

    pub fn screen_point_to_tile_point(&self, point: Vector2<f32>) -> Vector2<f32> {
        let trans = self.visual_transform() * self.tile_to_local();
        let trans = invert_transform(&trans);
        apply_transform(&trans, point)
    }

    pub fn tile_point_to_screen_point(&self, point: Vector2<f32>) -> Vector2<f32> {
        let trans = self.visual_transform() * self.tile_to_local();
        apply_transform(&trans, point)
    }

    fn tile_to_local(&self) -> Matrix3<f32> {
        let translation = self.actual_local_size.get() * 0.5;
        Matrix3::new_translation(&self.view_position)
            * Matrix3::new_translation(&translation)
            * Matrix3::new_nonuniform_scaling(&Vector2::new(self.zoom, -self.zoom))
    }

    fn calc_mouse_position(&self, screen_point: Vector2<f32>) -> MousePos {
        let tile_point = self.screen_point_to_tile_point(screen_point);
        MousePos {
            fine: screen_point,
            grid: self.tile_point_to_grid_pos(tile_point),
            subgrid: if self.slice_mode {
                Vector2::new(
                    calc_slice_coord(tile_point.x, self.tile_size.x),
                    calc_slice_coord(tile_point.y, self.tile_size.y),
                )
            } else {
                Vector2::default()
            },
        }
    }

    fn set_cursor_position(&mut self, pos: Option<Vector2<i32>>) {
        if self.cursor_position == pos {
            return;
        }
        self.cursor_position = pos;
        let text = if let Some(pos) = pos {
            format!("{}, {}", pos.x, pos.y)
        } else {
            "".into()
        };
        self.position_text.set_text(text);
        self.position_text.build();
    }

    fn tile_point_to_grid_pos(&self, pos: Vector2<f32>) -> Vector2<i32> {
        let s = self.tile_size;
        Vector2::new((pos.x / s.x).floor() as i32, (pos.y / s.y).floor() as i32)
    }
    fn send_update(&mut self) {
        if self.kind != TilePaletteStage::Tiles {
            panic!();
        }
        let Some(page) = self.page else {
            return;
        };
        match &self.content {
            TileResource::Empty => (),
            TileResource::TileSet(resource) => {
                self.tile_set_update.clear();
                self.tile_set_update
                    .convert(&self.update, &resource.data_ref(), page);
                self.sender.do_command(SetTileSetTilesCommand {
                    tile_set: resource.clone(),
                    tiles: self.tile_set_update.clone(),
                });
                self.tile_set_update.clear();
                self.update.clear();
            }
            TileResource::Brush(resource) => {
                if let Some(tile_set) = &resource.data_ref().tile_set {
                    self.sender.do_command(SetBrushTilesCommand {
                        brush: resource.clone(),
                        page,
                        tiles: self.update.build_tiles_update(&tile_set.data_ref()),
                    });
                }
            }
        }
    }
    fn send_tile_set_update(&mut self) {
        if self.kind != TilePaletteStage::Tiles {
            panic!();
        }
        if let TileResource::TileSet(resource) = &self.content {
            self.sender.do_command(SetTileSetTilesCommand {
                tile_set: resource.clone(),
                tiles: self.tile_set_update.clone(),
            });
            self.tile_set_update.clear();
        }
    }
    fn delete_tiles(&mut self, ui: &mut UserInterface) -> bool {
        let state = self.state.lock_mut();
        if state.selection_palette() != self.handle || !state.has_selection() {
            return false;
        }
        for position in state.selection_positions() {
            self.update.insert(*position, None);
        }
        drop(state);
        self.send_update();
        true
    }
    fn set_page(
        &mut self,
        resource: TileResource,
        page: Option<Vector2<i32>>,
        _ui: &mut UserInterface,
    ) {
        let mut state = self.state.lock_mut();
        if state.selection_palette() == self.handle {
            self.selecting_tiles.clear();
            state.clear_selection();
        }
        self.page = page;
        self.content = resource;
    }
    fn send_new_page(&mut self, page: Vector2<i32>, ui: &mut UserInterface) {
        self.page = Some(page);
        ui.send_message(PaletteMessage::set_page(
            self.handle,
            MessageDirection::FromWidget,
            self.content.clone(),
            Some(page),
        ));
    }
    fn drawing_mode(&self) -> Option<DrawingMode> {
        if self.editable {
            match self.kind {
                TilePaletteStage::Pages => Some(DrawingMode::Pick),
                TilePaletteStage::Tiles => Some(self.state.lock().drawing_mode),
            }
        } else {
            Some(DrawingMode::Pick)
        }
    }
    pub fn sync_selection_to_model(&mut self) {
        let page = self.page.unwrap_or_default();
        let mut state = self.state.lock_mut();
        self.selecting_tiles.clone_from(state.selection_positions());
        let tiles = state.selection_tiles_mut();
        self.content.get_tiles(
            self.stage(),
            page,
            self.selecting_tiles.iter().copied(),
            tiles,
        );
        self.selecting_tiles.clear();
    }
    fn update_selection(&mut self) {
        let MouseMode::Drawing { start_tile, end } = self.mode.clone() else {
            return;
        };
        let end_tile = end.grid;
        if self.kind == TilePaletteStage::Tiles && self.page.is_none() {
            return;
        }
        let page = self.page.unwrap_or_default();
        let mut state = self.state.lock_mut();
        state.set_palette(self.handle);
        let positions = state.selection_positions_mut();
        positions.clone_from(&self.selecting_tiles);
        positions.extend(TileRect::from_points(start_tile, end_tile).iter());
        let tiles = state.selection_tiles_mut();
        tiles.clear();
        self.content.get_tiles(
            self.stage(),
            page,
            self.selecting_tiles.iter().copied(),
            tiles,
        );
        let rect = TileRect::from_points(start_tile, end_tile);
        self.content
            .get_tiles(self.stage(), page, rect.iter(), tiles);
        state.update_stamp(self.content.get_tile_set());
    }
    fn finalize_selection(&mut self, ui: &mut UserInterface) {
        let MouseMode::Drawing { start_tile, end } = self.mode.clone() else {
            return;
        };
        let end_tile = end.grid;
        match self.kind {
            TilePaletteStage::Tiles => {
                if self.page.is_none() {
                    return;
                }
            }
            TilePaletteStage::Pages => self.send_new_page(end_tile, ui),
            _ => (),
        }
        let page = self.page.unwrap_or_default();
        self.selecting_tiles
            .extend(TileRect::from_points(start_tile, end_tile).iter());
        let mut state = self.state.lock_mut();
        state.set_palette(self.handle);
        let positions = state.selection_positions_mut();
        positions.clone_from(&self.selecting_tiles);
        let tiles = state.selection_tiles_mut();
        tiles.clear();
        self.content.get_tiles(
            self.stage(),
            page,
            self.selecting_tiles.iter().copied(),
            tiles,
        );
        state.update_stamp(self.content.get_tile_set());
    }
    fn select_all(&mut self) {
        let Some(page) = self.page else {
            return;
        };
        let mut state = self.state.lock_mut();
        let results = match self.stage() {
            TilePaletteStage::Tiles => self.content.get_all_tile_positions(page),
            TilePaletteStage::Pages => self.content.get_all_page_positions(),
        };
        state.set_palette(self.handle);
        let tiles = state.selection_tiles_mut();
        tiles.clear();
        self.content
            .get_tiles(self.stage(), page, results.into_iter(), tiles);
        state.update_stamp(self.content.get_tile_set());
    }
    fn begin_motion(&mut self, mode: DrawingMode, pos: MousePos, ui: &mut UserInterface) {
        match mode {
            DrawingMode::Pick => {
                if self.editable && ui.keyboard_modifiers().alt {
                    self.mode = MouseMode::Dragging {
                        initial_position: pos.fine,
                        offset: Vector2::new(0, 0),
                    };
                    self.begin_drag(pos, ui);
                } else {
                    let start_tile = pos.grid;
                    self.mode = MouseMode::Drawing {
                        start_tile,
                        end: pos,
                    };
                    if !ui.keyboard_modifiers().shift {
                        self.selecting_tiles.clear();
                    }
                    self.update_selection();
                }
            }
            _ => {
                let start_tile = pos.grid;
                self.mode = MouseMode::Drawing {
                    start_tile,
                    end: pos.clone(),
                };
                self.draw(mode, start_tile, start_tile, pos.subgrid, ui);
            }
        }
    }

    fn drag_offset(&self, drag_vector: Vector2<f32>) -> Vector2<i32> {
        let t = self.tile_size;
        let p = drag_vector / self.zoom;
        Vector2::new((p.x / t.x).round() as i32, -(p.y / t.y).round() as i32)
    }
    fn continue_motion(&mut self, mode: DrawingMode, pos: MousePos, ui: &mut UserInterface) {
        match mode {
            DrawingMode::Pick => match &self.mode {
                MouseMode::Dragging {
                    initial_position, ..
                } => {
                    let offset = self.drag_offset(pos.fine - *initial_position);
                    self.mode = MouseMode::Dragging {
                        initial_position: *initial_position,
                        offset,
                    };
                    self.overlay.movable_position = offset;
                }
                MouseMode::Drawing { start_tile, end } => {
                    if end.grid != pos.grid {
                        self.mode = MouseMode::Drawing {
                            start_tile: *start_tile,
                            end: pos,
                        };
                        self.update_selection();
                    }
                }
                _ => (),
            },
            mode => match self.mode.clone() {
                MouseMode::None => {
                    if mode == DrawingMode::Draw || mode == DrawingMode::FloodFill {
                        self.overlay.movable_position = pos.grid;
                    }
                }
                MouseMode::Drawing { start_tile, end } => {
                    if end.grid != pos.grid || self.slice_mode && end.subgrid != pos.subgrid {
                        self.draw(mode, start_tile, pos.grid, pos.subgrid, ui);
                        self.mode = MouseMode::Drawing {
                            start_tile,
                            end: pos,
                        };
                    }
                }
                _ => (),
            },
        }
    }

    fn end_motion(&mut self, mode: DrawingMode, pos: MousePos, ui: &mut UserInterface) {
        match mode {
            DrawingMode::Pick => match &self.mode {
                MouseMode::Dragging {
                    initial_position, ..
                } => {
                    let offset = self.drag_offset(pos.fine - *initial_position);
                    self.mode = MouseMode::None;
                    self.end_drag(offset);
                }
                MouseMode::Drawing { start_tile, .. } => {
                    self.mode = MouseMode::Drawing {
                        start_tile: *start_tile,
                        end: pos,
                    };
                    self.finalize_selection(ui);
                }
                _ => (),
            },
            _ => {
                if let MouseMode::Drawing { start_tile, .. } = self.mode.clone() {
                    let end_tile = pos.grid;
                    self.mode = MouseMode::None;
                    self.end_draw(mode, start_tile, end_tile, ui);
                }
            }
        }
    }

    fn begin_drag(&mut self, pos: MousePos, ui: &mut UserInterface) {
        let state = self.state.lock();
        if state.selection_palette() != self.handle {
            return;
        }
        let Some(page) = self.page else {
            return;
        };
        if self.kind == TilePaletteStage::Tiles && self.content.is_material_page(page) {
            return;
        }
        let Some(tile_set) = self.content.get_tile_set() else {
            return;
        };
        let mut tile_set = tile_set.state();
        let Some(tile_set) = tile_set.data() else {
            return;
        };
        let tiles = state.selection_positions();
        self.overlay.movable_position = Vector2::default();
        self.overlay.erased_tiles.clear();
        self.overlay.movable_tiles.clear();
        for pos in tiles.iter() {
            let Some(handle) = TileDefinitionHandle::try_new(page, *pos) else {
                continue;
            };
            let Some(data) = tile_set.get_tile_render_data(self.kind, handle) else {
                continue;
            };
            let _ = self.overlay.erased_tiles.insert(*pos);
            let _ = self.overlay.movable_tiles.insert(*pos, data);
        }
    }

    fn end_drag(&mut self, offset: Vector2<i32>) {
        match self.kind {
            TilePaletteStage::Pages => self.end_page_drag(offset),
            TilePaletteStage::Tiles => self.end_tile_drag(offset),
        }
        let mut state = self.state.lock_mut();
        let sel = state.selection_positions_mut();
        sel.clear();
        sel.extend(self.overlay.iter().map(|(p, _)| p));
        self.overlay.erased_tiles.clear();
        self.overlay.movable_tiles.clear();
    }

    fn end_page_drag(&mut self, offset: Vector2<i32>) {
        let state = self.state.lock();
        if state.selection_palette() != self.handle {
            return;
        }
        let pages = state
            .selection_positions()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        match self.content.clone() {
            TileResource::Empty => (),
            TileResource::TileSet(tile_set) => {
                self.sender
                    .do_command(MoveTileSetPageCommand::new(tile_set, pages, offset));
            }
            TileResource::Brush(brush) => {
                self.sender
                    .do_command(MoveBrushPageCommand::new(brush, pages, offset));
            }
        }
    }
    fn end_tile_drag(&mut self, offset: Vector2<i32>) {
        let state = self.state.lock();
        if state.selection_palette() != self.handle {
            return;
        }
        let Some(page) = self.page else {
            return;
        };
        let tiles = state
            .selection_positions()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        match self.content.clone() {
            TileResource::Empty => (),
            TileResource::TileSet(tile_set) => {
                self.sender
                    .do_command(MoveTileSetTileCommand::new(tile_set, page, tiles, offset));
            }
            TileResource::Brush(brush) => {
                self.sender
                    .do_command(MoveBrushTileCommand::new(brush, page, tiles, offset));
            }
        }
    }

    fn draw(
        &mut self,
        mode: DrawingMode,
        start: Vector2<i32>,
        end: Vector2<i32>,
        sub_pos: Vector2<usize>,
        ui: &mut UserInterface,
    ) {
        let Some(page) = self.page else {
            return;
        };
        let state = self.state.lock();
        let stamp = &state.stamp;
        match mode {
            DrawingMode::Pick => (),
            DrawingMode::Draw | DrawingMode::FloodFill => self.update.draw_tiles(end, stamp),
            DrawingMode::Erase => {
                if stamp.is_empty() {
                    self.update.erase(end);
                } else {
                    self.update.erase_stamp(end, stamp);
                }
            }
            DrawingMode::RectFill => {
                self.update.clear();
                if state.random_mode {
                    self.update.rect_fill_random(start, end, stamp);
                } else {
                    self.update.rect_fill(start, end, stamp);
                }
            }
            DrawingMode::NineSlice => {
                self.update.clear();
                if state.random_mode {
                    self.update.nine_slice_random(start, end, stamp);
                } else {
                    self.update.nine_slice(start, end, stamp);
                }
            }
            DrawingMode::Line => {
                self.update.clear();
                if state.random_mode {
                    self.update.draw_line(start, end, &RandomTileSource(stamp));
                } else {
                    self.update.draw_line(start, end, &stamp.repeat(start, end));
                }
            }
            DrawingMode::Material => {
                if let DrawValue::Material(value) = &state.draw_value {
                    self.tile_set_update.set_material(page, end, value.clone());
                }
            }
            DrawingMode::Property => {
                use TileSetPropertyValue as Value;
                if let Some(prop_id) = &state.active_prop {
                    match &state.draw_value {
                        DrawValue::I8(v) => self
                            .tile_set_update
                            .set_property_slice(page, end, sub_pos, *prop_id, *v),
                        DrawValue::I32(v) => self.tile_set_update.set_property(
                            page,
                            end,
                            *prop_id,
                            Some(Value::I32(*v)),
                        ),
                        DrawValue::F32(v) => self.tile_set_update.set_property(
                            page,
                            end,
                            *prop_id,
                            Some(Value::F32(*v)),
                        ),
                        DrawValue::String(v) => self.tile_set_update.set_property(
                            page,
                            end,
                            *prop_id,
                            Some(Value::String(v.clone())),
                        ),
                        _ => (),
                    }
                }
            }
            DrawingMode::Color => {
                if let DrawValue::Color(v) = &state.draw_value {
                    self.tile_set_update.set_color(page, end, *v);
                }
            }
            DrawingMode::Collider => {
                if let (Some(prop_id), DrawValue::Collider(v)) =
                    (&state.active_prop, &state.draw_value)
                {
                    self.tile_set_update.set_collider(page, end, *prop_id, *v);
                }
            }
        }
    }
    fn end_draw(
        &mut self,
        mode: DrawingMode,
        _start: Vector2<i32>,
        _end: Vector2<i32>,
        _ui: &mut UserInterface,
    ) {
        match mode {
            DrawingMode::Pick => (),
            DrawingMode::Draw => self.send_update(),
            DrawingMode::Erase => self.send_update(),
            DrawingMode::Line => self.send_update(),
            DrawingMode::FloodFill => self.send_update(),
            DrawingMode::RectFill => self.send_update(),
            DrawingMode::NineSlice => self.send_update(),
            DrawingMode::Material => self.send_tile_set_update(),
            DrawingMode::Property => self.send_tile_set_update(),
            DrawingMode::Color => self.send_tile_set_update(),
            DrawingMode::Collider => self.send_tile_set_update(),
        }
    }
    fn accept_material_drop(&mut self, _material: MaterialResource, _ui: &UserInterface) {
        // TODO
        todo!();
    }
    fn push_cell_rect(&self, position: Vector2<i32>, thickness: f32, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let position = Vector2::new(position.x as f32 * size.x, position.y as f32 * size.y);
        let rect = Rect { position, size }.inflate(thickness * 0.5, thickness * 0.5);
        ctx.push_rect(&rect, thickness);
    }
    fn push_cell_rect_filled(&self, position: Vector2<i32>, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let position = Vector2::new(position.x as f32 * size.x, position.y as f32 * size.y);
        let rect = Rect { position, size };
        ctx.push_rect_filled(&rect, None);
    }
    fn push_subcell_rect(&self, position: Subposition, thickness: f32, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let subsize = size / 3.0;
        let position = Vector2::new(
            position.tile.x as f32 * size.x + position.subtile.x as f32 * subsize.x,
            position.tile.y as f32 * size.y + position.subtile.y as f32 * subsize.y,
        );
        let rect = Rect {
            position,
            size: subsize,
        };
        ctx.push_rect(&rect, thickness);
    }
    fn push_subcell_rect_filled(&self, position: Subposition, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let subsize = size / 3.0;
        let position = Vector2::new(
            position.tile.x as f32 * size.x + position.subtile.x as f32 * subsize.x,
            position.tile.y as f32 * size.y + position.subtile.y as f32 * subsize.y,
        );
        let rect = Rect {
            position,
            size: subsize,
        };
        ctx.push_rect_filled(&rect, None);
    }
    fn push_erase_area(&self, thickness: f32, ctx: &mut DrawingContext) {
        let Some(cursor_position) = self.cursor_position else {
            return;
        };
        let state = self.state.lock();
        let stamp = &state.stamp;
        if stamp.is_empty() {
            self.push_x(cursor_position, thickness, ctx);
        } else {
            for pos in stamp.keys() {
                self.push_x(cursor_position + pos, thickness, ctx);
            }
        }
    }
    fn push_x(&self, position: Vector2<i32>, thickness: f32, ctx: &mut DrawingContext) {
        let size = self.tile_size;
        let position = Vector2::new(position.x as f32 * size.x, position.y as f32 * size.y);
        ctx.push_line(position, position + size, thickness);
        ctx.push_line(
            position + Vector2::new(size.x, 0.0),
            position + Vector2::new(0.0, size.y),
            thickness,
        );
    }
    fn commit_color(&self, color: Color, ctx: &mut DrawingContext) {
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(color),
            CommandTexture::None,
            None,
        );
    }
    fn draw_material_background(&self, ctx: &mut DrawingContext) {
        if self.kind != TilePaletteStage::Tiles || !self.editable {
            return;
        }
        let TileResource::TileSet(tile_set) = &self.content else {
            return;
        };
        let Some(page) = self.page else {
            return;
        };
        let mut tile_set = tile_set.state();
        let Some(page) = tile_set.data().and_then(|t| t.pages.get(&page)) else {
            return;
        };
        let TileSetPageSource::Material(mat) = &page.source else {
            return;
        };
        let tile_size = mat.tile_size;
        let mut material = mat.material.state();
        let Some(material) = material.data() else {
            return;
        };
        let Some(tex) = material.texture("diffuseTexture") else {
            return;
        };
        let TextureKind::Rectangle { width, height } = tex.data_ref().kind() else {
            return;
        };
        let width = width as f32 * self.tile_size.x / (tile_size.x as f32);
        let height = height as f32 * self.tile_size.y / (tile_size.y as f32);
        let rect = Rect {
            position: Vector2::default(),
            size: Vector2::new(width, height),
        };
        ctx.transform_stack.push(
            ctx.transform_stack.transform()
                * Matrix3::new_nonuniform_scaling(&Vector2::new(1.0, -1.0)),
        );
        ctx.push_rect_filled(&rect, None);
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(self.material_color),
            CommandTexture::Texture(tex.into_untyped()),
            None,
        );
        ctx.transform_stack.pop();
    }
}

impl Control for PaletteWidget {
    fn draw(&self, ctx: &mut DrawingContext) {
        let bounds = self.bounding_rect();
        ctx.push_rect_filled(&bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );
        let page = self.page.unwrap_or_default();
        let transform = self.tile_to_local();
        let inv_transform = invert_transform(&transform);
        let bounds = bounds.transform(&inv_transform);
        ctx.transform_stack
            .push(self.visual_transform() * transform);

        self.draw_material_background(ctx);

        let stage = self.stage();
        if stage == TilePaletteStage::Tiles && self.page.is_some()
            || stage == TilePaletteStage::Pages
        {
            self.content.tile_render_loop(stage, page, |pos, data| {
                if self.overlay.covers(pos) {
                    return;
                }
                if self.update.contains_key(&pos) {
                    return;
                }
                let Some(handle) = TileDefinitionHandle::try_new(page, pos) else {
                    return;
                };
                if self.tile_set_update.contains_key(&handle) {
                    return;
                }
                let t = self.tile_size;
                let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
                let rect = Rect { position, size: t };
                draw_tile(rect, self.clip_bounds(), &data, ctx);
            });
        }

        if let Some(tile_set) = self.content.get_tile_set() {
            let mut tile_set = tile_set.state();
            if let Some(tile_set) = tile_set.data() {
                for (pos, v) in self.update.iter() {
                    let Some((t, h)) = v else {
                        continue;
                    };
                    let Some(data) = tile_set.get_transformed_render_data(*t, *h) else {
                        continue;
                    };
                    let t = self.tile_size;
                    let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
                    let rect = Rect { position, size: t };
                    draw_tile(rect, self.clip_bounds(), &data, ctx);
                }
                for (handle, v) in self.tile_set_update.iter() {
                    let pos = handle.tile();
                    let Some(handle) = v.substitute_transform_handle(*handle) else {
                        continue;
                    };
                    let data = tile_set
                        .get_tile_render_data(TilePaletteStage::Tiles, handle)
                        .unwrap_or_else(TileRenderData::missing_data);
                    let Some(data) = v.modify_render(&data) else {
                        continue;
                    };
                    let t = self.tile_size;
                    let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
                    let rect = Rect { position, size: t };
                    draw_tile(rect, self.clip_bounds(), &data, ctx);
                }
                for (pos, data) in self.overlay.iter() {
                    let t = self.tile_size;
                    let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
                    let rect = Rect { position, size: t };
                    draw_tile(rect, self.clip_bounds(), data, ctx);
                }
            }
        }

        ctx.push_grid(self.zoom, self.tile_size, bounds);
        self.commit_color(Color::BLACK, ctx);

        // Transform areas
        if stage == TilePaletteStage::Tiles && self.content.is_transform_page(page) {
            let area_size = Vector2::new(self.tile_size.x * 4.0, self.tile_size.y * 2.0);
            ctx.push_grid(self.zoom, area_size, bounds);
            self.commit_color(Color::ORANGE, ctx);
        }

        let line_thickness = 1.0 / self.zoom;
        let left = bounds.left_bottom_corner().x;
        let right = bounds.right_bottom_corner().x;
        let top = bounds.left_top_corner().y;
        let bottom = bounds.left_bottom_corner().y;

        // Axis lines
        ctx.push_line(
            Vector2::new(left, 0.0),
            Vector2::new(right, 0.0),
            line_thickness,
        );
        ctx.push_line(
            Vector2::new(0.0, top),
            Vector2::new(0.0, bottom),
            line_thickness,
        );
        self.commit_color(Color::RED, ctx);

        for (pos, c) in self.highlight.iter() {
            self.push_subcell_rect_filled(*pos, ctx);
            self.commit_color(*c, ctx);
        }
        for pos in self.highlight.keys() {
            self.push_subcell_rect(*pos, line_thickness, ctx);
        }
        self.commit_color(Color::BLACK, ctx);

        if let Some(pos) = self.cursor_position {
            if self.slice_mode {
                let pos = Subposition {
                    tile: pos,
                    subtile: self.slice_position,
                };
                self.push_subcell_rect_filled(pos, ctx);
            } else {
                self.push_cell_rect_filled(pos, ctx);
            }
            self.commit_color(Color::WHITE.with_new_alpha(150), ctx);
        }

        if self.editable
            && self.kind == TilePaletteStage::Tiles
            && self.state.lock().drawing_mode == DrawingMode::Erase
        {
            self.push_erase_area(line_thickness, ctx);
            self.commit_color(Color::RED, ctx);
        }

        // Selection highlight
        if stage == TilePaletteStage::Pages {
            if let Some(active) = self.page {
                self.push_cell_rect(active, line_thickness * 3.0, ctx);
                self.commit_color(Color::GREEN_YELLOW, ctx);
            }
        }
        let state = self.state.lock();
        if state.selection_palette() == self.handle {
            for sel in state.selection_positions() {
                self.push_cell_rect(*sel, line_thickness * 2.0, ctx)
            }
            self.commit_color(Color::WHITE, ctx);
        }

        ctx.transform_stack.pop();

        ctx.draw_text(
            self.clip_bounds(),
            Vector2::new(2.0, 2.0),
            &self.position_text,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            ui.capture_mouse(self.handle());
            if *button == MouseButton::Middle {
                self.mode = MouseMode::Panning {
                    initial_view_position: self.view_position,
                    click_position: *pos,
                };
            } else if *button == MouseButton::Left && !message.handled() {
                if let Some(mode) = self.drawing_mode() {
                    let mouse_pos = self.calc_mouse_position(*pos);
                    self.begin_motion(mode, mouse_pos, ui);
                }
            }
        } else if let Some(WidgetMessage::MouseUp { pos, button, .. }) = message.data() {
            ui.release_mouse_capture();
            if *button == MouseButton::Left {
                if let Some(mode) = self.drawing_mode() {
                    let mouse_pos = self.calc_mouse_position(*pos);
                    self.end_motion(mode, mouse_pos, ui);
                }
            }
            self.mode = MouseMode::None;
        } else if let Some(WidgetMessage::MouseMove { pos, .. }) = message.data() {
            if let MouseMode::Panning {
                initial_view_position,
                click_position,
            } = &self.mode
            {
                self.view_position = initial_view_position + (*pos - click_position);
            }
            let mouse_pos = self.calc_mouse_position(*pos);
            self.slice_position = mouse_pos.subgrid;
            self.set_cursor_position(Some(mouse_pos.grid));
            if let Some(drawing_mode) = self.drawing_mode() {
                self.continue_motion(drawing_mode, mouse_pos, ui);
            }
        } else if let Some(WidgetMessage::MouseEnter { .. }) = message.data() {
            self.overlay.active = true;
        } else if let Some(WidgetMessage::MouseLeave { .. }) = message.data() {
            self.overlay.active = false;
            self.set_cursor_position(None);
        } else if let Some(WidgetMessage::MouseWheel { amount, pos }) = message.data() {
            let tile_pos = self.screen_point_to_tile_point(*pos);
            self.zoom = (self.zoom + 0.1 * amount).clamp(0.2, 2.0);
            let new_pos = self.tile_point_to_screen_point(tile_pos);
            self.view_position += pos - new_pos;
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                if let Some(material) = item.resource::<Material>() {
                    self.accept_material_drop(material, ui);
                }
            }
        } else if let Some(msg) = message.data::<PaletteMessage>() {
            if message.direction() == MessageDirection::ToWidget {
                match msg {
                    PaletteMessage::SetPage { source, page } => {
                        self.set_page(source.clone(), *page, ui)
                    }
                    PaletteMessage::SelectAll => self.select_all(),
                    PaletteMessage::Delete => drop(self.delete_tiles(ui)),
                    PaletteMessage::MaterialColor(color) => self.material_color = *color,
                    PaletteMessage::SyncToState => self.sync_to_state(),
                    _ => (),
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            if *key == KeyCode::Delete && !message.handled() && self.delete_tiles(ui) {
                message.set_handled(true);
            }
        }
    }
}

pub struct PaletteWidgetBuilder {
    widget_builder: WidgetBuilder,
    tile_resource: TileResource,
    sender: MessageSender,
    state: TileDrawStateRef,
    kind: TilePaletteStage,
    editable: bool,
}

impl PaletteWidgetBuilder {
    pub fn new(
        widget_builder: WidgetBuilder,
        sender: MessageSender,
        state: TileDrawStateRef,
    ) -> Self {
        Self {
            widget_builder,
            tile_resource: TileResource::Empty,
            sender,
            state,
            kind: TilePaletteStage::default(),
            editable: false,
        }
    }

    pub fn with_resource(mut self, tile_resource: TileResource) -> Self {
        self.tile_resource = tile_resource;
        self
    }

    pub fn with_kind(mut self, kind: TilePaletteStage) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(PaletteWidget {
            widget: self
                .widget_builder
                .with_allow_drop(true)
                .with_clip_to_bounds(false)
                .build(),
            sender: self.sender,
            state: self.state,
            overlay: PaletteOverlay::default(),
            content: self.tile_resource,
            kind: self.kind,
            editable: self.editable,
            material_color: DEFAULT_MATERIAL_COLOR,
            page: None,
            cursor_position: None,
            slice_position: Vector2::default(),
            slice_mode: true,
            position_text: FormattedTextBuilder::new(ctx.inner().default_font.clone())
                .with_brush(Brush::Solid(Color::WHITE))
                .build(),
            selecting_tiles: FxHashSet::default(),
            highlight: FxHashMap::default(),
            update: TransTilesUpdate::default(),
            tile_set_update: TileSetUpdate::default(),
            view_position: Default::default(),
            zoom: 1.0,
            tile_size: Vector2::repeat(32.0),
            mode: MouseMode::None,
        }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TileViewMessage {
    LocalPosition(Vector2<i32>),
}

impl TileViewMessage {
    define_constructor!(TileViewMessage:LocalPosition => fn local_position(Vector2<i32>), layout: false);
}

fn draw_tile(
    position: Rect<f32>,
    clip_bounds: Rect<f32>,
    tile: &TileRenderData,
    drawing_context: &mut DrawingContext,
) {
    let color = tile.color;
    if let Some(material_bounds) = &tile.material_bounds {
        if let Some(texture) = material_bounds
            .material
            .state()
            .data()
            .and_then(|m| m.texture("diffuseTexture"))
        {
            let kind = texture.data_ref().kind();
            if let TextureKind::Rectangle { width, height } = kind {
                let size = Vector2::new(width, height);
                let bounds = &material_bounds.bounds;
                drawing_context.push_rect_filled(
                    &position,
                    Some(&[
                        bounds.left_bottom_uv(size),
                        bounds.right_bottom_uv(size),
                        bounds.right_top_uv(size),
                        bounds.left_top_uv(size),
                    ]),
                );
                drawing_context.commit(
                    clip_bounds,
                    Brush::Solid(color),
                    CommandTexture::Texture(texture.into()),
                    None,
                );
            }
        } else {
            drawing_context.push_rect_filled(&position, None);
            drawing_context.commit(clip_bounds, Brush::Solid(color), CommandTexture::None, None);
        }
    } else {
        drawing_context.push_rect_filled(&position, None);
        drawing_context.commit(clip_bounds, Brush::Solid(color), CommandTexture::None, None);
    }
}
