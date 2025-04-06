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

//! The editor plugin for editing tile maps, tile sets, and tile map brushes.

#![allow(clippy::collapsible_match)] // STFU
#![warn(missing_docs)]

mod autotile;
mod brush_macro;
mod collider_editor;
mod colliders_tab;
mod commands;
mod handle_editor;
mod handle_field;
mod interaction_mode;
mod macro_inspector;
mod macro_tab;
mod misc;
pub mod palette;
pub mod panel;
mod panel_preview;
mod preview;
mod properties_tab;
mod tile_bounds_editor;
mod tile_editor;
mod tile_inspector;
mod tile_prop_editor;
pub mod tileset;
mod wfc;

use autotile::*;
pub use brush_macro::*;
use collider_editor::*;
use colliders_tab::*;
use fyrox::core::futures::executor::block_on;
use fyrox::core::log::Log;
use fyrox::fxhash::FxHashMap;
use fyrox::gui::grid::{Column, GridBuilder, Row};
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use fyrox::gui::text::TextBuilder;
use fyrox::gui::{message::KeyCode, texture::TextureResource};
use fyrox::gui::{HorizontalAlignment, VerticalAlignment};
use fyrox::scene::tilemap::tileset::{TileSetPropertyLayer, ELEMENT_MATCH_HIGHLIGHT_COLOR};
use fyrox::scene::tilemap::{StampElement, TileMapEffectRef};
pub use handle_editor::*;
use handle_field::*;
use interaction_mode::*;
use macro_inspector::*;
use palette::PaletteWidget;
use panel::TileMapPanel;
use panel_preview::*;
use properties_tab::*;
use tile_bounds_editor::*;
use tile_editor::*;
use tile_inspector::*;
use tile_prop_editor::*;
use wfc::*;

