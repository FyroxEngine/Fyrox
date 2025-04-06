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
    asset::{untyped::UntypedResource, Resource, ResourceData, ResourceDataRef},
    core::{
        futures::executor::block_on,
        log::Log,
        parking_lot::{lock_api::MutexGuard, RawMutex},
    },
    gui::{
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
    },
    scene::tilemap::{
        brush::{BrushMacroData, TileMapBrush, TileMapBrushResource},
        tileset::{
            NamableValue, TileSetPropertyLayer, TileSetPropertyType, TileSetPropertyValueElement,
        },
        MacroTilesUpdate,
    },
};

use crate::{
    command::{Command, CommandContext, CommandTrait},
    send_sync_message,
};

use super::*;

const PROPERTY_LABEL_WIDTH: f32 = 150.0;
const UNKNOWN_PROPERTY: &str = "UNKNOWN PROPERTY";

/// An Arc Mutex reference to a [`BrushMacroList`] that allows multiple objects
/// to share access to a common macro list. Among the things that need to share
/// the list are the tile map interaction mode and the [`TileSetEditor`].
#[derive(Default, Clone)]
pub struct BrushMacroListRef(Arc<Mutex<BrushMacroList>>);

impl BrushMacroListRef {
    /// Create a new reference and assign it to point to the given macro list.
    pub fn new(list: BrushMacroList) -> Self {
        Self(Arc::new(Mutex::new(list)))
    }
    /// Access the macro list.
    pub fn lock(&self) -> MutexGuard<RawMutex, BrushMacroList> {
        self.0
            .try_lock()
            .expect("BrushMacroList lock should not fail")
    }
}

/// An Arc Mutex reference to a [`MacroCellSetList`] which allows
/// shared access to the positions of cells which are currently involved
/// in some macro. This is only used in the [`TileSetEditor`], but it needs
/// to be shared with the [`PaletteWidget`] so that the widget can
/// render a highlight at those positions.
#[derive(Default, Clone)]
pub struct MacroCellSetListRef(Arc<Mutex<MacroCellSetList>>);

impl MacroCellSetListRef {
    /// Create a new reference and assign it to point to the given set of cells.
    pub fn new(list: MacroCellSetList) -> Self {
        Self(Arc::new(Mutex::new(list)))
    }
    /// Access the set of cells.
    pub fn lock(&self) -> MutexGuard<RawMutex, MacroCellSetList> {
        self.0.try_lock().expect("MacroCellSetList already in use")
    }
}

/// A context for calling some [`BrushMacro`] methods when using the macro
/// to modify a tile map. It gives the macro extensive access to whatever it
/// might need for a wide variety of automatic actions that may be desired
/// along with tile map editing.
pub struct TileMapContext<'a> {
    /// The handle for the tile map that is being edited.
    pub node: Handle<Node>,
    /// The handle for the scene that contains the tile map.
    pub scene: Handle<Scene>,
    /// The engine that contains the scene.
    pub engine: &'a mut Engine,
}

impl TileMapContext<'_> {
    /// The tile map that is being edited.
    pub fn tile_map(&self) -> &TileMap {
        self.engine.scenes[self.scene].graph[self.node]
            .cast()
            .unwrap()
    }
    /// The tile set resource from within the tile map, if the tile map has one.
    pub fn tile_set(&self) -> Option<&TileSetResource> {
        self.tile_map().tile_set()
    }
}

/// Calling some methods of [`BrushMacro`] may require access to the brush resource
/// and to the [`UntypedResource`] that the brush holds to represent the configuration
/// settings for an instance of the macro.
#[derive(Debug, Clone)]
pub struct BrushMacroInstance {
    /// The brush that has an instance of the macro.
    pub brush: TileMapBrushResource,
    /// The configuration for an instance of the macro.
    /// Some macros may not require any configuration, so this resource is optional.
    pub settings: Option<UntypedResource>,
}

