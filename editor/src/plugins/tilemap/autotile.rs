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
    asset::{
        untyped::{ResourceKind, UntypedResource},
        Resource, ResourceData,
    },
    autotile::{ConstraintFillRules, NeededTerrain, TerrainSource},
    core::swap_hash_map_entry,
    fxhash::FxHashMap,
    gui::{
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        formatted_text::WrapMode,
        stack_panel::StackPanelBuilder,
    },
    rand::thread_rng,
    scene::tilemap::{
        brush::TileMapBrushResource,
        tileset::{
            TileSetPropertyF32, TileSetPropertyId, TileSetPropertyNine, TileSetPropertyType,
            TileSetPropertyValueElement,
        },
        MacroTilesUpdate, TileSetAutoTileConstraint, TileSetAutoTileContext, TileSetAutoTiler,
        TileSetConstraintMap, TileSetPatternSource, TileTerrainId,
    },
};

use crate::{
    command::{Command, CommandContext, CommandTrait},
    send_sync_message,
};

use super::*;

const PATTERN_PROP_DESC: &str = concat!("Choose a nine-slice property from the tile set. ",
    "This property will provide the pattern that the autotiler uses to know whether two tiles match along each edge. ",
    "The central value of the nine is used to determine which tiles are permited in particular cell and how tiles are prioritized.");

const FREQUENCY_PROP_DESC: &str = concat!("Choose a float property from the tile set. ",
    "This property will provide the frequency that the autotiler uses to know know often to choose a tile when there is more than one ",
    "tile with the same pattern.");

#[derive(Default)]
pub struct AutoTileMacro {
    pattern_list: MacroPropertyField,
    frequency_list: MacroPropertyField,
    context: TileSetAutoTileContext,
    constraints: TileSetConstraintMap,
    autotiler: TileSetAutoTiler,
}

#[derive(Default, Debug, Clone, Visit, Reflect)]
struct CellData {
    terrain_id: TileTerrainId,
    fill: ConstraintFillRules,
}

#[derive(Debug, Default, Clone, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "b320543d-3df0-43fd-b0d9-60a398f49853")]
pub(super) struct AutoTileInstance {
    frequency_property: Option<TileSetPropertyF32>,
    pattern_property: Option<TileSetPropertyNine>,
    cells: FxHashMap<TileDefinitionHandle, CellData>,
    #[reflect(hidden)]
    #[visit(skip)]
    widgets: InstanceCellWidgets,
}

#[derive(Debug, Default, Clone)]
struct InstanceCellWidgets {
    handle: Handle<UiNode>,
    value_field: MacroPropertyValueField,
    adjacent_toggle: Handle<UiNode>,
    diagonal_toggle: Handle<UiNode>,
}

impl ResourceData for AutoTileInstance {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, _path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        Err("Saving is not supported!".to_string().into())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}

pub struct TileSetTerrainSource<'a, 'b> {
    update: &'a MacroTilesUpdate,
    instance: &'b AutoTileInstance,
}

impl TerrainSource for TileSetTerrainSource<'_, '_> {
    type Position = Vector2<i32>;
    type Terrain = TileTerrainId;

    fn iter(&self) -> impl Iterator<Item = NeededTerrain<Vector2<i32>, TileTerrainId>> + '_ {
        self.update.iter().filter_map(|(p, v)| {
            let cell_data = self.instance.cells.get(v.as_ref()?.brush_cell.as_ref()?)?;
            Some(NeededTerrain {
                position: *p,
                terrain: cell_data.terrain_id,
                fill: cell_data.fill,
            })
        })
    }

    fn contains_position(&self, position: &Self::Position) -> bool {
        let Some(el) = self.update.get(position).cloned().flatten() else {
            return false;
        };
        let Some(brush_cell) = el.brush_cell else {
            return false;
        };
        self.instance.cells.contains_key(&brush_cell)
    }
}