use crate::fyrox::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        math::{plane::Plane, Matrix4Ext},
        parking_lot::{Mutex, MutexGuard},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
        Uuid,
    },
    engine::Engine,
    fxhash::FxHashSet,
    graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        image::ImageBuilder,
        key::HotKey,
        message::{MessageDirection, UiMessage},
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, Thickness, UiNode, UserInterface,
    },
    scene::{
        debug::Line,
        node::Node,
        tilemap::{
            tileset::{TileSet, TileSetResource},
            RandomTileSource, Stamp, TileBook, TileCollider, TileDefinitionHandle, TileMap,
            TilePaletteStage,
        },
        Scene,
    },
};
use crate::{
    interaction::{make_interaction_mode_button, InteractionMode},
    load_image,
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::tilemap::{palette::PaletteMessage, preview::TileSetPreview, tileset::TileSetEditor},
    scene::{controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
use fyrox::asset::manager::ResourceManager;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

lazy_static! {
    static ref VISIBLE_IMAGE: Option<TextureResource> =
        load_image!("../../../resources/visible.png");
    static ref BRUSH_IMAGE: Option<TextureResource> = load_image!("../../../resources/brush.png");
    static ref ERASER_IMAGE: Option<TextureResource> = load_image!("../../../resources/eraser.png");
    static ref FILL_IMAGE: Option<TextureResource> = load_image!("../../../resources/fill.png");
    static ref PICK_IMAGE: Option<TextureResource> = load_image!("../../../resources/pipette.png");
    static ref RECT_FILL_IMAGE: Option<TextureResource> =
        load_image!("../../../resources/rect_fill.png");
    static ref NINE_SLICE_IMAGE: Option<TextureResource> =
        load_image!("../../../resources/nine_slice.png");
    static ref LINE_IMAGE: Option<TextureResource> = load_image!("../../../resources/line.png");
    static ref TURN_LEFT_IMAGE: Option<TextureResource> =
        load_image!("../../../resources/turn_left.png");
    static ref TURN_RIGHT_IMAGE: Option<TextureResource> =
        load_image!("../../../resources/turn_right.png");
    static ref FLIP_X_IMAGE: Option<TextureResource> = load_image!("../../../resources/flip_x.png");
    static ref FLIP_Y_IMAGE: Option<TextureResource> = load_image!("../../../resources/flip_y.png");
    static ref RANDOM_IMAGE: Option<TextureResource> = load_image!("../../../resources/die.png");
    static ref PALETTE_IMAGE: Option<TextureResource> =
        load_image!("../../../resources/palette.png");
}

/// A structure to keep track of which cells of a tile map brush are involved in macros.
/// Each macro is expected to keep track of which cells it is using, but this information
/// is also duplicated here for easier access.
#[derive(Default, Debug, Clone)]
pub struct MacroCellSetList {
    /// This list has one entry for each macro instance in the current brush.
    /// Each entry is the set of cells in the brush that are used by the instance.
    content: Vec<FxHashSet<TileDefinitionHandle>>,
    /// A map from brush pages to sets of cells within the page that are used by
    /// some macro.
    cells_by_page: FxHashMap<Vector2<i32>, FxHashSet<Vector2<i32>>>,
}

impl Deref for MacroCellSetList {
    type Target = Vec<FxHashSet<TileDefinitionHandle>>;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl DerefMut for MacroCellSetList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

impl MacroCellSetList {
    /// Remove all cells from the list.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cells_by_page.clear();
    }
    /// True if the given cell is being used by a macro.
    pub fn cell_has_any_macro(&self, handle: TileDefinitionHandle) -> bool {
        self.content.iter().any(|s| s.contains(&handle))
    }
    /// True if the given cell is being used by the macro at the given index.
    pub fn cell_has_macro(&self, handle: TileDefinitionHandle, index: usize) -> bool {
        self.content
            .get(index)
            .map(|s| s.contains(&handle))
            .unwrap_or_default()
    }
    /// The set of cells that are being used by any macro on the given page.
    pub fn cells_on_page(&self, page: Vector2<i32>) -> Option<&FxHashSet<Vector2<i32>>> {
        self.cells_by_page.get(&page)
    }
    /// Perform some final calculations after all cells have been added to the list.
    pub fn finalize(&mut self) {
        self.cells_by_page.clear();
        for set in self.content.iter() {
            for handle in set.iter() {
                _ = self
                    .cells_by_page
                    .entry(handle.page())
                    .or_default()
                    .insert(handle.tile());
            }
        }
    }
}

fn make_drawing_mode_button(
    ctx: &mut BuildContext,
    width: f32,
    height: f32,
    image: Option<TextureResource>,
    tooltip: &str,
    tab_index: Option<usize>,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
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
                .with_width(width)
                .with_height(height),
        )
        .with_opt_texture(image)
        .build(ctx),
    )
    .build(ctx)
}

/// The possible drawing mode when the user is editing tiles.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Visit, Reflect)]
pub enum DrawingMode {
    /// Paste the currently selected tiles as a stamp wherever the user clicks or drags the mouse.
    #[default]
    Draw,
    /// Erase tiles in the shape of the currently selected tiles wherever the user clicks or drags the mouse,
    /// or erase a single cell if no tiles are selected.
    Erase,
    /// Flood a the cells of a tile map, replacing regions of identical cell. This operation is only possible on a tile map.
    /// For tile set and brush editing, this operation behaves exactly like [`DrawingMode::Draw`].
    FloodFill,
    /// Select whatever tiles the user drags the mouse over, creating a rect of selected tiles. Hold shift to select multiple rects.
    Pick,
    /// Drag the mouse to create a rect filled with the currently selected tiles.
    RectFill,
    /// Drag the mouse to create a rect filled with the currently selected tiles, with special consideration
    /// taken for the sides, corners, and center of the selected tiles, so the selection is divided into nine areas
    /// before it fills the rect.
    NineSlice,
    /// Drag the mouse to draw a line with the currently selected tiles.
    Line,
    /// Use the currently active tile set editor field to modify the data of tiles in a tile set.
    /// This does nothing to tile maps or brushes.
    Editor,
}

#[derive(Debug, PartialEq, Clone)]
struct OpenTilePanelMessage {
    resource: TileBook,
    center: Option<TileDefinitionHandle>,
}

impl OpenTilePanelMessage {
    fn message(resource: TileBook, center: Option<TileDefinitionHandle>) -> UiMessage {
        UiMessage::with_data(Self { resource, center })
    }
}