impl BrushMacroInstance {
    /// A typed reference to the configuration resource.
    pub fn settings<T>(&self) -> Option<Resource<T>>
    where
        T: ResourceData + Default + TypeUuidProvider,
    {
        self.settings.as_ref()?.try_cast()
    }
    /// A data ref to the brush resource.
    pub fn brush(&self) -> ResourceDataRef<TileMapBrush> {
        self.brush.data_ref()
    }
    /// The tile set resource taken from the brush.
    pub fn tile_set(&self) -> Option<TileSetFromBrush> {
        TileSetFromBrush::try_new(&self.brush)
    }
}

/// The context required for handling UI messages for some particular selected
/// brush cell. This is used for both received [`UiMessage`] and when the brush
/// cell settings need to sync to the brush resource.
#[derive(Debug, Clone)]
pub struct MacroMessageContext {
    /// The brush that has an instance of the macro.
    pub brush: TileMapBrushResource,
    /// The currently selected brush cell, or None if no cell is selected.
    pub cell: Option<TileDefinitionHandle>,
}

impl From<BrushMacroCellContext> for MacroMessageContext {
    fn from(value: BrushMacroCellContext) -> Self {
        Self {
            brush: value.brush,
            cell: value.cell,
        }
    }
}

impl MacroMessageContext {
    /// A reference to the brush's data.
    pub fn brush(&self) -> ResourceDataRef<TileMapBrush> {
        self.brush.data_ref()
    }
    /// The tile set taken from the brush.
    pub fn tile_set(&self) -> Option<TileSetFromBrush> {
        TileSetFromBrush::try_new(&self.brush)
    }
    /// A copy of the list of configuration resources for the macro instances
    /// that match the given UUID. This is copied instead of being borrowed
    /// to avoid holding a lock on the brush resource while iterating through
    /// the macro instances.
    pub fn instances_with_uuid(&self, uuid: Uuid) -> Vec<UntypedResource> {
        self.brush
            .data_ref()
            .macros
            .instances_with_uuid(uuid)
            .cloned()
            .collect()
    }
}

/// Context for methods of [`BrushMacro`] that are specific to a single brush cell,
/// such as adding a new cell to the macro, removing a cell, or building the widgets
/// for editing the settings of a cell.
#[derive(Debug, Clone)]
pub struct BrushMacroCellContext {
    /// The brush that has an instance of the macro.
    pub brush: TileMapBrushResource,
    /// The configuration for an instance of the macro.
    pub settings: Option<UntypedResource>,
    /// The currently selected brush cell, or None if no cell is selected.
    pub cell: Option<TileDefinitionHandle>,
}

impl From<BrushMacroCellContext> for BrushMacroInstance {
    fn from(value: BrushMacroCellContext) -> Self {
        Self {
            brush: value.brush,
            settings: value.settings,
        }
    }
}

impl BrushMacroCellContext {
    /// A typed reference to the configuration resource.
    pub fn settings<T>(&self) -> Option<Resource<T>>
    where
        T: ResourceData + Default + TypeUuidProvider,
    {
        self.settings.as_ref()?.try_cast()
    }
    /// A reference to the brush's data.
    pub fn brush(&self) -> ResourceDataRef<TileMapBrush> {
        self.brush.data_ref()
    }
    /// The tile set resource taken from the brush.
    pub fn tile_set(&self) -> Option<TileSetFromBrush> {
        TileSetFromBrush::try_new(&self.brush)
    }
}

/// Access to the tile set of a brush through a lock on the brush's data.
pub struct TileSetFromBrush<'a>(ResourceDataRef<'a, TileMapBrush>);

impl Deref for TileSetFromBrush<'_> {
    type Target = Resource<TileSet>;

    fn deref(&self) -> &Self::Target {
        self.0.tile_set.as_ref().unwrap()
    }
}

impl<'a> TileSetFromBrush<'a> {
    /// Construct access to the tile set of the given brush after confirming that
    /// the brush is loaded, the brush has a tile set, and the tile set is loaded.
    pub fn try_new(brush: &'a TileMapBrushResource) -> Option<Self> {
        Log::verify(block_on(brush.clone()));
        if !brush.is_ok() {
            return None;
        }
        let brush_guard = brush.data_ref();
        let tile_set = brush_guard.tile_set.clone();
        drop(brush_guard);
        if let Some(tile_set) = tile_set {
            Log::verify(block_on(tile_set.clone()));
            if !tile_set.is_ok() {
                return None;
            }
        }
        Some(Self(brush.data_ref()))
    }
}

