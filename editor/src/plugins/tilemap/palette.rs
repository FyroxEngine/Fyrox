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

//! The [`PaletteWidget`] is the core of the tile set editor, because this widget
//! is responsible for displaying a grid of tiles where the user may select tiles,
//! drag tiles, and use drawing tools upon the tiles.

use fyrox::scene::tilemap::brush::TileMapBrushResource;
use fyrox::scene::tilemap::tileset::OptionTileSet;
use fyrox::scene::tilemap::{ResourceTilePosition, RotTileHandle};

use super::{commands::*, *};
use crate::asset::item::AssetItem;
use crate::command::{Command, CommandGroup};
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
    fxhash::FxHashSet,
    graph::BaseSceneGraph,
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        formatted_text::{FormattedText, FormattedTextBuilder},
        message::CursorIcon,
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
    material::{Material, MaterialResource},
    resource::texture::TextureKind,
    scene::tilemap::{
        tileset::{TileSetPageSource, TileSetRef},
        OrthoTransformation, TileBook, TilePaletteStage, TileRect, TileRenderData, TileSetUpdate,
        TileSource, TransTilesUpdate,
    },
};

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

/// The tint of the background material that is used for tile atlas pages of tile sets.
/// This tint makes it possible to visibly distinguish the background material from actual tiles.
pub const DEFAULT_MATERIAL_COLOR: Color = Color::from_rgba(255, 255, 255, 125);
/// A mostly-transparent rectangle is drawn over the tile that the mouse is currently over,
/// thereby visually confirming for the user which tile they would click on if they clicked.
/// This is the color of that rectangle.
pub const CURSOR_HIGHLIGHT_COLOR: Color = Color::from_rgba(255, 255, 255, 50);
/// When a macro is using some cells within a brush, those cells are indicated by a special outline
/// in the brush editor. This is the color of that outline.
pub const MACRO_CELL_HIGHLIGHT_COLOR: Color = Color::DARK_SLATE_BLUE;

const MOUSE_CLICK_DELAY_FRAMES: usize = 1;
const NO_PAGE_COLOR: Color = Color::from_rgba(20, 5, 5, 255);
const ANIMATION_BOOKEND_COLOR: Color = Color::DARK_CYAN;

/// Messages for the [`PaletteWidget`] widget.
#[derive(Debug, PartialEq, Clone)]
pub enum PaletteMessage {
    /// Display the given page of the given resource.
    SetPage {
        /// The resource to show in the palette widget.
        source: TileBook,
        /// The coordinates of the page, or None to show no page.
        page: Option<Vector2<i32>>,
    },
    /// Center the view on the given grid position.
    Center(Vector2<i32>),
    /// Select all tiles/pages in this view.
    SelectAll,
    /// Select the given position.
    SelectOne(Vector2<i32>),
    /// Delete the selected tiles/pages in this view.
    Delete,
    /// Set the tint of the background material.
    MaterialColor(Color),
    /// Notify this widget that the editor state has changed.
    SyncToState,
    /// Notify that the user has pressed a mouse button.
    /// This is needed in order to delay the start of mouse operations
    /// by one frame so that they do not clash with operations that happen
    /// when de-focusing whatever was previously in focus.
    BeginMotion(Vector2<f32>),
}

impl PaletteMessage {
    define_constructor!(
        /// Display the given page of the given resource.
        PaletteMessage:SetPage => fn set_page(source: TileBook, page: Option<Vector2<i32>>), layout: false);
    define_constructor!(
        /// Center the view on the given grid position.
        PaletteMessage:Center => fn center(Vector2<i32>), layout: false);
    define_constructor!(
        /// Select all tiles/pages in this view.
        PaletteMessage:SelectAll => fn select_all(), layout: false);
    define_constructor!(
        /// Select the given position.
        PaletteMessage:SelectOne => fn select_one(Vector2<i32>), layout: false);
    define_constructor!(
        /// Delete the selected tiles/pages in this view.
        PaletteMessage:Delete => fn delete(), layout: false);
    define_constructor!(
        /// Set the tint of the background material.
        PaletteMessage:MaterialColor => fn material_color(Color), layout: false);
    define_constructor!(
        /// Notify this widget that the editor state has changed.
        PaletteMessage:SyncToState => fn sync_to_state(), layout: false);
    define_constructor!(
        /// Notify that the user has pressed a mouse button.
        /// This is needed in order to delay the start of mouse operations
        /// by one frame so that they do not clash with operations that happen
        /// when de-focusing whatever was previously in focus.
        PaletteMessage:BeginMotion => fn begin_motion(Vector2<f32>), layout: false);
}

/// The operation of the current mouse motion.
#[derive(Clone, Default, Debug, PartialEq)]
enum MouseMode {
    /// The mouse is doing nothing relevant.
    #[default]
    None,
    /// The middle mouse button is down and the mouse is dragging to move the view.
    Panning {
        initial_view_position: Vector2<f32>,
        click_position: Vector2<f32>,
    },
    /// The left mouse button is down and the mouse is moving some tiles.
    Dragging {
        initial_position: Vector2<f32>,
        offset: Vector2<i32>,
    },
    /// The left mouse button is down and the mouse is performing some draw operation,
    /// such as a fill rect operation or a flood fill. Possible operations include
    /// selecting tiles.
    Drawing {
        start_tile: Vector2<i32>,
        end: MousePos,
    },
}

/// A collection of data relevant to the position of the mouse.
#[derive(Clone, Default, Debug, PartialEq)]
struct MousePos {
    /// The mouse position in floats.
    fine: Vector2<f32>,
    /// The position of the tile grid cell that contains the mouse.
    grid: Vector2<i32>,
    /// The position of one of the nine areas within a tile cell that contains the mouse.
    /// This is used for editing nine slice property values.
    subgrid: Vector2<usize>,
}

/// A position within a tile grid, including coordinates within the cell
/// to indicate one of the nine divisions of the tile for the purpose of
/// nine slice properties.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Hash)]
pub struct Subposition {
    /// The tile cell coordinates
    pub tile: Vector2<i32>,
    /// The nine slice area within the cell, with x in 0..3 and y in 0..3.
    pub subtile: Vector2<usize>,
}

/// Calculate the one of the three subtile positions along some axis when
/// given the position on the axis and the size of the tiles.
/// It always returns 0, 1, or 2.
fn calc_slice_coord(position: f32, step: f32) -> usize {
    let p = position / step;
    let p = (p - p.floor()) * 3.0;
    (p.floor() as i32).clamp(0, 2) as usize
}