/// This allows a UI message to be stored for a certain number of frames and then sent.
#[derive(Debug, PartialEq, Clone)]
struct DelayedMessage {
    delay_frames: usize,
    content: UiMessage,
}

impl DelayedMessage {
    fn message(delay_frames: usize, content: UiMessage) -> UiMessage {
        UiMessage::with_data(Self {
            delay_frames,
            content,
        })
    }
}

/// The editor plugin for editing tile maps, tile sets, and tile map brushes.
#[derive(Default)]
pub struct TileMapEditorPlugin {
    /// The state that is shared to allow this plugin to coordinate with the tile map
    /// interaction mode, the tile map control panel, and the tile set editor.
    state: TileDrawStateRef,
    /// List if tools that can be added to a tile map brush to assist with tile map editing.
    brush_macro_list: BrushMacroListRef,
    /// The tile set editor, if it is open.
    tile_set_editor: Option<TileSetEditor>,
    /// The tile map control panel, if it is open. The control panel allows the user
    /// to select editing tools like rect fill, erase, pick, and it allows the user
    /// to select tiles to draw with. It is centeral to editing a tile map.
    panel: Option<TileMapPanel>,
    /// The currently selected tile map, or NONE.
    tile_map: Handle<Node>,
    /// The plugin provides a service where it holds onto some messages and sends them
    /// in the next frame.
    delayed_messages: Vec<DelayedMessage>,
}

/// This is the state that is shared between the plugin, the palette widgets, the interaction mode,
/// and the control panel, so that they can all be synchronized with whatever editing operation the
/// user is currently performing.
#[derive(Default, Clone, Visit)]
pub struct TileDrawState {
    /// True if the state has been changed and the change has not yet caused the UI to update.
    dirty: bool,
    /// The tile set that contains the definitions of the tiles that are being edited.
    tile_set: Option<TileSetResource>,
    /// The current stamp that the user uses when drawing tiles to a tile set, brush, or tile map.
    /// The stamp acts as a TileSource for drawing operations and it is rendered in the [`PanelPreview`] widget.
    stamp: Stamp,
    /// The tool that the user has selected for editing tiles: Draw, Pick, Rectangle, Fill, etc.
    drawing_mode: DrawingMode,
    /// If the user is editing a tile set by drawing an editor value, then this is the editor.
    #[visit(skip)]
    active_editor: Option<TileEditorRef>,
    /// The UUIDs of the colliders that are currently visible to the user.
    #[visit(skip)]
    visible_colliders: FxHashSet<Uuid>,
    /// Does the user want tiles to be randomized?
    random_mode: bool,
    /// The currently selected tiles.
    selection: TileDrawSelection,
}

impl Debug for TileDrawState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileDrawState")
            .field("dirty", &self.dirty)
            .field("tile_set", &self.tile_set)
            .field("stamp", &self.stamp)
            .field("drawing_mode", &self.drawing_mode)
            .field("random_mode", &self.random_mode)
            .field("selection", &self.selection)
            .finish()
    }
}

type TileEditorRef = Arc<Mutex<dyn TileEditor>>;

/// An Arc Mutex of the shared [`TileDrawState`] state that can be cloned and given
/// to the various objects that need it. It provides methods to access the state
/// and automatically takes care of marking the state as dirty when it is accessed
/// as mutable.
#[derive(Debug, Default, Clone)]
pub struct TileDrawStateRef(Arc<Mutex<TileDrawState>>);
/// A guard object for locking and unlocking shared [`TileDrawState`] when it is
/// needed only for immutable access to the current state. It can be converted
/// to the mutable version if necessary.
pub struct TileDrawStateGuard<'a>(MutexGuard<'a, TileDrawState>);
/// A guard object for locking and unlocking shared [`TileDrawState`] when it is
/// needed for mutable access to the current state. The state is automatically
/// marked as dirty when this is created.
pub struct TileDrawStateGuardMut<'a>(MutexGuard<'a, TileDrawState>);

