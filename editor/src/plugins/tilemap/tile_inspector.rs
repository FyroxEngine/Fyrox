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

//! [`TileInspector`] is responsible for the widgets that allow the user to edit a
//! tile's data. The primary mechanism for doing this is through a collection of
//! objects that have the [`TileEditor`] trait. Each TileEditor provides its own
//! widgets and does its own synchronization and message handling, while
//! `TileInspector` is just responsible for managing the `TileEditor` objects.

use std::fmt::Debug;

use crate::{
    command::{Command, CommandGroup},
    plugins::material::editor::{MaterialFieldEditorBuilder, MaterialFieldMessage},
    send_sync_message, MSG_SYNC_FLAG,
};
use fyrox::{
    asset::ResourceDataRef,
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    gui::{
        button::{Button, ButtonMessage},
        decorator::DecoratorMessage,
        expander::ExpanderBuilder,
        grid::{Column, GridBuilder, Row},
        message::UiMessage,
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        vec::{Vec2EditorBuilder, Vec2EditorMessage},
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
    material::{MaterialResource, MaterialResourceExtension},
    scene::tilemap::{brush::*, tileset::*, *},
};

use super::*;
use commands::*;
use palette::*;

pub const FIELD_LABEL_WIDTH: f32 = 100.0;

struct OptionIterator<I>(Option<I>);

impl<I: Iterator> Iterator for OptionIterator<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.as_mut()?.next()
    }
}

pub struct TileEditorStateRef {
    pub page: Option<Vector2<i32>>,
    pub pages_palette: Handle<UiNode>,
    pub tiles_palette: Handle<UiNode>,
    pub state: TileDrawStateRef,
    pub tile_book: TileBook,
}

impl TileEditorStateRef {
    pub fn lock(&self) -> TileEditorState {
        TileEditorState {
            page: self.page,
            pages_palette: self.pages_palette,
            tiles_palette: self.tiles_palette,
            state: Some(self.state.lock()),
            data: TileResourceData::new(&self.tile_book),
        }
    }
}

/// A combination of a guard for [`TileDrawState`] and a guard for either a [`TileSetResource`] or a [`TileMapBrushResource`].
/// This gives a tile editor easy access to all the relevant information, including the current page, the currently selected tiles,
/// and the data from whatever resource is being edited without needing to keep track of locking and unlocking resources.
pub struct TileEditorState<'a> {
    /// The currently open page.
    page: Option<Vector2<i32>>,
    /// The handle of the palette widget for pages. This is used with `state` to determine whether
    /// the current selection is a page.
    pages_palette: Handle<UiNode>,
    /// The handle of the palette widget for tiles. This is used with `state` to determine whether
    /// the current selection is a tile.
    tiles_palette: Handle<UiNode>,
    /// The [`TileDrawState`] that contains the currently selected tiles.
    /// It is Option so that it can be briefly taken and then returned as needed,
    /// but otherwise it can always be safely assumed to be `Some`.
    state: Option<TileDrawStateGuard<'a>>,
    /// The resource that we are editing. This is for read-only access.
    /// Modifying resources is always done through [`commands`].
    data: TileResourceData<'a>,
}

/// An abstract resource guard that could either guard a [`TileSetResource`] or a [`TileMapBrushResource`].
enum TileResourceData<'a> {
    Empty,
    TileSet(ResourceDataRef<'a, TileSet>),
    Brush(ResourceDataRef<'a, TileMapBrush>),
}

impl Debug for TileResourceData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::TileSet(_) => write!(f, "TileSet(..)"),
            Self::Brush(_) => write!(f, "Brush(..)"),
        }
    }
}