impl BrushMacro for AutoTileMacro {
    fn uuid(&self) -> &Uuid {
        &uuid!("ab05f992-2591-4729-8164-7b5cc1141d72")
    }

    fn name(&self) -> &'static str {
        "Autotiler"
    }

    fn on_instance_ui_message(
        &mut self,
        context: &BrushMacroInstance,
        message: &UiMessage,
        editor: &mut Editor,
    ) {
        let ui = editor.engine.user_interfaces.first_mut();
        let Some(tile_set) = context.tile_set() else {
            return;
        };
        if let Some(TileSetPropertyMessage(uuid)) = message.data() {
            if message.destination() == self.pattern_list.handle() {
                editor.message_sender.do_command(SetPatternPropCommand {
                    brush: context.brush.clone(),
                    instance: context.settings().unwrap(),
                    data: uuid.map(TileSetPropertyNine),
                });
            } else if message.destination() == self.frequency_list.handle() {
                editor.message_sender.do_command(SetFrequencyPropCommand {
                    brush: context.brush.clone(),
                    instance: context.settings().unwrap(),
                    data: uuid.map(TileSetPropertyF32),
                });
            }
        } else {
            self.pattern_list
                .on_ui_message(&tile_set.data_ref(), message, ui);
            self.frequency_list
                .on_ui_message(&tile_set.data_ref(), message, ui);
        }
    }

    fn on_cell_ui_message(
        &mut self,
        context: &MacroMessageContext,
        message: &UiMessage,
        editor: &mut Editor,
    ) {
        let Some(cell) = context.cell else {
            return;
        };
        let ui = editor.engine.user_interfaces.first_mut();
        for r in context.instances_with_uuid(*self.uuid()) {
            let instance = r.try_cast::<AutoTileInstance>().unwrap();
            let mut settings = instance.data_ref();
            let prop_id = settings
                .pattern_property
                .as_ref()
                .map(|prop| prop.property_uuid());
            let tile_set = context.tile_set();
            let tile_set = tile_set.as_ref().map(|t| t.data_ref());
            let prop =
                prop_id.and_then(|id| tile_set.as_ref().and_then(|set| set.find_property(*id)));
            settings
                .widgets
                .value_field
                .on_ui_message(prop, message, ui);
            let cell_data = settings.cells.get(&cell).cloned().unwrap_or_default();
            if let Some(&TileSetPropertyValueMessage(TileSetPropertyValueElement::I8(v))) =
                message.data()
            {
                if message.destination() == settings.widgets.value_field.handle() {
                    drop(settings);
                    editor.message_sender.do_command(SetCellCommand {
                        brush: context.brush.clone(),
                        instance,
                        cell,
                        data: Some(CellData {
                            terrain_id: v,
                            ..cell_data
                        }),
                    });
                }
            } else if let Some(&CheckBoxMessage::Check(Some(checked))) = message.data() {
                if message.destination() == settings.widgets.adjacent_toggle {
                    drop(settings);
                    editor.message_sender.do_command(SetCellCommand {
                        brush: context.brush.clone(),
                        instance,
                        cell,
                        data: Some(CellData {
                            fill: ConstraintFillRules {
                                include_adjacent: checked,
                                ..cell_data.fill
                            },
                            ..cell_data
                        }),
                    });
                } else if message.destination() == settings.widgets.diagonal_toggle {
                    drop(settings);
                    editor.message_sender.do_command(SetCellCommand {
                        brush: context.brush.clone(),
                        instance,
                        cell,
                        data: Some(CellData {
                            fill: ConstraintFillRules {
                                include_diagonal: checked,
                                ..cell_data.fill
                            },
                            ..cell_data
                        }),
                    });
                }
            }
        }
    }

    fn create_instance(&self, _brush: &TileMapBrushResource) -> Option<UntypedResource> {
        Some(UntypedResource::new_ok(
            ResourceKind::Embedded,
            AutoTileInstance::default(),
        ))
    }

    fn can_create_cell(&self) -> bool {
        true
    }

    fn fill_cell_set(
        &self,
        context: &BrushMacroInstance,
        cell_set: &mut FxHashSet<TileDefinitionHandle>,
    ) {
        let Some(data) = context
            .settings
            .as_ref()
            .and_then(|r| r.try_cast::<AutoTileInstance>())
        else {
            return;
        };
        let data = data.data_ref();
        cell_set.extend(data.cells.keys());
    }

    fn create_cell(&self, context: &BrushMacroCell) -> Option<Command> {
        let instance = context.settings()?;
        let cell = context.cell?;
        Some(Command::new(SetCellCommand {
            brush: context.brush.clone(),
            cell,
            instance,
            data: Some(CellData::default()),
        }))
    }

    fn remove_cell(&self, context: &BrushMacroCell) -> Option<Command> {
        let instance = context.settings()?;
        let cell = context.cell?;
        Some(Command::new(SetCellCommand {
            brush: context.brush.clone(),
            cell,
            instance,
            data: None,
        }))
    }

    fn build_instance_editor(
        &mut self,
        context: &BrushMacroInstance,
        ctx: &mut BuildContext,
    ) -> Option<Handle<UiNode>> {
        let instance = context.settings::<AutoTileInstance>().unwrap();
        let instance = instance.data_ref();
        let pattern_id = instance
            .pattern_property
            .as_ref()
            .map(|p| p.property_uuid());
        let frequency_id = instance
            .frequency_property
            .as_ref()
            .map(|p| p.property_uuid());
        let tile_set = context.tile_set();
        let tile_set = tile_set.as_ref().map(|t| t.data_ref());
        let tile_set = tile_set.as_deref();
        self.pattern_list = MacroPropertyField::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(5.0)),
            "Pattern Property".into(),
            TileSetPropertyType::NineSlice,
            pattern_id,
            tile_set,
            ctx,
        );
        self.frequency_list = MacroPropertyField::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(5.0)),
            "Frequency Property".into(),
            TileSetPropertyType::F32,
            frequency_id,
            tile_set,
            ctx,
        );
        let pattern_prop_help_text =
            TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(5.0)))
                .with_wrap(WrapMode::Word)
                .with_text(PATTERN_PROP_DESC)
                .build(ctx);
        let freq_prop_help_text =
            TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(5.0)))
                .with_wrap(WrapMode::Word)
                .with_text(FREQUENCY_PROP_DESC)
                .build(ctx);
        let handle = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(5.0))
                .with_child(pattern_prop_help_text)
                .with_child(self.pattern_list.handle())
                .with_child(freq_prop_help_text)
                .with_child(self.frequency_list.handle()),
        )
        .build(ctx);
        Some(handle)
    }

    fn build_cell_editor(
        &mut self,
        context: &BrushMacroCell,
        ctx: &mut BuildContext,
    ) -> Option<Handle<UiNode>> {
        let instance = context.settings::<AutoTileInstance>().unwrap();
        let mut instance = instance.data_ref();
        let prop_id = instance
            .pattern_property
            .as_ref()
            .map(|p| p.property_uuid());
        let value = context.cell.and_then(|c| instance.cells.get(&c));
        let tile_set = context.tile_set();
        let tile_set = tile_set.as_ref().map(|set| set.data_ref());
        let prop = if let (Some(tile_set), Some(uuid)) = (tile_set.as_ref(), prop_id) {
            tile_set.find_property(*uuid)
        } else {
            None
        };
        let terrain_id = value.map(|d| d.terrain_id).unwrap_or_default();
        let adjacent = value.map(|d| d.fill.include_adjacent).unwrap_or_default();
        let diagonal = value.map(|d| d.fill.include_diagonal).unwrap_or_default();
        let value_field = MacroPropertyValueField::new(
            WidgetBuilder::new(),
            "Terrain".into(),
            TileSetPropertyValueElement::I8(terrain_id),
            prop,
            ctx,
        );
        let adjacent_toggle = CheckBoxBuilder::new(WidgetBuilder::new())
            .checked(Some(adjacent))
            .build(ctx);
        let diagonal_toggle = CheckBoxBuilder::new(WidgetBuilder::new())
            .checked(Some(diagonal))
            .build(ctx);
        let adjacent_field = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_column(1))
                        .with_text("Adjacent")
                        .build(ctx),
                )
                .with_child(adjacent_toggle),
        )
        .add_row(Row::auto())
        .add_column(Column::strict(20.0))
        .add_column(Column::stretch())
        .build(ctx);
        let diagonal_field = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().on_column(1))
                        .with_text("Diagonal")
                        .build(ctx),
                )
                .with_child(diagonal_toggle),
        )
        .add_row(Row::auto())
        .add_column(Column::strict(20.0))
        .add_column(Column::stretch())
        .build(ctx);
        let handle = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_visibility(value.is_some())
                .with_child(value_field.handle())
                .with_child(adjacent_field)
                .with_child(diagonal_field),
        )
        .build(ctx);
        instance.widgets.handle = handle;
        instance.widgets.value_field = value_field;
        instance.widgets.adjacent_toggle = adjacent_toggle;
        instance.widgets.diagonal_toggle = diagonal_toggle;
        Some(handle)
    }

    fn sync_instance_editor(&mut self, context: &BrushMacroInstance, ui: &mut UserInterface) {
        let Some(instance) = context.settings::<AutoTileInstance>() else {
            return;
        };
        let instance = instance.data_ref();
        let pattern_id = instance
            .pattern_property
            .as_ref()
            .map(|p| p.property_uuid());
        let frequency_id = instance
            .frequency_property
            .as_ref()
            .map(|p| p.property_uuid());
        let tile_set = context.tile_set();
        let tile_set = tile_set.as_ref().map(|t| t.data_ref());
        let tile_set = tile_set.as_deref();
        self.pattern_list.sync(pattern_id, tile_set, ui);
        self.frequency_list.sync(frequency_id, tile_set, ui);
    }

    fn sync_cell_editors(&mut self, context: &MacroMessageContext, ui: &mut UserInterface) {
        for r in context.instances_with_uuid(*self.uuid()) {
            let settings = r.try_cast::<AutoTileInstance>().unwrap();
            let settings = settings.data_ref();
            let prop_id = settings
                .pattern_property
                .as_ref()
                .map(|prop| prop.property_uuid());
            let tile_set = context.tile_set();
            let tile_set = tile_set.as_ref().map(|t| t.data_ref());
            let prop =
                prop_id.and_then(|id| tile_set.as_ref().and_then(|set| set.find_property(*id)));
            let cell_data = context
                .cell
                .as_ref()
                .and_then(|cell| settings.cells.get(cell));
            let value = cell_data.map(|cd| cd.terrain_id).unwrap_or_default();
            let value = TileSetPropertyValueElement::I8(value);
            let adjacent = cell_data.map(|d| d.fill.include_adjacent);
            let diagonal = cell_data.map(|d| d.fill.include_diagonal);
            settings.widgets.value_field.sync(value, prop, ui);
            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    settings.widgets.adjacent_toggle,
                    MessageDirection::ToWidget,
                    adjacent,
                ),
            );
            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    settings.widgets.diagonal_toggle,
                    MessageDirection::ToWidget,
                    diagonal,
                ),
            );
            ui.send_message(WidgetMessage::visibility(
                settings.widgets.handle,
                MessageDirection::ToWidget,
                cell_data.is_some(),
            ));
        }
    }

    fn begin_update(
        &mut self,
        context: &BrushMacroInstance,
        stamp: &Stamp,
        tile_map: &TileMapContext,
    ) {
        let Some(tile_set) = tile_map.tile_set() else {
            self.context.clear();
            return;
        };
        if context.tile_set().as_deref() != Some(tile_set) {
            self.context.clear();
            return;
        }
        let instance = context.settings::<AutoTileInstance>().unwrap();
        let instance = instance.data_ref();
        let Some(pattern_property) = instance.pattern_property else {
            self.context.clear();
            return;
        };
        let frequency_property = instance.frequency_property;
        // Check that at least one cell in the stamp is part of this instance.
        if !stamp.values().any(|StampElement { brush_cell, .. }| {
            brush_cell.is_some_and(|handle| instance.cells.contains_key(&handle))
        }) {
            self.context.clear();
            return;
        }
        Log::verify(self.context.fill_pattern_map(
            &tile_set.data_ref(),
            pattern_property,
            frequency_property,
        ));
    }

    fn amend_update(
        &mut self,
        context: &BrushMacroInstance,
        update: &mut MacroTilesUpdate,
        tile_map: &TileMap,
    ) {
        if self.context.is_empty() {
            return;
        }
        let Some(instance) = &context.settings() else {
            return;
        };
        let instance = instance.data_ref();
        let instance = &instance;
        let terrains = TileSetTerrainSource { update, instance };
        let Some(tile_set) = tile_map.tile_set() else {
            return;
        };
        let tile_set = tile_set.data_ref();
        let tile_set = &tile_set;
        let Some(property_id) = instance.pattern_property else {
            return;
        };
        let patterns = TileSetPatternSource {
            tile_map,
            tile_set,
            update,
            property_id,
        };
        self.constraints.fill_from(&terrains, &patterns);
        self.autotiler.clear();
        self.autotiler.autotile(&TileSetAutoTileConstraint {
            position_constraints: &self.constraints,
            pattern_constraints: &self.context.patterns,
        });
        self.autotiler
            .apply_autotile_to_update(&mut thread_rng(), &self.context.values, update);
    }

    fn create_command(
        &mut self,
        _context: &BrushMacroInstance,
        _update: &mut MacroTilesUpdate,
        _tile_map: &TileMapContext,
    ) -> Option<Command> {
        None
    }
}