impl Deref for TileDrawStateGuard<'_> {
    type Target = TileDrawState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for TileDrawStateGuardMut<'_> {
    type Target = TileDrawState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TileDrawStateGuardMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

const STATE_UPDATE_DEBUG: bool = false;

impl TileDrawStateRef {
    /// Access the state immutably.
    pub fn lock(&self) -> TileDrawStateGuard {
        TileDrawStateGuard(self.0.try_lock().expect("State lock failed."))
    }
    /// Mutably access the state and mark the state as dirty so that everyone
    /// that is using the state will know to update themselves.
    pub fn lock_mut(&self, reason: &str) -> TileDrawStateGuardMut {
        self.lock().into_mut(reason)
    }
    /// Return true if the state has been modified, and reset
    /// the state to no longer being dirty.
    pub fn check_dirty(&self) -> bool {
        let mut state = self.0.lock();
        let dirty = state.dirty;
        state.dirty = false;
        dirty
    }
}

impl<'a> TileDrawStateGuard<'a> {
    /// Convert an immutable state into a mutable state, and mark the state as dirty.
    pub fn into_mut(self, reason: &str) -> TileDrawStateGuardMut<'a> {
        if STATE_UPDATE_DEBUG {
            println!("State Update: {reason}");
        }
        let mut result = TileDrawStateGuardMut(self.0);
        result.dirty = true;
        result
    }
}

impl<'a> TileDrawStateGuardMut<'a> {
    /// Convert an mutable state back to an immutable state.
    pub fn into_const(self) -> TileDrawStateGuard<'a> {
        TileDrawStateGuard(self.0)
    }
}

/// This represents the currently selected tiles, including the positions,
/// the page, and the widget or tile map where the selection occurred.
#[derive(Default, Debug, Clone, Visit)]
struct TileDrawSelection {
    /// The selection either comes from a [`PaletteWidget`] or a tile map node.
    /// This field allows each object to check if it its tiles are selected.
    pub source: SelectionSource,
    /// The page of the currently selected tiles.
    pub page: Vector2<i32>,
    /// The currently selected cells.
    pub positions: FxHashSet<Vector2<i32>>,
}

impl TileDrawState {
    /// True if the given editor is the active editor.
    #[inline]
    pub fn is_active_editor(&self, editor: &TileEditorRef) -> bool {
        if let Some(active) = &self.active_editor {
            Arc::ptr_eq(editor, active)
        } else {
            false
        }
    }
    /// Set whether the given collider is visible.
    pub fn set_visible_collider(&mut self, uuid: Uuid, visible: bool) {
        if visible {
            let _ = self.visible_colliders.insert(uuid);
        } else {
            let _ = self.visible_colliders.remove(&uuid);
        }
    }
    /// True if the current selection is not empty
    #[inline]
    pub fn has_selection(&self) -> bool {
        !self.selection.positions.is_empty()
    }
    /// The handle of the palette widget that is currently being used to select tiles, or else Handle::NONE.
    #[inline]
    pub fn selection_palette(&self) -> Handle<UiNode> {
        match self.selection.source {
            SelectionSource::Widget(h) => h,
            _ => Handle::NONE,
        }
    }
    /// The handle of the tile map node that is currently being used to select tiles, or else Handle::NONE.
    #[inline]
    pub fn selection_node(&self) -> Handle<Node> {
        match self.selection.source {
            SelectionSource::Node(h) => h,
            _ => Handle::NONE,
        }
    }
    /// Some [`PaletteWidget`] is being used to select tiles.
    #[inline]
    pub fn set_palette(&mut self, handle: Handle<UiNode>) {
        self.selection.source = SelectionSource::Widget(handle);
    }
    /// Some [`TileMap`] is being used to select tiles.
    #[inline]
    pub fn set_node(&mut self, handle: Handle<Node>) {
        self.selection.source = SelectionSource::Node(handle);
    }
    /// The positions of the currently selected tiles.
    #[inline]
    pub fn selection_positions(&self) -> &FxHashSet<Vector2<i32>> {
        &self.selection.positions
    }
    /// The positions of the currently selected tiles.
    #[inline]
    pub fn selection_positions_mut(&mut self) -> &mut FxHashSet<Vector2<i32>> {
        self.on_selection_changed();
        &mut self.selection.positions
    }
    /// Perform necessary cleanup when the selected tiles are changed.
    pub fn on_selection_changed(&mut self) {
        if self.drawing_mode == DrawingMode::Editor {
            self.drawing_mode = DrawingMode::Pick;
        }
    }
    /// Set selection to nothing.
    #[inline]
    pub fn clear_selection(&mut self) {
        self.stamp.clear();
        self.selection.positions.clear();
        self.selection.source = SelectionSource::None;
        self.on_selection_changed();
    }
    /// Update the stamp stored within this state to reflect the current selection
    /// and tile set. The given `tile_handle` function is used to determine the handles
    /// for each selection position.
    #[inline]
    pub fn update_stamp<F>(
        &mut self,
        book: Option<TileBook>,
        tile_set: Option<TileSetResource>,
        tile_handle: F,
    ) where
        F: Fn(Vector2<i32>) -> Option<StampElement>,
    {
        self.tile_set = tile_set;
        self.stamp.build(
            book,
            self.selection
                .positions
                .iter()
                .copied()
                .filter_map(|p| Some((p, tile_handle(p)?))),
        );
    }
}