impl<'a> TileResourceData<'a> {
    fn new(tile_book: &'a TileBook) -> Self {
        match tile_book {
            TileBook::Empty => Self::Empty,
            TileBook::TileSet(resource) => Self::TileSet(resource.data_ref()),
            TileBook::Brush(resource) => Self::Brush(resource.data_ref()),
        }
    }
    /// The type of the page at the given position, such as atlas, freeform, transform, brush, etc.
    fn page_type(&self, position: Vector2<i32>) -> Option<PageType> {
        match self {
            TileResourceData::Empty => None,
            TileResourceData::TileSet(tile_set) => tile_set
                .as_loaded_ref()
                .and_then(|t| t.get_page(position))
                .map(|p| p.page_type()),
            TileResourceData::Brush(brush) => brush.as_loaded_ref().and_then(|t| {
                if t.has_page_at(position) {
                    Some(PageType::Brush)
                } else {
                    None
                }
            }),
        }
    }
    fn tile_set(&self) -> Option<&ResourceDataRef<'a, TileSet>> {
        if let Self::TileSet(v) = self {
            Some(v)
        } else {
            None
        }
    }
    fn brush(&self) -> Option<&ResourceDataRef<'a, TileMapBrush>> {
        if let Self::Brush(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl<'a> TileEditorState<'a> {
    fn is_tile_set(&self) -> bool {
        self.tile_set().is_some()
    }
    fn is_brush(&self) -> bool {
        self.brush().is_some()
    }
    fn state(&self) -> &TileDrawStateGuard<'a> {
        self.state.as_ref().unwrap()
    }
    pub fn is_active_editor(&self, editor: &TileEditorRef) -> bool {
        self.state().is_active_editor(editor)
    }
    pub fn is_visible_collider(&self, uuid: Uuid) -> bool {
        self.state().visible_colliders.contains(&uuid)
    }
    pub fn visible_colliders(&self) -> impl Iterator<Item = &Uuid> {
        self.state().visible_colliders.iter()
    }
    pub fn drawing_mode(&self) -> DrawingMode {
        self.state().drawing_mode
    }
    /// Force the UI to update itself as if the state had changed.
    pub fn touch(&mut self) {
        let state = self.state.take().unwrap().into_mut("touch");
        self.state = Some(state.into_const());
    }
    pub fn set_active_editor(&mut self, editor: Option<TileEditorRef>) {
        let mut state = self.state.take().unwrap().into_mut("set_active_editor");
        state.active_editor = editor;
        self.state = Some(state.into_const());
    }
    pub fn set_drawing_mode(&mut self, mode: DrawingMode) {
        let mut state = self.state.take().unwrap().into_mut("set_drawing_mode");
        state.drawing_mode = mode;
        self.state = Some(state.into_const());
    }
    pub fn set_visible_collider(&mut self, uuid: Uuid, visible: bool) {
        let mut state = self.state.take().unwrap().into_mut("set_visible_collider");
        state.set_visible_collider(uuid, visible);
        self.state = Some(state.into_const());
    }
    pub fn tile_set(&self) -> Option<&ResourceDataRef<'a, TileSet>> {
        self.data.tile_set()
    }
    pub fn brush(&self) -> Option<&ResourceDataRef<'a, TileMapBrush>> {
        self.data.brush()
    }
    pub fn page(&self) -> Option<Vector2<i32>> {
        self.page
    }
    /// The user is currently selecting pages.
    pub fn has_pages(&self) -> bool {
        self.state().selection_palette() == self.pages_palette && self.state().has_selection()
    }
    /// The user is currently selecting tiles.
    pub fn has_tiles(&self) -> bool {
        self.state().selection_palette() == self.tiles_palette && self.state().has_selection()
    }
    /// The number of selected tile positions, regardless of whether those positions actually contain tiles.
    pub fn tiles_count(&self) -> usize {
        if self.state().selection_palette() == self.tiles_palette {
            self.state().selection_positions().len()
        } else {
            0
        }
    }
    /// The number of selected page positions, regardless of whether those positions actually contain pages.
    pub fn pages_count(&self) -> usize {
        if self.state().selection_palette() == self.pages_palette {
            self.state().selection_positions().len()
        } else {
            0
        }
    }
    pub fn selected_positions(&self) -> impl Iterator<Item = Vector2<i32>> + '_ {
        self.state().selection_positions().iter().copied()
    }
    /// The property layer with the given UUID within the tile set, if we are editing a tile set
    /// and the tile set has a layer with that UUID. None, otherwise.
    pub fn find_property(&self, property_id: Uuid) -> Option<&TileSetPropertyLayer> {
        self.tile_set()?.find_property(property_id)
    }
    /// The collider layer with the given UUID within the tile set, if we are editing a tile set
    /// and the tile set has a layer with that UUID. None, otherwise.
    pub fn find_collider(&self, collider_id: Uuid) -> Option<&TileSetColliderLayer> {
        self.tile_set()?.find_collider(collider_id)
    }
    /// Iterator over all the property layers of the resource. The iterator will be empty for brushes.
    pub fn properties(&self) -> impl Iterator<Item = &TileSetPropertyLayer> {
        OptionIterator(self.tile_set().map(|d| d.properties.iter()))
    }
    /// Iterator over all the collider layers of the resource. The iterator will be empty for brushes.
    pub fn colliders(&self) -> impl Iterator<Item = &TileSetColliderLayer> {
        OptionIterator(self.tile_set().map(|d| d.colliders.iter()))
    }
    pub fn page_positions(&self) -> impl Iterator<Item = Vector2<i32>> + '_ {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(self.state().selection_positions().iter().copied()))
        } else {
            OptionIterator(None)
        }
    }
    /// Iterate through the selected page positions that do not contain pages.
    pub fn empty_page_positions(&self) -> impl Iterator<Item = Vector2<i32>> + '_ {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(
                self.state()
                    .selection_positions()
                    .iter()
                    .copied()
                    .filter(|p| {
                        if let Some(tile_set) = self.tile_set() {
                            !tile_set.pages.contains_key(p)
                        } else if let Some(brush) = self.brush() {
                            !brush.pages.contains_key(p)
                        } else {
                            false
                        }
                    }),
            ))
        } else {
            OptionIterator(None)
        }
    }
    /// Iterate through the selected tile set pages. If we are editing a brush, this iterator will be empty.
    pub fn tile_set_pages(&self) -> impl Iterator<Item = (Vector2<i32>, &TileSetPage)> {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(
                self.state()
                    .selection_positions()
                    .iter()
                    .copied()
                    .filter_map(|p| Some((p, self.tile_set()?.pages.get(&p)?))),
            ))
        } else {
            OptionIterator(None)
        }
    }
    /// Iterate through the selected brush pages. If we are editing a tile set, this iterator will be empty.
    pub fn brush_pages(&self) -> impl Iterator<Item = (Vector2<i32>, &TileMapBrushPage)> {
        if self.state().selection_palette() == self.pages_palette {
            OptionIterator(Some(
                self.state()
                    .selection_positions()
                    .iter()
                    .copied()
                    .filter_map(|p| Some((p, self.brush()?.pages.get(&p)?))),
            ))
        } else {
            OptionIterator(None)
        }
    }
    /// If exactly one page is selected and it happens to be a tile atlas page, then return the position and the page data.
    pub fn material_page(&self) -> Option<(Vector2<i32>, &TileMaterial)> {
        let mut pages = self.tile_set_pages();
        let result = pages.next();
        if pages.next().is_some() {
            return None;
        }
        let (position, page) = result?;
        if let TileSetPageSource::Atlas(m) = &page.source {
            Some((position, m))
        } else {
            None
        }
    }
    /// If exactly one page is selected and it happens to be an animation page, then return the position and the page data.
    pub fn animation_page(&self) -> Option<(Vector2<i32>, &AnimationTiles)> {
        let mut pages = self.tile_set_pages();
        let result = pages.next();
        if pages.next().is_some() {
            return None;
        }
        let (position, page) = result?;
        if let TileSetPageSource::Animation(m) = &page.source {
            Some((position, m))
        } else {
            None
        }
    }
    /// Iterate the selected positions in the form of `TileDefinitionHandle` using the current page for page coordinates.
    pub fn tile_handles(&self) -> impl Iterator<Item = TileDefinitionHandle> + '_ {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| TileDefinitionHandle::try_new(page?, p))
    }
    /// If exactly one brush cell is selected, return the position of that cell within the brush.
    pub fn selected_brush_cell(&self) -> Option<TileDefinitionHandle> {
        if self.is_brush() {
            let mut iter = self.tile_handles();
            let result = iter.next()?;
            iter.next().is_none().then_some(result)
        } else {
            None
        }
    }
    /// Iterate the selected positions in the form of `TileDefinitionHandle` using the current page for page coordinates.
    /// and skip any position that already contains a tile.
    pub fn empty_tiles(&self) -> impl Iterator<Item = TileDefinitionHandle> + '_ {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| TileDefinitionHandle::try_new(page?, p))
            .filter(|&handle| {
                if let Some(tile_set) = self.tile_set() {
                    tile_set.is_free_at(handle.into())
                } else if let Some(brush) = self.brush() {
                    brush.is_free_at(handle.into())
                } else {
                    false
                }
            })
    }
    /// Iterate the selected freeform tiles to produce pairs of tile handles and borrows of the material bounds for each tile.
    pub fn tile_material_bounds(
        &self,
    ) -> impl Iterator<Item = (TileDefinitionHandle, &TileMaterialBounds)> {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| {
                let handle = TileDefinitionHandle::try_new(page?, p)?;
                Some((handle, self.tile_set()?.tile_bounds(handle)?))
            })
    }
    /// Iterate the selected tile set tiles to produce pairs of tile handles and borrows of data for each tile.
    pub fn tile_data(&self) -> impl Iterator<Item = (TileDefinitionHandle, &TileData)> {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| {
                let handle = TileDefinitionHandle::try_new(page?, p)?;
                Some((handle, self.tile_set()?.tile_data(handle)?))
            })
    }
    /// Iterate over the selected positions and produce tile handle pairs where the first handle refers to
    /// the selected position and the second handle refers to handle that the tile redirects to.
    pub fn tile_redirect(
        &self,
    ) -> impl Iterator<Item = (TileDefinitionHandle, TileDefinitionHandle)> + '_ {
        let page = self.page;
        self.state()
            .selection_positions()
            .iter()
            .copied()
            .filter_map(move |p| {
                let handle = TileDefinitionHandle::try_new(page?, p)?;
                if let Some(tile_set) = self.tile_set() {
                    Some((handle, tile_set.tile_redirect(handle)?))
                } else {
                    Some((handle, self.brush()?.tile_redirect(handle)?))
                }
            })
    }
}

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

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
    if button.is_none() {
        return;
    }
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