/// The trait that defines the operation necessary for implementing a macro for a tile map brush.
pub trait BrushMacro: 'static + Send + Sync {
    /// The UUID that identifies an instance of a macro as belonging to this macro.
    /// One macro may have several instances, and all of them will share this UUID.
    fn uuid(&self) -> &Uuid;
    /// The name of the macro that will appear in the brush editor.
    fn name(&self) -> &'static str;
    /// Handle a UI message that was received while the brush editor was displaying the settings
    /// for an instance of this macro in the Macros tab.
    fn on_instance_ui_message(
        &mut self,
        context: &BrushMacroInstance,
        message: &UiMessage,
        editor: &mut Editor,
    );
    /// Handle a UI message that was received while a particular brush cell was selected in the editor,
    /// or when no brush cell is selected. Each macro needs to keep track of the widgets that it uses to
    /// display the settings for cells in the tile inspector, and this method is where messages from those
    /// widgets should be handled.
    fn on_cell_ui_message(
        &mut self,
        context: &MacroMessageContext,
        message: &UiMessage,
        editor: &mut Editor,
    );
    /// Create a resource to store the settings of this macro.
    /// A return value of None indicates that his macro needs no settings resource.
    /// An instance will still be created without the resource.
    fn create_instance(&self, brush: &TileMapBrushResource) -> Option<UntypedResource>;
    /// True if instances of this macro can contain particular brush cells.
    fn can_create_cell(&self) -> bool;
    /// Create a command to modify the given instance's data to include the given cell.
    /// None is returned if no command is necessary, such as if the cell is already included
    /// or no cell is selected. Adding the currently selected cell will naturally require
    /// the widgets that edit the data to change, but that will wait until
    /// [`BrushMacro::sync_cell_editors`] is called.
    fn create_cell(&self, context: &BrushMacroCellContext) -> Option<Command>;
    /// Create a command to move cell data from the given cells to the given cells.
    /// The two lists of cells should always be the same length.
    /// This method works on multiple cells at once because `from` and `to` may
    /// share some cells in common, in which case correctly moving the cells
    /// requires first removing all the cells from `from`, then inserting all
    /// the cells into `to`.
    fn move_cells(
        &self,
        from: Box<[TileDefinitionHandle]>,
        to: Box<[TileDefinitionHandle]>,
        context: &BrushMacroInstance,
    ) -> Option<Command>;
    /// Create a command to move cell data from the given pages to the given pages.
    /// The two lists of pages should always be the same length.
    /// This method works on multiple pages at once because `from` and `to` may
    /// share some pages in common, in which case correctly moving the pages
    /// requires first removing all the pages from `from`, then inserting all
    /// the pages into `to`.
    fn move_pages(
        &self,
        from: Box<[Vector2<i32>]>,
        to: Box<[Vector2<i32>]>,
        context: &BrushMacroInstance,
    ) -> Option<Command>;
    /// Create a command to modify the given instances's data to copy the given cell.
    /// None is returned if no command is necessary, such as if the macro has no cell data.
    fn copy_cell(
        &self,
        source: Option<TileDefinitionHandle>,
        destination: TileDefinitionHandle,
        context: &BrushMacroInstance,
    ) -> Option<Command>;
    /// Create a command to modify the given instances's data to copy the given page.
    /// None is returned if no command is necessary, such as if the macro has no cell data.
    fn copy_page(
        &self,
        source: Option<Vector2<i32>>,
        destination: Vector2<i32>,
        context: &BrushMacroInstance,
    ) -> Option<Command>;
    /// Modify the given `cell_set` to include the handles of all the cells that are part of
    /// the given instance of this macro. This is necessary for the brush editor to accurately
    /// update itself.
    fn fill_cell_set(
        &self,
        context: &BrushMacroInstance,
        cell_set: &mut FxHashSet<TileDefinitionHandle>,
    );
    /// Build the widgets that will edit the configuration data for an instance of the
    /// macro. The macro is modified to remember the handles of the widgets so that it
    /// can respond correctly to UI messages. None is returned if this macro needs no
    /// widgets because it cannot be edited.
    fn build_instance_editor(
        &mut self,
        context: &BrushMacroInstance,
        ctx: &mut BuildContext,
    ) -> Option<Handle<UiNode>>;
    /// Build the widgets that will edit the configuration data for an instance of the
    /// macro and for a particular cell of the brush.
    /// The macro is modified to remember the handles of the widgets so that it
    /// can respond correctly to UI messages. Each instance may have it own separate widgets,
    /// so the handles must be remembered for every instance.
    /// None is returned if this macro needs no widgets because cell data cannot be edited.
    /// If no cell is selected or if the cell is not part of the macro, then the widgets
    /// should stil be created but they should be invisible. They can be made visible
    /// when [`BrushMacro::sync_cell_editors`] is called.
    fn build_cell_editor(
        &mut self,
        context: &BrushMacroCellContext,
        ctx: &mut BuildContext,
    ) -> Option<Handle<UiNode>>;
    /// Send the necessary messages to update the cell editor widgets to edit the data for
    /// the given instance.
    fn sync_instance_editor(&mut self, context: &BrushMacroInstance, ui: &mut UserInterface);
    /// Send the necessary messages to update the cell editor widgets to edit the data for
    /// the given cell, or make those widgets invisible if there is no selected cell or the
    /// cell has no data.
    fn sync_cell_editors(&mut self, context: &MacroMessageContext, ui: &mut UserInterface);
    /// This is called when the user begins drawing. This is called for every macro instance
    /// stored in the current brush when the user presses the mouse button down to begin a stroke
    /// and gives the macros a chance to prepare themselves for the series of [`amend_update`](BrushMacro::amend_update)
    /// calls that will follow.
    fn begin_update(
        &mut self,
        context: &BrushMacroInstance,
        stamp: &Stamp,
        tile_map: &TileMapContext,
    );
    /// This is called for every macro instance whenever the user uses a brush to update a tile map.
    /// - `context`: The macro instance.
    /// - `update`: The change that the user is attempting to make to the tile map. Modify this to
    ///   amend the change as appropriate.
    /// - `tile_map`: The tile map that is being edited.
    fn amend_update(
        &mut self,
        context: &BrushMacroInstance,
        update: &mut MacroTilesUpdate,
        tile_map: &TileMap,
    );
    /// This is called just before a command is submitted to actually modify the tile map.
    /// Each macro instance receives this call, and each instance may make any final modifications
    /// to the `update`, and may return an optional [`Command`] that will be collected with commands
    /// from the other macros along with the command to apply `update` to the tile map to construct
    /// the command that is finally sent to the editor.
    fn create_command(
        &mut self,
        context: &BrushMacroInstance,
        update: &mut MacroTilesUpdate,
        tile_map: &TileMapContext,
    ) -> Option<Command>;
}