/// An abstraction representing whatever object is currently being used to select tiles.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Visit)]
pub enum SelectionSource {
    /// There is no selection.
    #[default]
    None,
    /// A UI widget is selecting tiles.
    Widget(Handle<UiNode>),
    /// A tile map is selecting tiles.
    Node(Handle<Node>),
}

impl TileMapEditorPlugin {
    fn get_tile_map_mut<'a>(&self, editor: &'a mut Editor) -> Option<&'a mut TileMap> {
        let entry = editor.scenes.current_scene_entry_mut()?;
        let game_scene = entry.controller.downcast_mut::<GameScene>()?;
        let scene = &mut editor.engine.scenes[game_scene.scene];
        let node = scene.graph.try_get_mut(self.tile_map)?;
        node.component_mut::<TileMap>()
    }
    fn open_panel_for_tile_set(
        &mut self,
        resource: TileBook,
        center: Option<TileDefinitionHandle>,
        ui: &mut UserInterface,
        sender: &MessageSender,
        resource_manager: &ResourceManager,
    ) {
        if let Some(panel) = &mut self.panel {
            panel.to_top(ui);
        } else if let Some(editor) = &self.tile_set_editor {
            let panel = TileMapPanel::new(&mut ui.build_ctx(), self.state.clone(), sender.clone());
            panel.align(editor.window, ui);
            self.panel = Some(panel);
        }
        if let Some(panel) = &mut self.panel {
            panel.set_resource(resource, ui, resource_manager);
            if let Some(focus) = center {
                panel.set_focus(focus, ui);
            }
        }
    }
    fn open_panel_for_tile_map(&mut self, editor: &mut Editor) {
        let resource = if let Some(tile_map) = self.get_tile_map_mut(editor) {
            if let Some(brush) = tile_map.active_brush() {
                TileBook::Brush(brush.clone())
            } else if let Some(tile_set) = tile_map.tile_set() {
                TileBook::TileSet(tile_set.clone())
            } else {
                TileBook::Empty
            }
        } else {
            return;
        };

        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(panel) = &mut self.panel {
            panel.to_top(ui);
            panel.set_resource(resource, ui, &editor.engine.resource_manager);
        } else {
            let mut panel = TileMapPanel::new(
                &mut ui.build_ctx(),
                self.state.clone(),
                editor.message_sender.clone(),
            );
            panel.align(editor.scene_viewer.frame(), ui);
            panel.set_resource(resource, ui, &editor.engine.resource_manager);
            self.panel = Some(panel);
        }
    }
    fn update_state(&mut self) {
        let state = self.state.lock();
        if match state.drawing_mode {
            DrawingMode::Pick => false,
            DrawingMode::Editor => self.tile_set_editor.is_none(),
            _ => self.panel.is_none(),
        } {
            let mut state = state.into_mut("update_state");
            state.drawing_mode = DrawingMode::Pick;
            state.active_editor = None;
        } else if state.drawing_mode != DrawingMode::Editor && state.active_editor.is_some() {
            state
                .into_mut("update_state: drawing_mode != Editor")
                .active_editor = None;
        }
    }
    fn send_delayed_messages(&mut self, ui: &mut UserInterface) {
        let msgs = &mut self.delayed_messages;
        for dm in msgs.iter_mut() {
            dm.delay_frames = dm.delay_frames.saturating_sub(1);
        }
        let mut i = 0;
        while i < msgs.len() {
            if msgs[i].delay_frames == 0 {
                let m = msgs.swap_remove(i);
                ui.send_message(m.content);
            } else {
                i += 1;
            }
        }
    }
    fn on_tile_map_selected(&mut self, handle: Handle<Node>, editor: &mut Editor) {
        // Set the new tile map as the currently edited tile map.
        self.tile_map = handle;
        // Create new editor data and add it to the tile map, so the tile map node
        // will now render itself as being edited.
        let sender = editor.message_sender.clone();
        let Some(tile_map) = self.get_tile_map_mut(editor) else {
            return;
        };
        let mut interaction_mode = TileMapInteractionMode::new(
            handle,
            self.state.clone(),
            self.brush_macro_list.clone(),
            sender,
        );
        interaction_mode.on_tile_map_selected(tile_map);
        // Prepare the tile map interaction mode.
        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            // We have somehow lost the scene entry, so remove the effects from the tile map.
            if let Some(tile_map) = self.get_tile_map_mut(editor) {
                tile_map.before_effects.clear();
                tile_map.after_effects.clear();
            }
            return;
        };
        entry.interaction_modes.add(interaction_mode);
    }
}