fn make_property_editors(
    state: &TileEditorState,
    editors: &mut Vec<(Uuid, TileEditorRef)>,
    ctx: &mut BuildContext,
) {
    editors.clear();
    for prop_layer in state.properties() {
        editors.push((
            prop_layer.uuid,
            Arc::new(Mutex::new(TilePropertyEditor::new(
                prop_layer,
                &find_property_value(prop_layer, state),
                ctx,
            ))),
        ));
    }
}

fn make_collider_editors(
    state: &TileEditorState,
    editors: &mut Vec<(Uuid, TileEditorRef)>,
    ctx: &mut BuildContext,
) {
    editors.clear();
    editors.clear();
    for collider_layer in state.colliders() {
        editors.push((
            collider_layer.uuid,
            Arc::new(Mutex::new(TileColliderEditor::new(
                collider_layer,
                find_collider_value(collider_layer, state),
                ctx,
            ))),
        ));
    }
}

fn find_property_value(
    prop_layer: &TileSetPropertyLayer,
    state: &TileEditorState,
) -> TileSetPropertyOptionValue {
    let mut result = prop_layer.prop_type.default_option_value();
    let default_value = prop_layer.prop_type.default_value();
    for (_, data) in state.tile_data() {
        let value = data
            .properties
            .get(&prop_layer.uuid)
            .unwrap_or(&default_value);
        result.intersect(value);
    }
    result
}