/// List of [`BrushMacro`] implementations that the [`TileMapEditorPlugin`] keeps in order to allow
/// macro instances to be added to a [`TileMapBrush`].
pub struct BrushMacroList {
    content: Vec<Box<dyn BrushMacro>>,
}

impl Default for BrushMacroList {
    fn default() -> Self {
        let mut result = Self {
            content: Default::default(),
        };
        result.add(AutoTileMacro::default());
        result.add(WfcMacro::default());
        result
    }
}

impl BrushMacroList {
    /// Add the given macro to the list.
    #[inline]
    pub fn add<T: BrushMacro>(&mut self, brush_macro: T) {
        self.content
            .push(Box::new(brush_macro) as Box<dyn BrushMacro>);
    }
    /// True if the list is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    /// The number of macros in the list.
    #[inline]
    pub fn len(&self) -> usize {
        self.content.len()
    }
    /// Iterate through the macros.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &dyn BrushMacro> {
        self.content.iter().map(|b| b.as_ref())
    }
    /// Iterate through the macros.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn BrushMacro> {
        self.content.iter_mut().map(|b| b.as_mut())
    }
    /// Access the macro at the given index.
    #[inline]
    pub fn get_by_index(&self, index: usize) -> Option<&dyn BrushMacro> {
        self.content.get(index).map(|b| b.as_ref())
    }
    /// Access the macro at the given index.
    #[inline]
    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut dyn BrushMacro> {
        self.content.get_mut(index).map(|b| b.as_mut())
    }
    /// Find the macro with the given UUID.
    pub fn get_by_uuid(&self, uuid: &Uuid) -> Option<&dyn BrushMacro> {
        for brush_macro in self.content.iter() {
            if brush_macro.uuid() == uuid {
                return Some(brush_macro.as_ref());
            }
        }
        None
    }
    /// Find the macro with the given UUID.
    pub fn get_by_uuid_mut(&mut self, uuid: &Uuid) -> Option<&mut dyn BrushMacro> {
        for brush_macro in self.content.iter_mut() {
            if brush_macro.uuid() == uuid {
                return Some(brush_macro.as_mut());
            }
        }
        None
    }
    /// Remove all macros from the list.
    #[inline]
    pub fn clear(&mut self) {
        self.content.clear();
    }
}