#[derive(Debug)]
struct SetCellCommand {
    pub brush: TileMapBrushResource,
    pub instance: Resource<AutoTileInstance>,
    pub cell: TileDefinitionHandle,
    pub data: Option<CellData>,
}

impl SetCellCommand {
    fn swap(&mut self) {
        let mut instance = self.instance.data_ref();
        swap_hash_map_entry(instance.cells.entry(self.cell), &mut self.data);
        self.brush.data_ref().change_flag.set();
    }
}

impl CommandTrait for SetCellCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Update Autotile Cell".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
struct SetPatternPropCommand {
    pub brush: TileMapBrushResource,
    pub instance: Resource<AutoTileInstance>,
    pub data: Option<TileSetPropertyNine>,
}

impl SetPatternPropCommand {
    fn swap(&mut self) {
        let mut instance = self.instance.data_ref();
        std::mem::swap(&mut instance.pattern_property, &mut self.data);
        self.brush.data_ref().change_flag.set();
    }
}

impl CommandTrait for SetPatternPropCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Update Autotile Property".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
struct SetFrequencyPropCommand {
    pub brush: TileMapBrushResource,
    pub instance: Resource<AutoTileInstance>,
    pub data: Option<TileSetPropertyF32>,
}

impl SetFrequencyPropCommand {
    fn swap(&mut self) {
        let mut instance = self.instance.data_ref();
        std::mem::swap(&mut instance.frequency_property, &mut self.data);
        self.brush.data_ref().change_flag.set();
    }
}

impl CommandTrait for SetFrequencyPropCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Update Autotile Property".into()
    }

    fn execute(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _context: &mut dyn CommandContext) {
        self.swap();
    }
}