fn find_collider_value(
    collider_layer: &TileSetColliderLayer,
    state: &TileEditorState,
) -> TileCollider {
    let uuid = &collider_layer.uuid;
    let mut iter = state
        .tile_data()
        .map(|d| d.1)
        .map(|d| d.colliders.get(uuid));
    iter.next()
        .map(|c| c.cloned().unwrap_or_default())
        .unwrap_or_default()
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
            handle: GridBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::top_bottom(3.0))
                    .with_child(label)
                    .with_child(field),
            )
            .add_row(Row::auto())
            .add_column(Column::strict(FIELD_LABEL_WIDTH))
            .add_column(Column::stretch())
            .build(ctx),
            field,
        }
    }
}

/// Object that keeps track of the editors for all the property layers.
#[derive(Clone, Default, Visit, Reflect)]
struct PropertyEditors {
    handle: Handle<UiNode>,
    content: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    editors: Vec<(Uuid, TileEditorRef)>,
}

impl Debug for PropertyEditors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyEditors")
            .field("handle", &self.handle)
            .field("content", &self.content)
            .finish()
    }
}

impl PropertyEditors {
    fn new(state: &TileEditorState, ctx: &mut BuildContext<'_>) -> Self {
        let mut editors = Vec::default();
        make_property_editors(state, &mut editors, ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(editors.iter().map(|v| v.1.lock().handle())),
        )
        .build(ctx);
        Self {
            handle: ExpanderBuilder::new(WidgetBuilder::new())
                .with_header(make_label("Properties", ctx))
                .with_content(content)
                .build(ctx),
            content,
            editors,
        }
    }
    fn iter(&self) -> impl Iterator<Item = &TileEditorRef> + '_ {
        self.editors.iter().map(|v| &v.1)
    }
    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        // Check whether the list of layers has changed. Have layers been added or removed or changed their order?
        if self.needs_rebuild(state) {
            // The list has changed somehow, so remove the old editors and construct an all new editor for each layer.
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::remove(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                ));
            }
            make_property_editors(state, &mut self.editors, &mut ui.build_ctx());
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::link(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                    self.content,
                ));
            }
        } else {
            // The list has not changed, so just sync each editor because one of the layers may have changed.
            for (_, editor) in self.editors.iter() {
                editor.lock().sync_to_model(state, ui);
            }
        }
    }
    /// Check whether the tile set's layer list matches our list of editors.
    fn needs_rebuild(&self, state: &TileEditorState) -> bool {
        // Do the layers and the editors have the same UUIDs in the same order?
        !self
            .editors
            .iter()
            .map(|v| v.0)
            .eq(state.properties().map(|v| v.uuid))
    }
}

/// Object that keeps track of the editors for all the collider layers.
#[derive(Clone, Default, Visit, Reflect)]
struct ColliderEditors {
    handle: Handle<UiNode>,
    content: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    editors: Vec<(Uuid, TileEditorRef)>,
}

impl Debug for ColliderEditors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ColliderEditors")
            .field("handle", &self.handle)
            .field("content", &self.content)
            .finish()
    }
}