/// Command for changing the position of a macro instance within the instance
/// list of a [`TileMapBrush`]. This instance is swapped with whichever instance
/// already occupies the new position.
#[derive(Debug)]
pub struct MoveMacroCommand {
    /// The brush resource to modify
    pub brush: TileMapBrushResource,
    /// The starting position of the instance
    pub start: usize,
    /// The resulting position of the instance
    pub end: usize,
}

impl CommandTrait for MoveMacroCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Brush Macro".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        brush.macros.swap(self.start, self.end);
        brush.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        brush.macros.swap(self.start, self.end);
        brush.change_flag.set();
    }
}

/// Command for removing a macro instance from the instance list of a [`TileMapBrush`].
#[derive(Debug)]
pub struct RemoveMacroCommand {
    /// The brush resource to modify
    pub brush: TileMapBrushResource,
    /// The index of the instance to remove.
    pub index: usize,
    /// The data that was removed, after the operation is finished. It should be initially None.
    pub data: Option<BrushMacroData>,
}

impl CommandTrait for RemoveMacroCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Remove Brush Macro".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        self.data = Some(brush.macros.remove(self.index));
        brush.change_flag.set();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        if let Some(data) = self.data.take() {
            brush.macros.insert(self.index, data);
            brush.change_flag.set();
        }
    }
}

/// Command for inserting a macro instance into the instance list of a [`TileMapBrush`].
#[derive(Debug)]
pub struct AddMacroCommand {
    /// The brush resource to modify
    pub brush: TileMapBrushResource,
    /// The index where the new instance will be inserted
    pub index: usize,
    /// The instance data to insert
    pub data: Option<BrushMacroData>,
}

impl CommandTrait for AddMacroCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Add Brush Macro".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        if let Some(data) = self.data.take() {
            brush.macros.insert(self.index, data);
            brush.change_flag.set();
        }
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        let mut brush = self.brush.data_ref();
        self.data = Some(brush.macros.remove(self.index));
        brush.change_flag.set();
    }
}

/// Command for changing the name of a marco instances within a tile map brush.
#[derive(Debug)]
pub struct SetMacroNameCommand {
    /// The brush resouce to modify
    pub brush: TileMapBrushResource,
    /// The index of the instance that will be renamed
    pub index: usize,
    /// The new name for the instance
    pub name: String,
}

impl SetMacroNameCommand {
    fn swap(&mut self) {
        let mut brush = self.brush.data_ref();
        let brush_macro = brush.macros.get_mut(self.index).unwrap();
        std::mem::swap(&mut brush_macro.name, &mut self.name);
        brush.change_flag.set();
    }
}

impl CommandTrait for SetMacroNameCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Rename Brush Macro".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

/// Message sent from a [`MacroPropertyValueField`] when the value changes.
#[derive(Clone, Debug, PartialEq)]
pub struct TileSetPropertyValueMessage(pub TileSetPropertyValueElement);

impl TileSetPropertyValueMessage {
    /// Construct a message to indicate a change in the value of a [`MacroPropertyValueField`].
    pub fn value(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        value: TileSetPropertyValueElement,
    ) -> UiMessage {
        UiMessage::with_data(TileSetPropertyValueMessage(value))
            .with_destination(destination)
            .with_direction(direction)
    }
}