impl EditorPlugin for TileMapEditorPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        editor
            .asset_browser
            .preview_generators
            .add(TileSet::type_uuid(), TileSetPreview);
        let state = editor.engine.resource_manager.state();
        state.constructors_container.add::<AutoTileInstance>();
        state.constructors_container.add::<WfcInstance>();
    }

    fn on_exit(&mut self, editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.try_save(&editor.engine.resource_manager);
        }
    }

    fn on_suspended(&mut self, _editor: &mut Editor) {}

    fn on_mode_changed(&mut self, editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.try_save(&editor.engine.resource_manager);
        }
    }

    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let palette = self.state.lock().selection_palette();
        if let Some(palette) = ui
            .try_get_mut(palette)
            .and_then(|p| p.cast_mut::<PaletteWidget>())
        {
            palette.sync_selection_to_model();
        }

        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.sync_to_model(ui);
        }
        if let Some(panel) = self.panel.as_mut() {
            panel.sync_to_model(ui, &editor.engine.resource_manager);
        }
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        if let Some(delayed_message) = message.data::<DelayedMessage>() {
            self.delayed_messages.push(delayed_message.clone());
            return;
        }

        if let Some(tile_set_editor) = self.tile_set_editor.take() {
            self.tile_set_editor = tile_set_editor.handle_ui_message(message, editor);
        }

        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(OpenTilePanelMessage { resource, center }) = message.data() {
            self.open_panel_for_tile_set(
                resource.clone(),
                *center,
                ui,
                &editor.message_sender,
                &editor.engine.resource_manager,
            );
        } else if let Some(&TileDefinitionHandleEditorMessage::Goto(handle)) = message.data() {
            if let Some(panel) = &mut self.panel {
                panel.set_focus(handle, ui);
            }
        }

        if let Some(panel) = self.panel.take() {
            self.panel = panel.handle_ui_message(message, ui, &editor.engine.resource_manager);
        }
    }

    fn on_update(&mut self, editor: &mut Editor) {
        self.send_delayed_messages(editor.engine.user_interfaces.first_mut());

        self.update_state();

        if self.state.check_dirty() {
            if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
                tile_set_editor.sync_to_state(editor.engine.user_interfaces.first_mut());
            }
            if let Some(panel) = self.panel.as_mut() {
                panel.sync_to_state(editor.engine.user_interfaces.first_mut());
            }
            if let Some(interaction_mode) = editor
                .scenes
                .current_scene_entry_mut()
                .and_then(|s| s.interaction_modes.of_type_mut::<TileMapInteractionMode>())
            {
                interaction_mode.sync_to_state();
            }
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let tile_book: Option<TileBook> = if let Message::OpenTileSetEditor(tile_set) = message {
            Log::verify(block_on(tile_set.clone()));
            tile_set
                .is_ok()
                .then(|| TileBook::TileSet(tile_set.clone()))
        } else if let Message::OpenTileMapBrushEditor(brush) = message {
            let brush = brush.clone();
            Log::verify(block_on(brush.clone()));
            if brush.is_ok() {
                let is_loaded = brush.data_ref().block_until_tile_set_is_loaded();
                is_loaded.then_some(TileBook::Brush(brush))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(tile_book) = tile_book {
            if self.tile_set_editor.is_none() {
                let mut tile_set_editor = TileSetEditor::new(
                    tile_book.clone(),
                    self.state.clone(),
                    self.brush_macro_list.clone(),
                    editor.message_sender.clone(),
                    editor.engine.resource_manager.clone(),
                    &mut ui.build_ctx(),
                );
                tile_set_editor.set_tile_resource(&editor.engine.resource_manager, tile_book, ui);
                self.tile_set_editor = Some(tile_set_editor);
            } else if let Some(tile_set_editor) = &mut self.tile_set_editor {
                tile_set_editor.set_tile_resource(
                    &editor.engine.resource_manager,
                    tile_book.clone(),
                    ui,
                );
            }
        }

        if let Message::SetInteractionMode(uuid) = message {
            if *uuid == TileMapInteractionMode::type_uuid() && self.panel.is_none() {
                if let Some(tile_map) = self.get_tile_map_mut(editor) {
                    let resource = if let Some(brush) = tile_map.active_brush() {
                        TileBook::Brush(brush.clone())
                    } else if let Some(tile_set) = tile_map.tile_set() {
                        TileBook::TileSet(tile_set.clone())
                    } else {
                        TileBook::Empty
                    };
                    if !resource.is_empty() {
                        self.open_panel_for_tile_map(editor);
                    }
                }
            }
        }

        let Some(entry) = editor.scenes.current_scene_entry_mut() else {
            return;
        };

        let Some(selection) = entry.selection.as_graph() else {
            return;
        };

        let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() else {
            return;
        };

        if let Message::SelectionChanged { .. } = message {
            let scene = &mut editor.engine.scenes[game_scene.scene];
            entry
                .interaction_modes
                .remove_typed::<TileMapInteractionMode>();

            // Remove the editor data from the currently selected tile map, so it will render as normal.
            if let Some(tile_map) = scene
                .graph
                .try_get_mut(self.tile_map)
                .and_then(|n| n.component_mut::<TileMap>())
            {
                tile_map.before_effects.clear();
                tile_map.after_effects.clear();
            }

            if let Some(handle) = selection
                .nodes()
                .iter()
                .copied()
                .find(|h| scene.graph.try_get_of_type::<TileMap>(*h).is_some())
            {
                self.on_tile_map_selected(handle, editor);
            }
        }
    }
}

/// Create one item for the dropdown list.
pub fn make_named_value_list_option(
    ctx: &mut BuildContext,
    color: Color,
    name: &str,
) -> Handle<UiNode> {
    let icon = BorderBuilder::new(
        WidgetBuilder::new()
            .on_column(0)
            .with_background(Brush::Solid(color).into()),
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
            .with_corner_radius((4.0).into())
            .with_pad_by_corner_radius(false),
    )
    .build(ctx)
}

/// Create the items for the dropdown list list that lets the user select a named value.
pub fn make_named_value_list_items(
    layer: &TileSetPropertyLayer,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    let custom =
        make_named_value_list_option(ctx, ELEMENT_MATCH_HIGHLIGHT_COLOR.to_opaque(), "Custom");
    std::iter::once(custom)
        .chain(
            layer
                .named_values
                .iter()
                .map(|v| make_named_value_list_option(ctx, v.color.to_opaque(), &v.name)),
        )
        .collect()
}