impl ColliderEditors {
    fn new(state: &TileEditorState, ctx: &mut BuildContext<'_>) -> Self {
        let mut editors = Vec::default();
        make_collider_editors(state, &mut editors, ctx);
        let content = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(editors.iter().map(|v| v.1.lock().handle())),
        )
        .build(ctx);
        Self {
            handle: ExpanderBuilder::new(WidgetBuilder::new())
                .with_header(make_label("Colliders", ctx))
                .with_content(content)
                .build(ctx),
            content,
            editors,
        }
    }
    fn iter(&self) -> impl Iterator<Item = &TileEditorRef> + '_ {
        self.editors.iter().map(|v| &v.1)
    }
    fn sync_to_model(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        // Check whether the list of layers has changed. Have layers been added or removed or changed their order?
        if self.needs_rebuild(state) {
            // The list has changed somehow, so remove the old editors and construct an all new editor for each layer.
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::remove(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                ));
            }
            make_collider_editors(state, &mut self.editors, &mut ui.build_ctx());
            for (_, editor) in self.editors.iter() {
                ui.send_message(WidgetMessage::link(
                    editor.lock().handle(),
                    MessageDirection::ToWidget,
                    self.content,
                ));
            }
        } else {
            // The list has not changed, so just sync each editor because one of the layers may have changed.
            for (_, editor) in self.editors.iter() {
                editor.lock().sync_to_model(state, ui);
            }
        }
    }
    /// Check whether the tile set's layer list matches our list of editors.
    fn needs_rebuild(&self, state: &TileEditorState) -> bool {
        // Do the layers and the editors have the same UUIDs in the same order?
        !self
            .editors
            .iter()
            .map(|v| v.0)
            .eq(state.colliders().map(|v| v.uuid))
    }
}

#[derive(Visit, Reflect)]
pub struct TileInspector {
    handle: Handle<UiNode>,
    /// The shared state that represents the user's currently selected tool and tiles.
    #[visit(skip)]
    #[reflect(hidden)]
    state: TileDrawStateRef,
    /// Widget for editing macro cell data.
    macro_inspector: MacroInspector,
    /// The tile set editor palette widget that allows the user to select a page.
    /// This is *not* a widget within the TileInspector, but the TileInspector needs to have
    /// the handle in order to determine where the user is selecting.
    pages_palette: Handle<UiNode>,
    /// The tile set editor palette widget that allows the user to select a tile.
    /// This is *not* a widget within the TileInspector, but the TileInspector needs to have
    /// the handle in order to determine where the user is selecting.
    tiles_palette: Handle<UiNode>,
    /// The current resource to be edited.
    tile_book: TileBook,
    /// The collection of buttons for creating a new tile set page.
    tile_set_page_creator: Handle<UiNode>,
    /// The panel containing the button for creating a new brush page.
    brush_page_creator: Handle<UiNode>,
    /// The editor for changing the size of tiles in a tile atlas page.
    tile_size_inspector: InspectorField,
    /// The editor for changing the frame rate of an animation page.
    animation_speed_inspector: InspectorField,
    /// Button for creating a brush tile.
    create_tile: Handle<UiNode>,
    /// Button for creating a brush page.
    create_page: Handle<UiNode>,
    /// Button for creating an atlas page in a tile set.
    create_atlas: Handle<UiNode>,
    /// Button for creating a freeform page in a tile set.
    create_free: Handle<UiNode>,
    /// Button for creating a transform set page in a tile set.
    create_transform: Handle<UiNode>,
    /// Button for creating a animation page in a tile set.
    create_animation: Handle<UiNode>,
    /// A list of tile editors.
    #[visit(skip)]
    #[reflect(hidden)]
    tile_editors: Vec<TileEditorRef>,
    /// Inspector for setting the material of an atlas page.
    page_material_inspector: InspectorField,
    /// Handle of the material field of an atlas page.
    page_material_field: Handle<UiNode>,
    /// Field for setting the icon of a page.
    page_icon_field: Handle<UiNode>,
    /// Editors for every property layer.
    property_editors: PropertyEditors,
    /// Editors for every collider layer.
    collider_editors: ColliderEditors,
}

impl Debug for TileInspector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileInspector")
            .field("handle", &self.handle)
            .finish()
    }
}