/// Message sent from a [`MacroPropertyField`] when the value changes.
#[derive(Clone, Debug, PartialEq)]
pub struct TileSetPropertyMessage(pub Option<Uuid>);

impl TileSetPropertyMessage {
    /// Construct a message to indicate a change in the value of a [`MacroPropertyField`].
    pub fn property_id(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        value: Option<Uuid>,
    ) -> UiMessage {
        UiMessage::with_data(TileSetPropertyMessage(value))
            .with_destination(destination)
            .with_direction(direction)
    }
}

/// A field that allows the user to choose a property value from a [`TileSet`].
/// The property is represented as a [`TileSetPropertyValueElement`] internally, but the user
/// is given a dropdown list of value names if possible.
#[derive(Debug, Default, Clone)]
pub struct MacroPropertyValueField {
    handle: Handle<UiNode>,
    textbox: Handle<UiNode>,
    list: Handle<UiNode>,
}

fn make_index_and_value_list(
    prop: Option<&TileSetPropertyLayer>,
    value: NamableValue,
    ctx: &mut BuildContext,
) -> (usize, Vec<Handle<UiNode>>) {
    let index;
    let items;
    if let Some(prop) = prop {
        index = prop
            .find_value_index(value)
            .map(|i| i + 1)
            .unwrap_or_default();
        items = make_named_value_list_items(prop, ctx);
    } else {
        index = 0;
        items = vec![make_named_value_list_option(
            ctx,
            ELEMENT_MATCH_HIGHLIGHT_COLOR.to_opaque(),
            "Custom",
        )];
    }
    (index, items)
}

fn find_list_index(prop: Option<&TileSetPropertyLayer>, value: NamableValue) -> usize {
    if let Some(prop) = prop {
        prop.find_value_index(value)
            .map(|i| i + 1)
            .unwrap_or_default()
    } else {
        0
    }
}