/// Displays a scrollable grid of till cells, with options to allow the tiles
/// to be selected, dragged, and edits in various ways.
#[derive(Clone, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "5356a864-c026-4bd7-a4b1-30bacf77d8fa")]
#[reflect(derived_type = "UiNode")]
pub struct PaletteWidget {
    widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: MessageSender,
    /// The resource that holds the tiles to diaplay or edit.
    pub content: TileBook,
    /// The page to display within the tile resource.
    pub page: Option<Vector2<i32>>,
    /// The update that contains the current editing operation when the user
    /// is doing something like a rect fill, drawing a line, or erasing.
    #[visit(skip)]
    #[reflect(hidden)]
    pub update: TransTilesUpdate,
    /// The update that contains the current editing operation when the user
    /// is modifying tile data like color, material, collider shape, or property value.
    #[visit(skip)]
    #[reflect(hidden)]
    pub tile_set_update: TileSetUpdate,
    /// The current editor state that is shared between palette widgets like this one,
    /// and the tile map control panel, and the tile map interaction mode.
    /// This allows these diverse objects to coordinate with each other about what
    /// the user is currently doing.
    #[visit(skip)]
    #[reflect(hidden)]
    pub state: TileDrawStateRef,
    /// Whether this palette is showing actual tiles, or whether it is showing the tile icons
    /// that represent the pages of a tile set or brush.
    pub kind: TilePaletteStage,
    /// Are these tiles editable, or are they read-only?
    pub editable: bool,
    /// Is the user editing whole tiles, or is the user editing one of the nine subareas within a tile?
    /// True if the user is editing subareas.
    pub slice_mode: bool,
    /// The tint of the background material that is used for tile atlas pages of tile sets.
    /// This tint makes it possible to visibly distinguish the background material from actual tiles.
    material_color: Color,
    /// These are the positions of tiles that are in the process of being selected, but not actually selected.
    /// Tile selection is a two-stage process to give the user a smooth experience. The actually selected tiles
    /// are stored in the [`TileDrawState::selection`] so that all interested parties can see what is currently
    /// selected. In contrast, this set contains a record of what was selected before the user began the current
    /// mouse motion, if the user held shift to prevent that selection from being removed.
    ///
    /// In order to calculate the actual selection, this set is combined with the rect created by the current
    /// mouse motion.
    #[visit(skip)]
    #[reflect(hidden)]
    selecting_tiles: FxHashSet<Vector2<i32>>,
    /// The highlight that is used to visualize a property layer when in [`DrawingMode::Editor`].
    #[visit(skip)]
    #[reflect(hidden)]
    highlight: FxHashMap<Subposition, Color>,
    /// The cells which should be marked as being included in some macro.
    #[visit(skip)]
    #[reflect(hidden)]
    macro_cells: Option<MacroCellSetListRef>,
    /// The cells which should be marked as being included in some macro.
    #[visit(skip)]
    #[reflect(hidden)]
    macro_list: Option<BrushMacroListRef>,
    #[visit(skip)]
    #[reflect(hidden)]
    colliders: Vec<ColliderHighlight>,
    #[visit(skip)]
    #[reflect(hidden)]
    cursor_position: Option<Vector2<i32>>,
    /// A copy of the current drawing mode that is made whenever the user
    /// presses the mouse button. While the actual drawing mode may change
    /// during a mouse stroke, this value never will, so nothing breaks by changing
    /// tool in the middle of a mouse stroke.
    #[visit(skip)]
    #[reflect(hidden)]
    current_tool: DrawingMode,
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
    #[visit(skip)]
    #[reflect(hidden)]
    collider_triangles: RefCell<PaletteTriangleData>,
}

define_widget_deref!(PaletteWidget);

impl Debug for PaletteWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaletteWidget")
            .field("widget", &self.widget)
            .field("content", &self.content)
            .field("page", &self.page)
            .field("kind", &self.kind)
            .field("editable", &self.editable)
            .field("material_color", &self.material_color)
            .field("tile_size", &self.tile_size)
            .finish()
    }
}

type PaletteTriangleData = (Vec<Point2<f32>>, Vec<[u32; 3]>);

#[derive(Debug, Clone)]
struct ColliderHighlight {
    position: Vector2<i32>,
    color: Color,
    tile_collider: TileCollider,
}

impl ColliderHighlight {
    fn new(position: Vector2<i32>, color: Color, tile_collider: TileCollider) -> Self {
        Self {
            position,
            color,
            tile_collider,
        }
    }
}

#[derive(Default, Clone, Debug)]
struct PaletteOverlay {
    movable_position: Vector2<i32>,
    movable_tiles: FxHashMap<Vector2<i32>, TileRenderData>,
    erased_tiles: FxHashSet<Vector2<i32>>,
}