impl TileInspector {
    pub fn new(
        state: TileDrawStateRef,
        macro_list: BrushMacroListRef,
        cell_sets: MacroCellSetListRef,
        pages_palette: Handle<UiNode>,
        tiles_palette: Handle<UiNode>,
        tile_book: TileBook,
        sender: MessageSender,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Self {
        let create_page;
        let create_atlas;
        let create_free;
        let create_transform;
        let create_animation;

        let tile_editors: Vec<TileEditorRef> = vec![
            Arc::new(Mutex::new(TileMaterialEditor::new(
                ctx,
                sender.clone(),
                resource_manager.clone(),
            ))) as TileEditorRef,
            Arc::new(Mutex::new(TileColorEditor::new(ctx))) as TileEditorRef,
            Arc::new(Mutex::new(TileHandleEditor::new(None, ctx))) as TileEditorRef,
        ];

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
            .with_child({
                create_animation =
                    make_button("Animation", "Create a page that controls how tiles animate.", 4, 0, ctx);
                create_animation
            })
        ).add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        let page_material_field =
            MaterialFieldEditorBuilder::new(WidgetBuilder::new().on_column(1)).build(
                ctx,
                sender.clone(),
                DEFAULT_TILE_MATERIAL.deep_copy(),
                resource_manager,
            );
        let page_material_inspector = InspectorField::new("Material", page_material_field, ctx);
        let tile_size_field =
            Vec2EditorBuilder::<u32>::new(WidgetBuilder::new().on_column(1)).build(ctx);
        let tile_size_inspector = InspectorField::new("Tile Size", tile_size_field, ctx);
        let frame_rate_field = NumericUpDownBuilder::<f32>::new(WidgetBuilder::new().on_column(1))
            .with_min_value(0.0)
            .build(ctx);
        let animation_speed_inspector = InspectorField::new("Frame Rate", frame_rate_field, ctx);
        let page_icon_field = TileHandleFieldBuilder::new(WidgetBuilder::new())
            .with_label("Page Icon")
            .build(ctx);
        let macro_inspector = MacroInspector::new(
            macro_list,
            cell_sets,
            tile_book.brush_ref().cloned(),
            None,
            ctx,
        );
        let tile_editor_state = TileEditorStateRef {
            page: None,
            state: state.clone(),
            pages_palette,
            tiles_palette,
            tile_book: tile_book.clone(),
        };
        let tile_editor_state_lock = tile_editor_state.lock();
        let property_editors = PropertyEditors::new(&tile_editor_state_lock, ctx);
        let collider_editors = ColliderEditors::new(&tile_editor_state_lock, ctx);
        let handle = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(tile_set_page_creator)
                .with_child(brush_page_creator)
                .with_child(page_icon_field)
                .with_child(page_material_inspector.handle)
                .with_child(tile_size_inspector.handle)
                .with_child(animation_speed_inspector.handle)
                .with_child(create_tile)
                .with_children(tile_editors.iter().map(|e| e.lock().handle()))
                .with_child(property_editors.handle)
                .with_child(collider_editors.handle)
                .with_child(macro_inspector.handle()),
        )
        .build(ctx);
        Self {
            handle,
            state,
            macro_inspector,
            pages_palette,
            tiles_palette,
            tile_book,
            tile_editors,
            brush_page_creator,
            tile_set_page_creator,
            page_material_inspector,
            page_material_field,
            tile_size_inspector,
            animation_speed_inspector,
            create_tile,
            create_page,
            create_atlas,
            create_free,
            create_transform,
            create_animation,
            page_icon_field,
            property_editors,
            collider_editors,
        }
    }
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    pub fn set_tile_resource(&mut self, tile_book: TileBook, ui: &mut UserInterface) {
        self.tile_book = tile_book;
        self.sync_to_model(ui);
    }
    fn tile_editor_state(&self, ui: &UserInterface) -> TileEditorStateRef {
        let page = if self.state.lock().selection_palette() != self.tiles_palette {
            None
        } else {
            ui.node(self.tiles_palette)
                .cast::<PaletteWidget>()
                .unwrap()
                .page
        };
        TileEditorStateRef {
            page,
            pages_palette: self.pages_palette,
            tiles_palette: self.tiles_palette,
            state: self.state.clone(),
            tile_book: self.tile_book.clone(),
        }
    }
    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        let tile_editor_state = self.tile_editor_state(ui);
        let tile_editor_state = tile_editor_state.lock();
        self.property_editors.sync_to_model(&tile_editor_state, ui);
        self.collider_editors.sync_to_model(&tile_editor_state, ui);
        drop(tile_editor_state);
        self.sync_to_state(ui);
    }
    pub fn sync_to_state(&mut self, ui: &mut UserInterface) {
        let tile_editor_state = self.tile_editor_state(ui);
        let state = tile_editor_state.lock();
        let empty_tiles = state.empty_tiles().next().is_some();
        let empty_pages = state.empty_page_positions().next().is_some();
        let tile_set_empty_pages = state.tile_set().is_some() && empty_pages;
        let brush_empty_pages = state.brush().is_some() && empty_pages;
        let tile_data_selected = state.tile_data().next().is_some();
        let mat_page_selected = state.material_page().is_some();
        let anim_page_selected = state.animation_page().is_some();
        let brush_tile = state.selected_brush_cell();
        drop(state);
        self.macro_inspector.sync_to_cell(
            tile_editor_state.tile_book.brush_ref().cloned(),
            brush_tile,
            ui,
        );
        let state = tile_editor_state.lock();
        send_visibility(ui, self.macro_inspector.handle(), brush_tile.is_some());
        send_visibility(ui, self.tile_set_page_creator, tile_set_empty_pages);
        send_visibility(ui, self.brush_page_creator, brush_empty_pages);
        send_visibility(ui, self.create_tile, empty_tiles);
        send_visibility(ui, self.tile_set_page_creator, tile_set_empty_pages);
        send_visibility(
            ui,
            self.animation_speed_inspector.handle,
            anim_page_selected,
        );
        send_visibility(ui, self.tile_size_inspector.handle, mat_page_selected);
        send_visibility(ui, self.page_material_inspector.handle, mat_page_selected);
        send_visibility(
            ui,
            self.page_icon_field,
            state.tile_set_pages().next().is_some() || state.brush_pages().next().is_some(),
        );
        send_visibility(ui, self.property_editors.handle, tile_data_selected);
        send_visibility(ui, self.collider_editors.handle, tile_data_selected);
        self.sync_to_page(&state, ui);
        let page_icon = self.find_page_icon(&state);
        send_sync_message(
            ui,
            TileHandleEditorMessage::value(
                self.page_icon_field,
                MessageDirection::ToWidget,
                page_icon,
            ),
        );
        let iter = self
            .tile_editors
            .iter()
            .chain(self.property_editors.iter())
            .chain(self.collider_editors.iter());
        for editor_ref in iter {
            let mut editor = editor_ref.try_lock().expect("Failed to lock editor_ref");
            editor.sync_to_state(&state, ui);
            let draw_button = editor.draw_button();
            drop(editor);
            highlight_tool_button(
                draw_button,
                state.drawing_mode() == DrawingMode::Editor && state.is_active_editor(editor_ref),
                ui,
            );
        }
    }
    fn find_page_icon(&self, state: &TileEditorState) -> Option<TileDefinitionHandle> {
        if state.is_tile_set() {
            let mut iter = state.tile_set_pages().map(|(_, p)| p.icon);
            let icon = iter.next()?;
            if iter.all(|h| h == icon) {
                Some(icon)
            } else {
                None
            }
        } else if state.is_brush() {
            let mut iter = state.brush_pages().map(|(_, p)| p.icon);
            let icon = iter.next()?;
            if iter.all(|h| h == icon) {
                Some(icon)
            } else {
                None
            }
        } else {
            None
        }
    }
    fn sync_to_page(&mut self, state: &TileEditorState, ui: &mut UserInterface) {
        if let Some((_, mat)) = state.material_page() {
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
        } else if let Some((_, anim)) = state.animation_page() {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.animation_speed_inspector.field,
                    MessageDirection::ToWidget,
                    anim.frame_rate,
                ),
            );
        }
    }
    pub fn handle_ui_message(&mut self, message: &UiMessage, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();
        if message.flags == MSG_SYNC_FLAG || message.direction() == MessageDirection::ToWidget {
            return;
        }
        if !ui.is_node_child_of(message.destination(), self.handle()) {
            return;
        }
        if let Some(brush) = self.tile_book.brush_ref() {
            let tile_editor_state = self.tile_editor_state(ui);
            let cell = tile_editor_state.lock().selected_brush_cell();
            drop(tile_editor_state);
            self.macro_inspector
                .handle_ui_message(brush.clone(), cell, message, editor);
        }
        let ui = editor.engine.user_interfaces.first_mut();
        let tile_editor_state = self.tile_editor_state(ui);
        let mut tile_editor_state = tile_editor_state.lock();
        let sender = &editor.message_sender;
        let iter = self
            .tile_editors
            .iter()
            .chain(self.property_editors.iter())
            .chain(self.collider_editors.iter());
        for editor in iter {
            editor.lock().handle_ui_message(
                &mut tile_editor_state,
                message,
                ui,
                &self.tile_book,
                sender,
            );
        }
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create_atlas {
                self.create_tile_set_page(
                    TileSetPageSource::new_material(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_free {
                self.create_tile_set_page(
                    TileSetPageSource::new_free(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_transform {
                self.create_tile_set_page(
                    TileSetPageSource::new_transform(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_animation {
                self.create_tile_set_page(
                    TileSetPageSource::new_animation(),
                    &tile_editor_state,
                    sender,
                );
            } else if message.destination() == self.create_page {
                self.create_brush_page(&tile_editor_state, sender);
            } else if message.destination() == self.create_tile {
                self.create_tile(&tile_editor_state, sender);
            } else {
                let iter = self
                    .tile_editors
                    .iter()
                    .chain(self.property_editors.iter())
                    .chain(self.collider_editors.iter());
                for editor_ref in iter {
                    let draw_button = editor_ref.lock().draw_button();
                    if message.destination() == draw_button {
                        if tile_editor_state.is_active_editor(editor_ref) {
                            tile_editor_state.set_active_editor(None);
                            tile_editor_state.set_drawing_mode(DrawingMode::Pick);
                        } else {
                            tile_editor_state.set_active_editor(Some(editor_ref.clone()));
                            tile_editor_state.set_drawing_mode(DrawingMode::Editor);
                        }
                    }
                }
            }
        } else if let Some(MaterialFieldMessage::Material(material)) = message.data() {
            if message.destination() == self.page_material_inspector.field {
                self.set_page_material(material.clone(), &tile_editor_state, sender);
            }
        } else if let Some(Vec2EditorMessage::<u32>::Value(size)) = message.data() {
            if message.destination() == self.tile_size_inspector.field {
                self.set_page_tile_size(*size, &tile_editor_state, sender);
            }
        } else if let Some(NumericUpDownMessage::<f32>::Value(speed)) = message.data() {
            if message.destination() == self.animation_speed_inspector.field {
                self.set_animation_speed(*speed, &tile_editor_state, sender);
            }
        } else if let Some(TileHandleEditorMessage::Value(Some(handle))) = message.data() {
            if message.destination() == self.page_icon_field {
                self.apply_page_icon(*handle, &tile_editor_state, sender);
            }
        }
    }
    fn apply_page_icon(
        &self,
        icon: TileDefinitionHandle,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let cmds = match &self.tile_book {
            TileBook::Empty => return,
            TileBook::TileSet(tile_set) => state
                .page_positions()
                .map(|position| ModifyPageIconCommand::new(tile_set.clone(), position, icon))
                .map(Command::new)
                .collect::<Vec<_>>(),
            TileBook::Brush(brush) => state
                .page_positions()
                .map(|position| ModifyBrushPageIconCommand::new(brush.clone(), position, icon))
                .map(Command::new)
                .collect::<Vec<_>>(),
        };
        sender.do_command(CommandGroup::from(cmds).with_custom_name("Modify Tile Page Icon"));
    }
    /// Create default tiles at any empty tile positions in the current selection.
    fn create_tile(&self, state: &TileEditorState, sender: &MessageSender) {
        match &self.tile_book {
            TileBook::Empty => (),
            TileBook::TileSet(tile_set) => {
                let mut update = TileSetUpdate::default();
                for handle in state.empty_tiles() {
                    match state.data.page_type(handle.page()) {
                        Some(PageType::Atlas) => drop(
                            update
                                .insert(handle, TileDataUpdate::MaterialTile(TileData::default())),
                        ),
                        Some(PageType::Freeform) => drop(update.insert(
                            handle,
                            TileDataUpdate::FreeformTile(TileDefinition::default()),
                        )),
                        Some(PageType::Transform) => drop(update.insert(
                            handle,
                            TileDataUpdate::TransformSet(Some(TileDefinitionHandle::default())),
                        )),
                        Some(PageType::Animation) => drop(update.insert(
                            handle,
                            TileDataUpdate::TransformSet(Some(TileDefinitionHandle::default())),
                        )),
                        _ => (),
                    }
                }
                sender.do_command(SetTileSetTilesCommand {
                    tile_set: tile_set.clone(),
                    tiles: update,
                });
            }
            TileBook::Brush(brush) => {
                if let Some(page) = state.page {
                    let mut tiles = TilesUpdate::default();
                    for position in state.selected_positions() {
                        tiles.insert(position, Some(TileDefinitionHandle::EMPTY));
                    }
                    sender.do_command(SetBrushTilesCommand {
                        brush: brush.clone(),
                        page,
                        tiles,
                    });
                }
            }
        }
    }
    fn create_brush_page(&self, state: &TileEditorState, sender: &MessageSender) {
        let TileBook::Brush(brush) = &self.tile_book else {
            return;
        };
        let cmds = state
            .empty_page_positions()
            .map(|position| SetBrushPageCommand {
                brush: brush.clone(),
                position,
                page: Some(TileMapBrushPage {
                    icon: TileDefinitionHandle::new(0, 0, 0, -1),
                    tiles: Tiles::default(),
                }),
            })
            .map(Command::new)
            .collect::<Vec<_>>();
        sender.do_command(CommandGroup::from(cmds).with_custom_name("Create Brush Page"));
    }
    fn create_tile_set_page(
        &self,
        source: TileSetPageSource,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileBook::TileSet(tile_set) = &self.tile_book else {
            return;
        };
        let cmds = state
            .empty_page_positions()
            .filter_map(|position| {
                Some(SetTileSetPageCommand {
                    tile_set: tile_set.clone(),
                    position,
                    page: Some(TileSetPage {
                        icon: TileDefinitionHandle::try_new(position, Vector2::new(0, -1))?,
                        source: source.clone(),
                    }),
                })
            })
            .map(Command::new)
            .collect::<Vec<_>>();
        sender.do_command(CommandGroup::from(cmds).with_custom_name("Create Tile Set Page"));
    }
    fn set_page_material(
        &self,
        material: MaterialResource,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileBook::TileSet(tile_set) = self.tile_book.clone() else {
            return;
        };
        if let Some((page, _)) = state.material_page() {
            sender.do_command(ModifyPageMaterialCommand {
                tile_set,
                page,
                material,
            });
        }
    }
    fn set_page_tile_size(
        &self,
        size: Vector2<u32>,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileBook::TileSet(tile_set) = self.tile_book.clone() else {
            return;
        };
        if let Some((page, _)) = state.material_page() {
            sender.do_command(ModifyPageTileSizeCommand {
                tile_set,
                page,
                size,
            });
        }
    }
    fn set_animation_speed(
        &self,
        frame_rate: f32,
        state: &TileEditorState,
        sender: &MessageSender,
    ) {
        let TileBook::TileSet(tile_set) = self.tile_book.clone() else {
            return;
        };
        if let Some((page, _)) = state.animation_page() {
            sender.do_command(ModifyAnimationSpeedCommand {
                tile_set,
                page,
                frame_rate,
            });
        }
    }
}