impl MacroPropertyValueField {
    /// -`label`: The string that names this field on the left side.
    /// -`value`: The current value of the property. This determines the type of the field,
    ///    and cannot be changed later.
    /// -`prop`: The property of the tile set. This is used to build a dropdown list of
    ///    named values.
    pub fn new(
        widget_builder: WidgetBuilder,
        label: String,
        value: TileSetPropertyValueElement,
        prop: Option<&TileSetPropertyLayer>,
        ctx: &mut BuildContext,
    ) -> Self {
        use TileSetPropertyValueElement as Element;
        let label = TextBuilder::new(WidgetBuilder::new())
            .with_text(label)
            .build(ctx);
        let wb = WidgetBuilder::new().on_column(1);
        let textbox = match &value {
            Element::I32(v) => NumericUpDownBuilder::<i32>::new(wb)
                .with_value(*v)
                .build(ctx),
            Element::F32(v) => NumericUpDownBuilder::<f32>::new(wb)
                .with_value(*v)
                .build(ctx),
            Element::I8(v) => NumericUpDownBuilder::<i8>::new(wb)
                .with_value(*v)
                .build(ctx),
            Element::String(v) => TextBoxBuilder::new(wb).with_text(v).build(ctx),
        };
        let list = if let Ok(value) = value.try_into() {
            let (index, items) = make_index_and_value_list(prop, value, ctx);
            DropdownListBuilder::new(WidgetBuilder::new().on_column(1).on_row(1))
                .with_items(items)
                .with_selected(index)
                .build(ctx)
        } else {
            Handle::NONE
        };
        let handle = GridBuilder::new(
            widget_builder
                .with_child(label)
                .with_child(textbox)
                .with_child(list),
        )
        .add_column(Column::strict(PROPERTY_LABEL_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .build(ctx);
        Self {
            handle,
            textbox,
            list,
        }
    }
    /// The handle of the widget that holds the overall field.
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    /// Update the field's widgets to reflect any changes in the tile set.
    /// -`value`: The UUID of the currently selected property. This will be ignored
    ///    if its type does not match the type of the field.
    /// -`prop`: The property of the tile set. This is used to build a dropdown list of
    ///    named values.
    pub fn sync(
        &self,
        value: TileSetPropertyValueElement,
        prop: Option<&TileSetPropertyLayer>,
        ui: &mut UserInterface,
    ) {
        use TileSetPropertyValueElement as Element;
        let msg = match &value {
            Element::I32(v) => {
                NumericUpDownMessage::value(self.textbox, MessageDirection::ToWidget, *v)
            }
            Element::F32(v) => {
                NumericUpDownMessage::value(self.textbox, MessageDirection::ToWidget, *v)
            }
            Element::String(v) => {
                TextMessage::text(self.textbox, MessageDirection::ToWidget, v.to_string())
            }
            Element::I8(v) => {
                NumericUpDownMessage::value(self.textbox, MessageDirection::ToWidget, *v)
            }
        };
        send_sync_message(ui, msg);
        if let Ok(value) = value.try_into() {
            let (index, items) = make_index_and_value_list(prop, value, &mut ui.build_ctx());
            ui.send_message(DropdownListMessage::items(
                self.list,
                MessageDirection::ToWidget,
                items,
            ));
            send_sync_message(
                ui,
                DropdownListMessage::selection(self.list, MessageDirection::ToWidget, Some(index)),
            );
        }
    }
    fn on_numeric_message(
        &self,
        prop: Option<&TileSetPropertyLayer>,
        element: TileSetPropertyValueElement,
        value: NamableValue,
        ui: &mut UserInterface,
    ) {
        ui.send_message(TileSetPropertyValueMessage::value(
            self.handle,
            MessageDirection::FromWidget,
            element,
        ));
        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.list,
                MessageDirection::ToWidget,
                Some(find_list_index(prop, value)),
            ),
        );
    }
    /// Handle the given message, which might be relevant to some widget in the field.
    pub fn on_ui_message(
        &mut self,
        prop: Option<&TileSetPropertyLayer>,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) {
        if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.textbox
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(TileSetPropertyValueMessage::value(
                    self.handle,
                    MessageDirection::FromWidget,
                    TileSetPropertyValueElement::String(text.into()),
                ));
            }
        } else if let Some(NumericUpDownMessage::<i8>::Value(v)) = message.data() {
            if message.destination() == self.textbox
                && message.direction() == MessageDirection::FromWidget
            {
                self.on_numeric_message(
                    prop,
                    TileSetPropertyValueElement::I8(*v),
                    NamableValue::I8(*v),
                    ui,
                );
            }
        } else if let Some(NumericUpDownMessage::<i32>::Value(v)) = message.data() {
            if message.destination() == self.textbox
                && message.direction() == MessageDirection::FromWidget
            {
                self.on_numeric_message(
                    prop,
                    TileSetPropertyValueElement::I32(*v),
                    NamableValue::I32(*v),
                    ui,
                );
            }
        } else if let Some(NumericUpDownMessage::<f32>::Value(v)) = message.data() {
            if message.destination() == self.textbox
                && message.direction() == MessageDirection::FromWidget
            {
                self.on_numeric_message(
                    prop,
                    TileSetPropertyValueElement::F32(*v),
                    NamableValue::F32(*v),
                    ui,
                );
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.list
                && message.direction() == MessageDirection::FromWidget
                && *index > 0
            {
                if let Some(v) = prop.and_then(|p| p.named_values.get(index - 1)) {
                    ui.send_message(TileSetPropertyValueMessage::value(
                        self.handle,
                        MessageDirection::FromWidget,
                        v.value.into(),
                    ));
                    let msg = match v.value {
                        NamableValue::I32(v) => {
                            NumericUpDownMessage::value(self.textbox, MessageDirection::ToWidget, v)
                        }
                        NamableValue::F32(v) => {
                            NumericUpDownMessage::value(self.textbox, MessageDirection::ToWidget, v)
                        }
                        NamableValue::I8(v) => {
                            NumericUpDownMessage::value(self.textbox, MessageDirection::ToWidget, v)
                        }
                    };
                    send_sync_message(ui, msg);
                }
            }
        }
    }
}