impl PaletteOverlay {
    pub fn covers(&self, position: Vector2<i32>) -> bool {
        self.erased_tiles.contains(&position)
            || self
                .movable_tiles
                .contains_key(&(position - self.movable_position))
    }
    pub fn iter(&self) -> impl Iterator<Item = (Vector2<i32>, &TileRenderData)> {
        let offset = self.movable_position;
        self.movable_tiles
            .iter()
            .map(move |(p, d)| (*p + offset, d))
    }
    pub fn clear(&mut self) {
        self.movable_tiles.clear();
        self.erased_tiles.clear();
    }
    pub fn set_to_stamp(&mut self, stamp: &Stamp, tile_set: &OptionTileSet) {
        self.movable_tiles.clear();
        self.erased_tiles.clear();
        for (pos, StampElement { handle, .. }) in stamp.iter() {
            let data = if handle.is_empty() {
                TileRenderData::empty()
            } else {
                tile_set
                    .get_transformed_render_data(stamp.transformation(), *handle)
                    .unwrap_or_else(TileRenderData::missing_data)
            };
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

fn send_update_tile_set_pages(
    tile_set: &TileSetResource,
    update: &TransTilesUpdate,
    state: &TileDrawStateRef,
    sender: &mut MessageSender,
) {
    match state.lock().stamp.source() {
        Some(TileBook::TileSet(source)) => {
            send_copy_tile_set_pages(source, update, tile_set, sender)
        }
        _ => send_update_tile_set_icons(update, tile_set, sender),
    }
}

fn send_copy_tile_set_pages(
    source: &TileSetResource,
    update: &TransTilesUpdate,
    destination: &TileSetResource,
    sender: &mut MessageSender,
) {
    let mut commands = Vec::default();
    for (p, data) in update.iter() {
        let Some(RotTileHandle {
            element:
                StampElement {
                    handle,
                    source: element_source,
                },
            transform,
        }) = data
        else {
            commands.push(Command::new(SetTileSetPageCommand {
                tile_set: destination.clone(),
                position: *p,
                page: None,
            }));
            continue;
        };
        if let Some(ResourceTilePosition::Page(source_page)) = element_source {
            let source = source.data_ref();
            let mut page_data = source.get_page(*source_page).cloned();
            if let Some(page_data) = page_data.as_mut() {
                let icon = page_data.icon;
                let icon = source
                    .get_transformed_version(*transform, icon)
                    .unwrap_or(icon);
                page_data.icon = icon;
            }
            commands.push(Command::new(SetTileSetPageCommand {
                tile_set: destination.clone(),
                position: *p,
                page: page_data,
            }));
        } else {
            let source = source.data_ref();
            let icon = source
                .get_transformed_version(*transform, *handle)
                .unwrap_or(*handle);
            commands.push(Command::new(ModifyPageIconCommand::new(
                destination.clone(),
                *p,
                icon,
            )));
        }
    }
    if !commands.is_empty() {
        sender.do_command(CommandGroup::from(commands).with_custom_name("Edit Tile Set Pages"));
    }
}

fn send_update_tile_set_icons(
    update: &TransTilesUpdate,
    destination: &TileSetResource,
    sender: &mut MessageSender,
) {
    let mut commands = Vec::default();
    for (p, data) in update.iter() {
        let Some(RotTileHandle {
            element: StampElement { handle, .. },
            transform,
        }) = data
        else {
            commands.push(Command::new(SetTileSetPageCommand {
                tile_set: destination.clone(),
                position: *p,
                page: None,
            }));
            continue;
        };
        let tile_set = destination.data_ref();
        let icon = tile_set
            .get_transformed_version(*transform, *handle)
            .unwrap_or(*handle);
        commands.push(Command::new(ModifyPageIconCommand::new(
            destination.clone(),
            *p,
            icon,
        )));
    }
    if !commands.is_empty() {
        sender.do_command(CommandGroup::from(commands).with_custom_name("Edit Tile Set Pages"));
    }
}

fn send_update_brush_pages(
    brush: &TileMapBrushResource,
    update: &TransTilesUpdate,
    state: &TileDrawStateRef,
    macro_list: Option<&BrushMacroListRef>,
    sender: &mut MessageSender,
) {
    let guard = state.lock();
    match guard.stamp.source() {
        Some(TileBook::Brush(source)) => {
            send_copy_brush_pages(source, update, brush, macro_list, sender)
        }
        _ => {
            send_update_brush_icons(guard.tile_set.as_ref(), update, brush, macro_list, sender);
        }
    }
}

fn send_copy_brush_pages(
    source: &TileMapBrushResource,
    update: &TransTilesUpdate,
    destination: &TileMapBrushResource,
    macro_list: Option<&BrushMacroListRef>,
    sender: &mut MessageSender,
) {
    let mut commands = CommandGroup::default();
    let same_brush = source == destination;
    for (p, data) in update.iter() {
        let Some(RotTileHandle {
            element:
                StampElement {
                    handle,
                    source: element_source,
                },
            transform,
        }) = data
        else {
            make_commands_to_erase_brush_page(destination, *p, macro_list, &mut commands);
            continue;
        };
        if let Some(ResourceTilePosition::Page(source_page)) = element_source {
            let source = source.data_ref();
            let tile_set = source.tile_set();
            let mut page_data = source.pages.get(source_page).cloned();
            if let (Some(tile_set), Some(page_data)) = (tile_set, page_data.as_mut()) {
                let icon = page_data.icon;
                let icon = tile_set
                    .data_ref()
                    .get_transformed_version(*transform, icon)
                    .unwrap_or(icon);
                page_data.icon = icon;
            }
            if same_brush {
                drop(source);
                make_commands_to_copy_brush_page(
                    destination,
                    *source_page,
                    *p,
                    macro_list,
                    &mut commands,
                );
            }
            commands.push(SetBrushPageCommand {
                brush: destination.clone(),
                position: *p,
                page: page_data,
            });
        } else {
            let source = source.data_ref();
            let icon = if let Some(tile_set) = source.tile_set() {
                tile_set
                    .data_ref()
                    .get_transformed_version(*transform, *handle)
                    .unwrap_or(*handle)
            } else {
                *handle
            };
            commands.push(ModifyBrushPageIconCommand::new(
                destination.clone(),
                *p,
                icon,
            ));
        }
    }
    if !commands.is_empty() {
        sender.do_command(commands.with_custom_name("Edit Tile Set Pages"));
    }
}

fn send_update_brush_icons(
    tile_set: Option<&TileSetResource>,
    update: &TransTilesUpdate,
    destination: &TileMapBrushResource,
    macro_list: Option<&BrushMacroListRef>,
    sender: &mut MessageSender,
) {
    let mut commands = CommandGroup::default();
    for (p, data) in update.iter() {
        let Some(RotTileHandle {
            element: StampElement { handle, .. },
            transform,
        }) = data
        else {
            make_commands_to_erase_brush_page(destination, *p, macro_list, &mut commands);
            continue;
        };
        let icon = if let Some(tile_set) = tile_set {
            tile_set
                .data_ref()
                .get_transformed_version(*transform, *handle)
                .unwrap_or(*handle)
        } else {
            *handle
        };
        commands.push(ModifyBrushPageIconCommand::new(
            destination.clone(),
            *p,
            icon,
        ));
    }
    if !commands.is_empty() {
        sender.do_command(commands.with_custom_name("Edit Tile Set Pages"));
    }
}

fn make_commands_to_erase_brush_page(
    brush: &TileMapBrushResource,
    page: Vector2<i32>,
    macro_list: Option<&BrushMacroListRef>,
    commands: &mut CommandGroup,
) {
    if let Some(macro_list) = macro_list {
        let macro_list = macro_list.lock();
        for instance in brush.data_ref().macros.iter() {
            if let Some(m) = macro_list.get_by_uuid(&instance.macro_id) {
                if let Some(command) = m.copy_page(
                    None,
                    page,
                    &BrushMacroInstance {
                        brush: brush.clone(),
                        settings: instance.settings.clone(),
                    },
                ) {
                    commands.push_command(command);
                }
            }
        }
    }
    commands.push(SetBrushPageCommand {
        brush: brush.clone(),
        position: page,
        page: None,
    });
}

fn make_commands_to_copy_brush_page(
    brush: &TileMapBrushResource,
    from: Vector2<i32>,
    to: Vector2<i32>,
    macro_list: Option<&BrushMacroListRef>,
    commands: &mut CommandGroup,
) {
    if let Some(macro_list) = macro_list {
        let macro_list = macro_list.lock();
        for instance in brush.data_ref().macros.iter() {
            if let Some(m) = macro_list.get_by_uuid(&instance.macro_id) {
                if let Some(command) = m.copy_page(
                    Some(from),
                    to,
                    &BrushMacroInstance {
                        brush: brush.clone(),
                        settings: instance.settings.clone(),
                    },
                ) {
                    commands.push_command(command);
                }
            }
        }
    }
}

fn make_commands_to_copy_brush_tile(
    brush: &TileMapBrushResource,
    from: Option<TileDefinitionHandle>,
    to: TileDefinitionHandle,
    macro_list: Option<&BrushMacroListRef>,
    commands: &mut CommandGroup,
) {
    if let Some(macro_list) = macro_list {
        let macro_list = macro_list.lock();
        for instance in brush.data_ref().macros.iter() {
            if let Some(m) = macro_list.get_by_uuid(&instance.macro_id) {
                if let Some(command) = m.copy_cell(
                    from,
                    to,
                    &BrushMacroInstance {
                        brush: brush.clone(),
                        settings: instance.settings.clone(),
                    },
                ) {
                    commands.push_command(command);
                }
            }
        }
    }
}

impl PaletteWidget {
    /// Each brush and tile set has two palette areas: the pages and the tiles within each page.
    /// These two areas are called stages, and each of the two stages needs to be handled separately.
    /// Giving a particular `TilePaletteStage` to a tile map palette will control which kind of
    /// tiles it will display.
    pub fn stage(&self) -> TilePaletteStage {
        match &self.kind {
            TilePaletteStage::Pages => TilePaletteStage::Pages,
            _ => TilePaletteStage::Tiles,
        }
    }

    fn send_cursor_icon(&self, icon: Option<CursorIcon>, ui: &mut UserInterface) {
        ui.send_message(WidgetMessage::cursor(
            self.handle(),
            MessageDirection::ToWidget,
            icon,
        ));
    }
    fn sync_to_state(&mut self, ui: &mut UserInterface) {
        let state = self.state.lock();
        let drawing_mode = if self.editable {
            state.drawing_mode
        } else {
            DrawingMode::Pick
        };
        let icon = match drawing_mode {
            DrawingMode::Pick => Some(CursorIcon::Pointer),
            DrawingMode::Draw => Some(CursorIcon::Crosshair),
            DrawingMode::Erase => Some(CursorIcon::Crosshair),
            DrawingMode::FloodFill => Some(CursorIcon::Crosshair),
            DrawingMode::RectFill => Some(CursorIcon::Crosshair),
            DrawingMode::Line => Some(CursorIcon::Crosshair),
            DrawingMode::NineSlice => Some(CursorIcon::Crosshair),
            DrawingMode::Editor => None,
        };
        self.send_cursor_icon(icon, ui);
        if state.selection_palette() != self.handle {
            self.selecting_tiles.clear();
        }
        self.slice_mode = if let Some(editor) = &state.active_editor {
            self.editable
                && self.kind == TilePaletteStage::Tiles
                && state.drawing_mode == DrawingMode::Editor
                && editor.lock().slice_mode()
        } else {
            false
        };
        self.colliders.clear();
        if self.kind == TilePaletteStage::Tiles
            && self.page.is_some()
            && !state.visible_colliders.is_empty()
        {
            let page = self.page.unwrap();
            self.content
                .tile_collider_loop(page, |pos, uuid, color, tile_collider| {
                    if !state.visible_colliders.contains(&uuid) {
                        return;
                    }
                    self.colliders
                        .push(ColliderHighlight::new(pos, color, tile_collider.clone()));
                });
        }
        if self.editable {
            self.overlay.clear();
            self.highlight.clear();
            match state.drawing_mode {
                DrawingMode::Draw | DrawingMode::FloodFill => {
                    if let Some(tile_set) = state.tile_set.as_ref() {
                        self.overlay
                            .set_to_stamp(&state.stamp, &TileSetRef::new(tile_set).as_loaded());
                    }
                }
                DrawingMode::Editor => {
                    if let Some(editor) = &state.active_editor {
                        if let &Some(page) = &self.page {
                            editor.lock().highlight(
                                &mut self.highlight,
                                page,
                                &self.content,
                                &self.tile_set_update,
                            );
                        }
                    }
                }
                _ => (),
            }
        }
    }

    /// Convert a point measured in tiles to a point on the screen, depending on how this widget
    /// is currently scrolled and zoomed. This is the inverse of [`Self::tile_point_to_screen_point`].
    /// When measuring a point in tiles, (0,0) is the left-bottom corner of the (0,0) tile,
    /// and each unit along x or y is one tile.
    pub fn screen_point_to_tile_point(&self, point: Vector2<f32>) -> Vector2<f32> {
        let trans = self.visual_transform() * self.tile_to_local();
        let trans = invert_transform(&trans);
        apply_transform(&trans, point)
    }

    /// Convert a point on the screen to a point measured in tiles, depending on how this widget
    /// is currently scrolled and zoomed. This is the inverse of [`Self::screen_point_to_tile_point`].
    /// When measuring a point in tiles, (0,0) is the left-bottom corner of the (0,0) tile,
    /// and each unit along x or y is one tile.
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
        match self.kind {
            TilePaletteStage::Pages => self.send_update_pages(),
            TilePaletteStage::Tiles => self.send_update_tiles(),
        }
    }
    fn send_update_pages(&mut self) {
        assert_eq!(self.kind, TilePaletteStage::Pages);
        match &self.content {
            TileBook::Empty => (),
            TileBook::TileSet(tile_set) => {
                send_update_tile_set_pages(tile_set, &self.update, &self.state, &mut self.sender);
            }
            TileBook::Brush(brush) => {
                send_update_brush_pages(
                    brush,
                    &self.update,
                    &self.state,
                    self.macro_list.as_ref(),
                    &mut self.sender,
                );
            }
        }
        self.update.clear();
    }
    fn send_update_tiles(&mut self) {
        assert_eq!(self.kind, TilePaletteStage::Tiles);
        let Some(page) = self.page else {
            return;
        };
        let state = self.state.lock();
        let source = state.stamp.source();
        let source_set = state.tile_set.as_ref();
        match &self.content {
            TileBook::Empty => (),
            TileBook::TileSet(resource) => {
                self.tile_set_update.clear();
                self.tile_set_update.convert(
                    &self.update,
                    resource,
                    page,
                    source_set.unwrap_or(resource),
                );
                self.sender.do_command(SetTileSetTilesCommand {
                    tile_set: resource.clone(),
                    tiles: self.tile_set_update.clone(),
                });
                self.tile_set_update.clear();
                self.update.clear();
            }
            TileBook::Brush(resource) => {
                if let Some(source_set) = source_set
                    .cloned()
                    .or_else(|| resource.state().data()?.tile_set())
                {
                    let mut commands = CommandGroup::default();
                    let same_brush = if let Some(TileBook::Brush(brush)) = source.as_ref() {
                        brush == resource
                    } else {
                        false
                    };
                    for (p, d) in self.update.iter() {
                        let from = if same_brush {
                            d.as_ref().and_then(|d| d.element.source?.handle())
                        } else {
                            None
                        };
                        let Some(to) = TileDefinitionHandle::try_new(page, *p) else {
                            continue;
                        };
                        make_commands_to_copy_brush_tile(
                            resource,
                            from,
                            to,
                            self.macro_list.as_ref(),
                            &mut commands,
                        );
                    }
                    let mut source_set = TileSetRef::new(&source_set);
                    commands.push(SetBrushTilesCommand {
                        brush: resource.clone(),
                        page,
                        tiles: self.update.build_tiles_update(&source_set.as_loaded()),
                    });
                    self.sender
                        .do_command(commands.with_custom_name("Edit Brush Tiles"));
                }
                self.update.clear();
            }
        }
    }
    fn send_tile_set_update(&mut self) {
        assert_eq!(self.kind, TilePaletteStage::Tiles);
        if let TileBook::TileSet(resource) = &self.content {
            self.sender.do_command(SetTileSetTilesCommand {
                tile_set: resource.clone(),
                tiles: self.tile_set_update.clone(),
            });
            self.tile_set_update.clear();
        }
    }
    fn delete_tiles(&mut self, _ui: &mut UserInterface) -> bool {
        let state = self.state.lock_mut("delete_tiles");
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
    fn set_page(&mut self, resource: TileBook, page: Option<Vector2<i32>>, ui: &mut UserInterface) {
        let mut state = self.state.lock_mut("set_page");
        if state.selection_palette() == self.handle {
            self.selecting_tiles.clear();
            state.clear_selection();
        }
        self.page = page;
        self.content = resource;
        drop(state);
        self.sync_to_state(ui);
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
    fn drawing_mode(&self) -> DrawingMode {
        if self.editable {
            self.current_tool
        } else {
            DrawingMode::Pick
        }
    }
    fn update_stamp(&self, state: &mut TileDrawStateGuardMut) {
        let page = self.page.unwrap_or_default();
        state.update_stamp(
            Some(self.content.clone()),
            self.content.get_tile_set(),
            |p| {
                self.content
                    .get_stamp_element(ResourceTilePosition::new(self.stage(), page, p))
            },
        );
    }
    /// After the data changes in the tile set or the brush that the widget is displaying,
    /// call this method to rebuild the stamp from the currently selected tiles. This is necessary
    /// since the tiles in those positions may have changed.
    pub fn sync_selection_to_model(&mut self) {
        let mut state = self.state.lock_mut("sync_selection_to_model");
        self.selecting_tiles.clear();
        let stamp_trans = state.stamp.transformation();
        self.update_stamp(&mut state);
        state.stamp.transform(stamp_trans);
    }
    fn update_selection(&mut self) {
        let MouseMode::Drawing { start_tile, end } = self.mode.clone() else {
            return;
        };
        let end_tile = end.grid;
        if self.kind == TilePaletteStage::Tiles && self.page.is_none() {
            return;
        }
        let mut state = self.state.lock_mut("update_selection");
        state.set_palette(self.handle);
        let positions = state.selection_positions_mut();
        positions.clone_from(&self.selecting_tiles);
        let rect = TileRect::from_points(start_tile, end_tile);
        if self.selecting_tiles.contains(&start_tile) {
            positions.retain(|p| !rect.contains(*p))
        } else {
            positions.extend(rect.iter());
        }
        self.update_stamp(&mut state);
    }
    fn finalize_selection(&mut self, ui: &mut UserInterface) {
        let MouseMode::Drawing { end, .. } = self.mode.clone() else {
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
        }
        let mut state = self.state.lock_mut("finalize_selection");
        state.tile_set = self.content.get_tile_set();
        state.set_palette(self.handle);
        let positions = state.selection_positions();
        self.selecting_tiles.clone_from(positions);
        self.update_stamp(&mut state);
    }
    fn select_all(&mut self) {
        let Some(page) = self.page else {
            return;
        };
        let mut state = self.state.lock_mut("select_all");
        let results = match self.stage() {
            TilePaletteStage::Tiles => self.content.get_all_tile_positions(page),
            TilePaletteStage::Pages => self.content.get_all_page_positions(),
        };
        state.tile_set = self.content.get_tile_set();
        state.set_palette(self.handle);
        let sel = state.selection_positions_mut();
        sel.clear();
        sel.extend(results.iter().copied());
        self.update_stamp(&mut state);
    }
    fn select_one(&mut self, position: Vector2<i32>) {
        if self.page.is_none() {
            return;
        }
        let mut state = self.state.lock_mut("select_one");
        state.tile_set = self.content.get_tile_set();
        state.set_palette(self.handle);
        let sel = state.selection_positions_mut();
        sel.clear();
        sel.insert(position);
        self.update_stamp(&mut state);
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
        self.overlay.movable_position = pos.grid;
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
            mode => {
                if let MouseMode::Drawing { start_tile, end } = self.mode.clone() {
                    if end.grid != pos.grid || self.slice_mode && end.subgrid != pos.subgrid {
                        self.draw(mode, start_tile, pos.grid, pos.subgrid, ui);
                        self.mode = MouseMode::Drawing {
                            start_tile,
                            end: pos,
                        };
                    }
                }
            }
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

    fn begin_drag(&mut self, _pos: MousePos, _ui: &mut UserInterface) {
        let state = self.state.lock();
        if state.selection_palette() != self.handle {
            return;
        }
        let Some(page) = self.page else {
            return;
        };
        if self.kind == TilePaletteStage::Tiles && self.content.is_atlas_page(page) {
            return;
        }
        let tiles = state.selection_positions();
        self.overlay.movable_position = Vector2::default();
        self.overlay.erased_tiles.clear();
        self.overlay.movable_tiles.clear();
        for pos in tiles.iter() {
            let Some(data) = self
                .content
                .get_tile_render_data(ResourceTilePosition::new(self.kind, page, *pos))
            else {
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
        let mut state = self.state.lock_mut("end_drag");
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
            TileBook::Empty => (),
            TileBook::TileSet(tile_set) => {
                self.sender
                    .do_command(MoveTileSetPageCommand::new(tile_set, pages, offset));
            }
            TileBook::Brush(brush) => {
                let mut commands = CommandGroup::default();
                let from = pages.clone().into_boxed_slice();
                let to = pages.iter().map(|p| *p + offset).collect::<Box<_>>();
                if let Some(macro_list) = self.macro_list.as_ref() {
                    let macro_list = macro_list.lock();
                    for instance in brush.data_ref().macros.iter() {
                        if let Some(m) = macro_list.get_by_uuid(&instance.macro_id) {
                            if let Some(command) = m.move_pages(
                                from.clone(),
                                to.clone(),
                                &BrushMacroInstance {
                                    brush: brush.clone(),
                                    settings: instance.settings.clone(),
                                },
                            ) {
                                commands.push_command(command)
                            }
                        }
                    }
                }
                commands.push(MoveBrushPageCommand::new(brush, pages, offset));
                self.sender
                    .do_command(commands.with_custom_name("Move Brush Pages"));
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
        let tiles = state.selection_positions().iter().copied();
        match self.content.clone() {
            TileBook::Empty => (),
            TileBook::TileSet(tile_set) => {
                let data = tile_set.data_ref();
                let tiles = tiles
                    .filter(|p| data.has_tile_at(page, *p))
                    .collect::<Vec<_>>();
                drop(data);
                self.sender
                    .do_command(MoveTileSetTileCommand::new(tile_set, page, tiles, offset));
            }
            TileBook::Brush(brush) => {
                let mut commands = CommandGroup::default();
                let data = brush.data_ref();
                let tiles = tiles
                    .filter(|p| data.has_tile_at(page, *p))
                    .collect::<Vec<_>>();
                let from = tiles
                    .iter()
                    .filter_map(|p| TileDefinitionHandle::try_new(page, *p))
                    .collect::<Box<_>>();
                let to = tiles
                    .iter()
                    .filter_map(|p| TileDefinitionHandle::try_new(page, *p + offset))
                    .collect::<Box<_>>();
                if let Some(macro_list) = self.macro_list.as_ref() {
                    let macro_list = macro_list.lock();
                    for instance in data.macros.iter() {
                        if let Some(m) = macro_list.get_by_uuid(&instance.macro_id) {
                            if let Some(command) = m.move_cells(
                                from.clone(),
                                to.clone(),
                                &BrushMacroInstance {
                                    brush: brush.clone(),
                                    settings: instance.settings.clone(),
                                },
                            ) {
                                commands.push_command(command)
                            }
                        }
                    }
                }
                drop(data);
                commands.push(MoveBrushTileCommand::new(brush, page, tiles, offset));
                self.sender
                    .do_command(commands.with_custom_name("Move Brush Tiles"));
            }
        }
    }

    fn draw(
        &mut self,
        mode: DrawingMode,
        start: Vector2<i32>,
        end: Vector2<i32>,
        sub_pos: Vector2<usize>,
        _ui: &mut UserInterface,
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
            DrawingMode::Editor => {
                if let Some(editor) = &state.active_editor {
                    if let Some(handle) = TileDefinitionHandle::try_new(page, end) {
                        let editor = editor.lock();
                        editor.draw_tile(
                            handle,
                            sub_pos,
                            &state,
                            &mut self.tile_set_update,
                            &self.content,
                        );
                        editor.highlight(
                            &mut self.highlight,
                            page,
                            &self.content,
                            &self.tile_set_update,
                        );
                    }
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
            DrawingMode::Editor => self.send_tile_set_update(),
        }
    }
    fn accept_material_drop(&mut self, _material: MaterialResource, _ui: &UserInterface) {
        // TODO: Allow users to drag-and-drop materials into a palette to create
        // tiles or atlas pages.
    }
    fn push_tile_collider(
        &self,
        position: Vector2<i32>,
        transformation: OrthoTransformation,
        collider: &TileCollider,
        ctx: &mut DrawingContext,
    ) {
        let mut tris = self.collider_triangles.borrow_mut();
        let (vertices, triangles) = &mut *tris;
        let s = self.tile_size;
        let scaling = Matrix4::<f32>::new_nonuniform_scaling(&Vector3::new(s.x, s.y, 1.0));
        let transform = if transformation.is_identity() {
            scaling
        } else {
            let center = Vector3::new(0.5 + position.x as f32, 0.5 + position.y as f32, 0.0);
            let matrix = transformation.matrix().to_homogeneous().to_homogeneous();
            scaling
                * Matrix4::new_translation(&center)
                * matrix
                * Matrix4::new_translation(&-center)
        };
        let position = position.cast::<f32>().to_homogeneous();
        collider.build_collider_shape(&transform, position, vertices, triangles);
        let origin = ctx.last_vertex_index();
        for v in vertices.iter() {
            ctx.push_vertex(v.coords, Vector2::new(0.0, 0.0));
        }
        for [a, b, c] in triangles.iter().map(|tri| tri.map(|i| i + origin)) {
            ctx.push_triangle(a, b, c);
        }
        vertices.clear();
        triangles.clear();
    }
    fn draw_tile_colliders(&self, page: Vector2<i32>, ctx: &mut DrawingContext) {
        let is_overlay_visible = self.is_overlay_visible();
        let mut current_color = None;
        for highlight in self.colliders.iter() {
            let pos = highlight.position;
            let Some(handle) = TileDefinitionHandle::try_new(page, pos) else {
                continue;
            };
            if is_overlay_visible && self.overlay.covers(pos) {
                continue;
            }
            if self.update.contains_key(&pos) || self.tile_set_update.contains_key(&handle) {
                continue;
            }
            if let Some(cur) = current_color {
                if cur != highlight.color {
                    self.commit_color(cur, ctx);
                    current_color = Some(highlight.color);
                }
            } else {
                current_color = Some(highlight.color);
            }
            self.push_tile_collider(
                highlight.position,
                OrthoTransformation::identity(),
                &highlight.tile_collider,
                ctx,
            );
        }
        if let Some(cur) = current_color {
            self.commit_color(cur, ctx);
        }
        let Some(tile_set) = self.content.get_tile_set() else {
            return;
        };
        let tile_set = tile_set.data_ref();
        for uuid in self.state.lock().visible_colliders.iter() {
            for (pos, handle) in self.update.iter() {
                let Some((trans, handle)) = handle.as_ref().map(|h| h.pair()) else {
                    continue;
                };
                let Some(color) = tile_set.collider_color(*uuid) else {
                    continue;
                };
                let tile_collider = tile_set.tile_collider(handle, *uuid);
                if tile_collider.is_none() {
                    continue;
                }
                self.push_tile_collider(*pos, trans, tile_collider, ctx);
                self.commit_color(color, ctx);
            }
        }
        if self.tile_set_update.is_empty() {
            return;
        }
        for uuid in self.state.lock().visible_colliders.iter() {
            for (handle, value) in self.tile_set_update.iter() {
                let Some(handle) = value.substitute_transform_handle(*handle) else {
                    continue;
                };
                let Some(color) = tile_set.collider_color(*uuid) else {
                    continue;
                };
                let tile_collider = value
                    .get_tile_collider(uuid)
                    .unwrap_or_else(|| tile_set.tile_collider(handle, *uuid));
                self.push_tile_collider(
                    handle.tile(),
                    OrthoTransformation::identity(),
                    tile_collider,
                    ctx,
                );
                self.commit_color(color, ctx);
            }
        }
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
        let rect = rect.deflate(2.0, 2.0);
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
        let rect = rect.deflate(2.0, 2.0);
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
    fn draw_animation_bookends(&self, ctx: &mut DrawingContext) {
        let TileBook::TileSet(tile_set) = &self.content else {
            return;
        };
        let Some(page) = self.page else {
            return;
        };
        let mut tile_set = tile_set.state();
        let Some(page) = tile_set.data().and_then(|t| t.pages.get(&page)) else {
            return;
        };
        let TileSetPageSource::Animation(tiles) = &page.source else {
            return;
        };
        for pos in tiles.keys() {
            let left = Vector2::new(pos.x - 1, pos.y);
            let right = Vector2::new(pos.x + 1, pos.y);
            if !tiles.contains_key(&left) {
                self.push_left_bookend(left, ctx);
            }
            if !tiles.contains_key(&right) {
                self.push_right_bookend(right, ctx);
            }
        }
        self.commit_color(ANIMATION_BOOKEND_COLOR, ctx);
    }
    fn push_left_bookend(&self, position: Vector2<i32>, ctx: &mut DrawingContext) {
        let t = self.tile_size;
        let p = position.cast::<f32>();
        let offset = Vector2::new(p.x * t.x, p.y * t.y);
        let vertices = [(0.6, 0.5), (0.9, 0.0), (0.9, 1.0)]
            .map(|(x, y)| Vector2::new(x * t.x, y * t.y) + offset);
        ctx.push_triangle_filled(vertices);
    }
    fn push_right_bookend(&self, position: Vector2<i32>, ctx: &mut DrawingContext) {
        let t = self.tile_size;
        let p = position.cast::<f32>();
        let offset = Vector2::new(p.x * t.x, p.y * t.y);
        let vertices = [(0.4, 0.5), (0.1, 0.0), (0.1, 1.0)]
            .map(|(x, y)| Vector2::new(x * t.x, y * t.y) + offset);
        ctx.push_triangle_filled(vertices);
    }
    fn draw_material_background(&self, ctx: &mut DrawingContext) {
        if self.kind != TilePaletteStage::Tiles || !self.editable {
            return;
        }
        let TileBook::TileSet(tile_set) = &self.content else {
            return;
        };
        let Some(page) = self.page else {
            return;
        };
        let mut tile_set = tile_set.state();
        let Some(page) = tile_set.data().and_then(|t| t.pages.get(&page)) else {
            return;
        };
        let TileSetPageSource::Atlas(mat) = &page.source else {
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
            CommandTexture::Texture(tex),
            None,
        );
        ctx.transform_stack.pop();
    }
    fn is_overlay_visible(&self) -> bool {
        let drawing_mode = if self.editable {
            self.state.lock().drawing_mode
        } else {
            DrawingMode::Pick
        };
        match drawing_mode {
            DrawingMode::Draw => self.is_mouse_directly_over && self.mode == MouseMode::None,
            DrawingMode::Erase => self.is_mouse_directly_over,
            DrawingMode::FloodFill => self.is_mouse_directly_over,
            DrawingMode::Pick => matches!(self.mode, MouseMode::Dragging { .. }),
            DrawingMode::RectFill => true,
            DrawingMode::NineSlice => true,
            DrawingMode::Line => true,
            DrawingMode::Editor => false,
        }
    }
    fn draw_no_page(&self, ctx: &mut DrawingContext) {
        let bounds = self.bounding_rect();
        ctx.push_rect_filled(&bounds, None);
        self.commit_color(NO_PAGE_COLOR, ctx);
        let transform = self.tile_to_local();
        let inv_transform = invert_transform(&transform);
        let bounds = bounds.transform(&inv_transform);
        ctx.transform_stack
            .push(self.visual_transform() * transform);
        ctx.push_grid(self.zoom, self.tile_size, bounds);
        self.commit_color(Color::DIM_GRAY, ctx);
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
    }
}

impl Control for PaletteWidget {
    fn draw(&self, ctx: &mut DrawingContext) {
        let page = if let Some(page) = self.page {
            page
        } else if self.kind == TilePaletteStage::Pages {
            Vector2::new(0, 0)
        } else {
            self.draw_no_page(ctx);
            return;
        };
        if self.kind == TilePaletteStage::Tiles && !self.content.has_page_at(page) {
            self.draw_no_page(ctx);
            return;
        }
        let bounds = self.bounding_rect();
        ctx.push_rect_filled(&bounds, None);
        ctx.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );
        let transform = self.tile_to_local();
        let inv_transform = invert_transform(&transform);
        let bounds = bounds.transform(&inv_transform);
        ctx.transform_stack
            .push(self.visual_transform() * transform);

        self.draw_material_background(ctx);

        let stage = self.stage();
        let is_overlay_visible = self.is_overlay_visible();
        self.content.tile_render_loop(stage, page, |pos, data| {
            if is_overlay_visible && self.overlay.covers(pos) {
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

        if let Some(tile_set) = self.state.lock().tile_set.as_ref() {
            let mut tile_set = tile_set.state();
            if let Some(tile_set) = tile_set.data() {
                for (pos, v) in self.update.iter() {
                    let Some((t, h)) = v.as_ref().map(|v| v.pair()) else {
                        continue;
                    };
                    let Some(data) = tile_set.get_transformed_render_data(t, h) else {
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
                        .get_tile_render_data(handle.into())
                        .unwrap_or_else(TileRenderData::missing_data);
                    let Some(data) = v.modify_render(&data) else {
                        continue;
                    };
                    let t = self.tile_size;
                    let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
                    let rect = Rect { position, size: t };
                    draw_tile(rect, self.clip_bounds(), &data, ctx);
                }
            }
        }
        if is_overlay_visible {
            for (pos, data) in self.overlay.iter() {
                let t = self.tile_size;
                let position = Vector2::new(pos.x as f32 * t.x, pos.y as f32 * t.y);
                let rect = Rect { position, size: t };
                draw_tile(rect, self.clip_bounds(), data, ctx);
            }
        }

        if self.kind == TilePaletteStage::Tiles {
            self.draw_tile_colliders(page, ctx);
        }

        ctx.push_grid(self.zoom, self.tile_size, bounds);
        self.commit_color(Color::BLACK, ctx);

        // Transform areas
        if stage == TilePaletteStage::Tiles && self.content.is_transform_page(page) {
            let area_size = Vector2::new(self.tile_size.x * 4.0, self.tile_size.y * 2.0);
            ctx.push_grid(self.zoom, area_size, bounds);
            self.commit_color(Color::ORANGE, ctx);
        }
        if stage == TilePaletteStage::Tiles && self.content.is_animation_page(page) {
            self.draw_animation_bookends(ctx);
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
            self.commit_color(CURSOR_HIGHLIGHT_COLOR, ctx);
        }

        if self.editable && self.state.lock().drawing_mode == DrawingMode::Erase {
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
        if stage == TilePaletteStage::Tiles {
            if let Some(page) = self.page {
                if let Some(cell_set_list) = &self.macro_cells {
                    if let Some(cells_on_page) = cell_set_list.lock().cells_on_page(page) {
                        for position in cells_on_page.iter() {
                            self.push_cell_rect(*position, line_thickness * 6.0, ctx);
                        }
                        self.commit_color(MACRO_CELL_HIGHLIGHT_COLOR, ctx);
                    }
                }
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
            self.current_tool = self.state.lock().drawing_mode;
            if *button == MouseButton::Middle {
                self.mode = MouseMode::Panning {
                    initial_view_position: self.view_position,
                    click_position: *pos,
                };
            } else if *button == MouseButton::Left && !message.handled() {
                ui.send_message(DelayedMessage::message(
                    MOUSE_CLICK_DELAY_FRAMES,
                    PaletteMessage::begin_motion(self.handle(), MessageDirection::ToWidget, *pos),
                ));
            }
        } else if let Some(WidgetMessage::MouseUp { pos, button, .. }) = message.data() {
            ui.release_mouse_capture();
            if *button == MouseButton::Left {
                let mouse_pos = self.calc_mouse_position(*pos);
                self.end_motion(self.drawing_mode(), mouse_pos, ui);
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
            self.continue_motion(self.drawing_mode(), mouse_pos, ui);
        } else if let Some(WidgetMessage::MouseLeave) = message.data() {
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
                    PaletteMessage::Center(v) => {
                        let s = self.tile_size;
                        let s = Vector2::new(s.x * -self.zoom, s.y * self.zoom);
                        self.view_position = Vector2::new(v.x as f32 * s.x, v.y as f32 * s.y);
                    }
                    PaletteMessage::SelectAll => self.select_all(),
                    PaletteMessage::SelectOne(v) => self.select_one(*v),
                    PaletteMessage::Delete => drop(self.delete_tiles(ui)),
                    PaletteMessage::MaterialColor(color) => self.material_color = *color,
                    PaletteMessage::SyncToState => self.sync_to_state(ui),
                    PaletteMessage::BeginMotion(pos) => {
                        let mouse_pos = self.calc_mouse_position(*pos);
                        self.begin_motion(self.drawing_mode(), mouse_pos.clone(), ui);
                        if self.handle() != ui.captured_node() {
                            self.end_motion(self.drawing_mode(), mouse_pos, ui);
                            self.mode = MouseMode::None;
                        }
                    }
                }
            }
        } else if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            if *key == KeyCode::Delete && !message.handled() && self.delete_tiles(ui) {
                message.set_handled(true);
            }
        }
    }
}

/// Builder for [`PaletteWidget`]
pub struct PaletteWidgetBuilder {
    widget_builder: WidgetBuilder,
    tile_book: TileBook,
    page: Option<Vector2<i32>>,
    sender: MessageSender,
    state: TileDrawStateRef,
    macro_cells: Option<MacroCellSetListRef>,
    macro_list: Option<BrushMacroListRef>,
    kind: TilePaletteStage,
    editable: bool,
}

impl PaletteWidgetBuilder {
    /// Build a [`PaletteWidget`] with the given sender and [`TileDrawStateRef`].
    /// The state is a shared reference that the palette will keep for its lifetime so that
    /// it can cooperate with other palettes and with the tile map interaction mode.
    pub fn new(
        widget_builder: WidgetBuilder,
        sender: MessageSender,
        state: TileDrawStateRef,
    ) -> Self {
        Self {
            widget_builder,
            tile_book: TileBook::Empty,
            sender,
            state,
            macro_cells: None,
            macro_list: None,
            kind: TilePaletteStage::default(),
            editable: false,
            page: None,
        }
    }

    /// The coordinates of the page to display. The default is None.
    pub fn with_page(mut self, page: Vector2<i32>) -> Self {
        self.page = Some(page);
        self
    }

    /// The resource to display in the form of a [`TileBook`] that may be either
    /// a [`TileMapBrush`](fyrox::scene::tilemap::brush::TileMapBrush) resource or a [`TileSet`] resource.
    pub fn with_resource(mut self, tile_book: TileBook) -> Self {
        self.tile_book = tile_book;
        self
    }

    /// Each brush and tile set has two palette areas: the pages and the tiles within each page.
    /// These two areas are called stages, and each of the two stages needs to be handled separately.
    /// Giving a particular `TilePaletteStage` to a tile map palette will control which kind of
    /// tiles it will display.
    pub fn with_kind(mut self, kind: TilePaletteStage) -> Self {
        self.kind = kind;
        self
    }

    /// Some palettes are editable while others exist only so the user can select tiles.
    /// The default is `false` which indicates that the palette is not for editing.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Giving the palette access to a list of macro cells allows it to draw outlines
    /// around cells that are involved in some macros.
    pub fn with_macro_cells(mut self, macro_cells: MacroCellSetListRef) -> Self {
        self.macro_cells = Some(macro_cells);
        self
    }

    /// Giving the palette access to a list of macro cells allows it to draw outlines
    /// around cells that are involved in some macros.
    pub fn with_macro_list(mut self, macro_list: BrushMacroListRef) -> Self {
        self.macro_list = Some(macro_list);
        self
    }

    /// Build the [`PaletteWidget`].
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(PaletteWidget {
            widget: self
                .widget_builder
                .with_allow_drop(true)
                .with_clip_to_bounds(false)
                .build(ctx),
            sender: self.sender,
            state: self.state,
            macro_cells: self.macro_cells,
            macro_list: self.macro_list,
            overlay: PaletteOverlay::default(),
            content: self.tile_book,
            kind: self.kind,
            editable: self.editable,
            material_color: DEFAULT_MATERIAL_COLOR,
            page: self.page,
            cursor_position: None,
            current_tool: DrawingMode::Pick,
            slice_position: Vector2::default(),
            slice_mode: false,
            position_text: FormattedTextBuilder::new(ctx.inner().default_font.clone())
                .with_brush(Brush::Solid(Color::WHITE))
                .build(),
            selecting_tiles: FxHashSet::default(),
            highlight: FxHashMap::default(),
            colliders: Vec::default(),
            update: TransTilesUpdate::default(),
            tile_set_update: TileSetUpdate::default(),
            view_position: Default::default(),
            zoom: 1.0,
            tile_size: Vector2::repeat(32.0),
            mode: MouseMode::None,
            collider_triangles: RefCell::default(),
        }))
    }
}

const CHECKERSIZE: f32 = 10.0;

/// Draw a checkerboard pattern into the given drawing context for the purpose of visualizing
/// the transparency of whatever is drawn on top of the checkerboard.
pub fn draw_checker_board(bounds: Rect<f32>, clip_bounds: Rect<f32>, ctx: &mut DrawingContext) {
    let transform = ctx.transform_stack.transform();
    let bounds = bounds.transform(transform);
    let Some(clip_bounds) = *clip_bounds.clip_by(bounds) else {
        return;
    };
    ctx.transform_stack.push(Matrix3::identity());
    let start = bounds.left_top_corner() / CHECKERSIZE;
    let end = bounds.right_bottom_corner() / CHECKERSIZE;
    let start = Vector2::new(start.x.floor() as i64, start.y.floor() as i64);
    let end = Vector2::new(end.x.ceil() as i64, end.y.ceil() as i64);
    for y in start.y..end.y {
        for x in start.x..end.x {
            let rect = Rect::new(
                x as f32 * CHECKERSIZE,
                y as f32 * CHECKERSIZE,
                CHECKERSIZE,
                CHECKERSIZE,
            );
            let color = if (x + y) & 1 == 0 {
                Color::opaque(127, 127, 127)
            } else {
                Color::WHITE
            };
            ctx.push_rect_multicolor(&rect, [color; 4]);
        }
    }
    ctx.commit(
        clip_bounds,
        Brush::Solid(Color::WHITE),
        CommandTexture::None,
        None,
    );
    ctx.transform_stack.pop();
}

fn draw_empty_tile(position: Rect<f32>, clip_bounds: Rect<f32>, ctx: &mut DrawingContext) {
    let Some(image) = ERASER_IMAGE.clone() else {
        return;
    };
    ctx.push_rect_filled(
        &position,
        Some(&[
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
        ]),
    );
    ctx.commit(
        clip_bounds,
        Brush::Solid(Color::WHITE),
        CommandTexture::Texture(image),
        None,
    );
}

fn draw_tile(
    position: Rect<f32>,
    clip_bounds: Rect<f32>,
    tile: &TileRenderData,
    ctx: &mut DrawingContext,
) {
    if tile.is_empty() {
        draw_empty_tile(position, clip_bounds, ctx);
        return;
    }
    draw_checker_board(position, clip_bounds, ctx);
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
                ctx.push_rect_filled(
                    &position,
                    Some(&[
                        bounds.left_bottom_uv(size),
                        bounds.right_bottom_uv(size),
                        bounds.right_top_uv(size),
                        bounds.left_top_uv(size),
                    ]),
                );
                ctx.commit(
                    clip_bounds,
                    Brush::Solid(color),
                    CommandTexture::Texture(texture),
                    None,
                );
            }
        } else {
            ctx.push_rect_filled(&position, None);
            ctx.commit(clip_bounds, Brush::Solid(color), CommandTexture::None, None);
        }
    } else {
        ctx.push_rect_filled(&position, None);
        ctx.commit(clip_bounds, Brush::Solid(color), CommandTexture::None, None);
    }
}