fn make_item(text: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    DecoratorBuilder::new(BorderBuilder::new(
        WidgetBuilder::new().with_child(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(text)
                .build(ctx),
        ),
    ))
    .build(ctx)
}

/// A field that allows the user to choose a property from a [`TileSet`].
/// The property is represented as UUID internally, but the user
/// is given a dropdown list of property names.
#[derive(Debug, Default)]
pub struct MacroPropertyField {
    prop_type: TileSetPropertyType,
    handle: Handle<UiNode>,
    list: Handle<UiNode>,
}

fn make_index_and_items(
    prop_type: TileSetPropertyType,
    value: Option<&Uuid>,
    tile_set: Option<&TileSet>,
    ctx: &mut BuildContext,
) -> (usize, Vec<Handle<UiNode>>) {
    let mut items = vec![make_item("None", ctx)];
    let mut index = value.is_none().then_some(0);
    if let Some(tile_set) = tile_set {
        for (i, prop) in tile_set
            .properties
            .iter()
            .filter(|p| p.prop_type == prop_type)
            .enumerate()
        {
            if Some(&prop.uuid) == value {
                index = Some(i + 1);
            }
            items.push(make_item(&prop.name, ctx));
        }
    }
    let index = if let Some(index) = index {
        index
    } else {
        let index = items.len();
        items.push(make_item(UNKNOWN_PROPERTY, ctx));
        index
    };
    (index, items)
}

impl MacroPropertyField {
    /// -`label`: The string that names this field on the left side.
    /// -`prop_type`: The type of value that the property is expected to hold.
    ///     Only properties that match this type will be listed.
    /// -`value`: The UUID of the currently selected property, or None.
    /// -`tile_set`: The tile set to search for properties.
    pub fn new(
        widget_builder: WidgetBuilder,
        label: String,
        prop_type: TileSetPropertyType,
        value: Option<&Uuid>,
        tile_set: Option<&TileSet>,
        ctx: &mut BuildContext,
    ) -> Self {
        let label = TextBuilder::new(WidgetBuilder::new())
            .with_text(label)
            .build(ctx);
        let (index, items) = make_index_and_items(prop_type, value, tile_set, ctx);
        let list = DropdownListBuilder::new(WidgetBuilder::new().on_column(1))
            .with_items(items)
            .with_selected(index)
            .build(ctx);
        let handle = GridBuilder::new(widget_builder.with_child(label).with_child(list))
            .add_column(Column::strict(PROPERTY_LABEL_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::auto())
            .build(ctx);
        Self {
            prop_type,
            handle,
            list,
        }
    }
    /// The handle of the widget that holds the overall field.
    pub fn handle(&self) -> Handle<UiNode> {
        self.handle
    }
    /// Update the field's widgets to reflect any changes in the tile set.
    /// -`value`: The UUID of the currently selected property.
    pub fn sync(&self, value: Option<&Uuid>, tile_set: Option<&TileSet>, ui: &mut UserInterface) {
        let (index, items) =
            make_index_and_items(self.prop_type, value, tile_set, &mut ui.build_ctx());
        ui.send_message(DropdownListMessage::items(
            self.list,
            MessageDirection::ToWidget,
            items,
        ));
        send_sync_message(
            ui,
            DropdownListMessage::selection(self.list, MessageDirection::ToWidget, Some(index)),
        );
    }
    /// Handle the given message, which might be relevant to some widget in the field.
    pub fn on_ui_message(
        &mut self,
        tile_set: &TileSet,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) {
        if let Some(DropdownListMessage::SelectionChanged(index)) = message.data() {
            if message.destination() == self.list
                && message.direction() == MessageDirection::FromWidget
            {
                let id = if let Some(index) = *index {
                    if index > 0 {
                        tile_set
                            .properties
                            .iter()
                            .filter(|p| p.prop_type == self.prop_type)
                            .nth(index - 1)
                            .map(|p| p.uuid)
                    } else {
                        None
                    }
                } else {
                    None
                };
                ui.send_message(TileSetPropertyMessage::property_id(
                    self.handle,
                    MessageDirection::FromWidget,
                    id,
                ));
            }
        }
    }
}
