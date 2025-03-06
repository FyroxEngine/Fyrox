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

//! The [`InteractionMode`] for editing a tile map.

use commands::{MoveMapTileCommand, SetMapTilesCommand};
use fyrox::{
    asset::untyped::UntypedResource,
    fxhash::FxHashMap,
    scene::tilemap::{
        brush::TileMapBrushResource,
        tileset::{OptionTileSet, TileSetRef},
        MacroTilesUpdate, OptionTileRect, TileCursorEffect, TileEraseEffect, TileMapData,
        TileOverlayEffect, TileSelectionEffect, TileSource, TileUpdateEffect, TilesUpdate,
        TransTilesUpdate,
    },
};

use crate::{
    command::{Command, CommandGroup},
    make_color_material,
};

use super::*;

const CURSOR_COLOR: Color = Color::from_rgba(255, 255, 255, 30);
const SELECT_COLOR: Color = Color::from_rgba(255, 255, 0, 200);
const ERASE_COLOR: Color = Color::from_rgba(255, 0, 0, 255);
const SELECT_BORDER_THICKNESS: f32 = 0.1;
const ERASE_BORDER_THICKNESS: f32 = 0.1;

const PICK_KEY: KeyCode = KeyCode::Digit1;
const ERASE_KEY: KeyCode = KeyCode::Digit2;
const RECT_KEY: KeyCode = KeyCode::Digit3;
const DEL_KEY: KeyCode = KeyCode::Delete;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseMode {
    None,
    Dragging,
    Drawing,
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "33fa8ef9-a29c-45d4-a493-79571edd870a")]
pub struct TileMapInteractionMode {
    tile_map: Handle<Node>,
    /// The state that is shared between this interaction mode and the
    /// tile map control panel, allowing this object to be aware of the chosen tool
    /// and the selected stamp.
    state: TileDrawStateRef,
    /// List if tools that can be added to a tile map brush to assist with tile map editing.
    brush_macro_list: BrushMacroListRef,
    /// Temporary space to store tiles that are in the process of being amended by macros.
    macro_update: MacroTilesUpdate,
    /// A copy of the list of macro instance resources taken from the current brush.
    /// This is temporary storage that is used to avoid repeated allocation.
    macro_instance_list: Vec<(Uuid, Option<UntypedResource>)>,
    /// A copy of the current drawing mode that is made whenever the user
    /// presses the mouse button. While the actual drawing mode may change
    /// during a mouse stroke, this value never will, so nothing breaks by changing
    /// tool in the middle of a mouse stroke.
    current_tool: DrawingMode,
    /// The cell that started the current mouse motion.
    click_grid_position: Option<Vector2<i32>>,
    /// The most recent cell of the current mouse motion.
    /// It is used to determine whether the mouse has moved.
    current_grid_position: Option<Vector2<i32>>,
    /// The sender for sending commands to modify the tile map.
    sender: MessageSender,
    /// The current state of mouse operations.
    mouse_mode: MouseMode,
    /// These are the positions of tiles that are in the process of being selected, but not actually selected.
    /// Tile selection is a two-stage process to give the user a smooth experience. The actually selected tiles
    /// are stored in the [`TileDrawState::selection`] so that all interested parties can see what is currently
    /// selected. In contrast, this set contains a record of what was selected before the user began the current
    /// mouse motion, if the user held shift to prevent that selection from being removed.
    ///
    /// In order to calculate the actual selection, this set is combined with the rect created by the current
    /// mouse motion.
    selecting: FxHashSet<Vector2<i32>>,
    cursor_effect: Arc<Mutex<TileCursorEffect>>,
    select_effect: Arc<Mutex<TileSelectionEffect>>,
    erase_select_effect: Arc<Mutex<TileSelectionEffect>>,
    overlay_effect: Arc<Mutex<TileOverlayEffect>>,
    erase_effect: Arc<Mutex<TileEraseEffect>>,
    update_effect: Arc<Mutex<TileUpdateEffect>>,
}

impl TileMapInteractionMode {
    pub fn new(
        tile_map: Handle<Node>,
        state: TileDrawStateRef,
        brush_macro_list: BrushMacroListRef,
        sender: MessageSender,
    ) -> Self {
        let cursor_material = make_color_material(CURSOR_COLOR);
        let select_material = make_color_material(SELECT_COLOR);
        let erase_material = make_color_material(ERASE_COLOR);
        Self {
            tile_map,
            state,
            brush_macro_list,
            macro_update: MacroTilesUpdate::default(),
            macro_instance_list: Vec::default(),
            current_tool: DrawingMode::Pick,
            click_grid_position: None,
            current_grid_position: None,
            sender,
            mouse_mode: MouseMode::None,
            selecting: FxHashSet::default(),
            overlay_effect: Arc::new(Mutex::new(TileOverlayEffect {
                active: false,
                offset: Vector2::default(),
                tiles: FxHashMap::default(),
            })),
            cursor_effect: Arc::new(Mutex::new(TileCursorEffect {
                material: Some(cursor_material),
                position: None,
            })),
            update_effect: Arc::new(Mutex::new(TileUpdateEffect {
                active: true,
                update: TransTilesUpdate::default(),
            })),
            erase_select_effect: Arc::new(Mutex::new(TileSelectionEffect {
                material: Some(erase_material),
                offset: None,
                positions: FxHashSet::default(),
                thickness: ERASE_BORDER_THICKNESS,
            })),
            erase_effect: Arc::new(Mutex::new(TileEraseEffect {
                positions: FxHashSet::default(),
            })),
            select_effect: Arc::new(Mutex::new(TileSelectionEffect {
                material: Some(select_material),
                offset: Some(Vector2::default()),
                positions: FxHashSet::default(),
                thickness: SELECT_BORDER_THICKNESS,
            })),
        }
    }
    pub fn on_tile_map_selected(&mut self, tile_map: &mut TileMap) {
        tile_map.before_effects.clear();
        tile_map.before_effects.extend([
            self.overlay_effect.clone() as TileMapEffectRef,
            self.update_effect.clone() as TileMapEffectRef,
            self.erase_effect.clone() as TileMapEffectRef,
        ]);
        tile_map.after_effects.clear();
        tile_map.after_effects.extend([
            self.cursor_effect.clone() as TileMapEffectRef,
            self.erase_select_effect.clone() as TileMapEffectRef,
            self.select_effect.clone() as TileMapEffectRef,
        ]);
    }
    fn pick_grid(
        &self,
        scene: &Scene,
        game_scene: &GameScene,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) -> Option<Vector2<i32>> {
        let tile_map = scene.graph.try_get_of_type::<TileMap>(self.tile_map)?;
        let global_transform = tile_map.global_transform();

        let camera = scene.graph[game_scene.camera_controller.camera].as_camera();
        let ray = camera.make_ray(mouse_position, frame_size);

        let plane =
            Plane::from_normal_and_point(&global_transform.look(), &global_transform.position())
                .unwrap_or_default();

        ray.plane_intersection_point(&plane)
            .map(|intersection| tile_map.world_to_grid(intersection))
    }
    pub fn sync_to_state(&mut self) {
        let state = self.state.lock();
        if state.selection_node() != self.tile_map {
            self.select_effect.lock().positions.clear();
            self.selecting.clear();
        }
        match state.drawing_mode {
            DrawingMode::Draw => {
                let mut overlay = self.overlay_effect.lock();
                let mut erase_overlay = self.erase_select_effect.lock();
                overlay.tiles.clear();
                erase_overlay.positions.clear();
                let stamp = &state.stamp;
                let tile_set = state.tile_set.as_ref().map(TileSetRef::new);
                if let Some(mut tile_set) = tile_set {
                    let tile_set = tile_set.as_loaded();
                    for (pos, StampElement { handle, .. }) in stamp.iter() {
                        let handle = tile_set
                            .get_transformed_version(stamp.transformation(), *handle)
                            .unwrap_or(*handle);
                        let _ = overlay.tiles.insert(pos, handle);
                    }
                }
            }
            DrawingMode::Erase => {
                let mut overlay = self.overlay_effect.lock();
                let mut erase_overlay = self.erase_select_effect.lock();
                overlay.tiles.clear();
                erase_overlay.positions.clear();
                if state.stamp.is_empty() {
                    let _ = erase_overlay.positions.insert(Vector2::new(0, 0));
                } else {
                    erase_overlay.positions.extend(state.stamp.keys());
                }
            }
            DrawingMode::Pick => {
                if self.mouse_mode == MouseMode::None {
                    self.overlay_effect.lock().tiles.clear();
                    self.erase_select_effect.lock().positions.clear();
                }
            }
            _ => {
                self.overlay_effect.lock().tiles.clear();
                self.erase_select_effect.lock().positions.clear();
            }
        }
    }
    fn delete(&mut self) {
        let sel = &self.select_effect.lock().positions;
        if sel.is_empty() {
            return;
        }
        let mut update = TilesUpdate::default();
        for position in sel {
            let _ = update.insert(*position, None);
        }
        self.sender.do_command(SetMapTilesCommand {
            tile_map: self.tile_map,
            tiles: update,
        });
    }
}

fn update_select(
    tile_map: &TileMap,
    selected: &mut FxHashSet<Vector2<i32>>,
    selecting: &FxHashSet<Vector2<i32>>,
    state: &mut TileDrawStateGuardMut<'_>,
    start: Vector2<i32>,
    end: Vector2<i32>,
) {
    let rect = OptionTileRect::from_points(start, end);
    let sel = state.selection_positions_mut();
    sel.clone_from(selecting);
    if selecting.contains(&start) {
        for pos in rect.iter() {
            sel.remove(&pos);
        }
    } else {
        sel.extend(rect.iter());
    }
    selected.clone_from(sel);
    let Some(tiles) = tile_map.tiles().map(|r| r.data_ref()) else {
        return;
    };
    let Some(tiles) = tiles.as_loaded_ref() else {
        return;
    };
    state.update_stamp(None, tile_map.tile_set().cloned(), |p| {
        tiles.get(p).map(|t| t.into())
    });
}

fn macro_begin(
    tile_map_context: &TileMapContext,
    update: &mut TransTilesUpdate,
    macro_update: &mut MacroTilesUpdate,
    stamp: &Stamp,
    macros: &mut BrushMacroList,
    macro_instances: &mut Vec<(Uuid, Option<UntypedResource>)>,
) {
    let Some(brush) = stamp.brush() else {
        return;
    };
    let tile_map_handle = tile_map_context.node;
    let Some(tile_map) = tile_map_context.engine.scenes[tile_map_context.scene].graph
        [tile_map_handle]
        .cast::<TileMap>()
    else {
        return;
    };
    let brush_guard = brush.data_ref();
    macro_instances.clear();
    macro_instances.extend(
        brush_guard
            .macros
            .iter()
            .map(|m| (m.macro_id, m.settings.clone())),
    );
    if macro_instances.is_empty() {
        return;
    }
    let tile_set = brush_guard.tile_set();
    let mut tile_set_guard = tile_set.as_ref().map(|ts| ts.data_ref());
    let tile_set = OptionTileSet(tile_set_guard.as_deref_mut());
    update.fill_macro_tiles_update(&tile_set, macro_update);
    drop(tile_set_guard);
    drop(brush_guard);
    let mut context = BrushMacroInstance {
        brush: brush.clone(),
        settings: None,
    };
    for (id, settings) in macro_instances.drain(..) {
        let Some(brush_macro) = macros.get_by_uuid_mut(&id) else {
            continue;
        };
        context.settings = settings;
        brush_macro.begin_update(&context, stamp, tile_map_context);
        brush_macro.amend_update(&context, macro_update, tile_map);
    }
    macro_update.fill_trans_tiles_update(update);
}

fn macro_amend_update(
    tile_map: &TileMap,
    update: &mut TransTilesUpdate,
    macro_update: &mut MacroTilesUpdate,
    brush: &TileMapBrushResource,
    macros: &mut BrushMacroList,
    macro_instances: &mut Vec<(Uuid, Option<UntypedResource>)>,
) {
    let brush_guard = brush.data_ref();
    macro_instances.clear();
    macro_instances.extend(
        brush_guard
            .macros
            .iter()
            .map(|m| (m.macro_id, m.settings.clone())),
    );
    if macro_instances.is_empty() {
        return;
    }
    let tile_set = brush_guard.tile_set();
    let mut tile_set_guard = tile_set.as_ref().map(|ts| ts.data_ref());
    let tile_set = OptionTileSet(tile_set_guard.as_deref_mut());
    update.fill_macro_tiles_update(&tile_set, macro_update);
    drop(tile_set_guard);
    drop(brush_guard);
    let mut context = BrushMacroInstance {
        brush: brush.clone(),
        settings: None,
    };
    for (id, settings) in macro_instances.drain(..) {
        let Some(brush_macro) = macros.get_by_uuid_mut(&id) else {
            continue;
        };
        context.settings = settings;
        brush_macro.amend_update(&context, macro_update, tile_map);
    }
    macro_update.fill_trans_tiles_update(update);
}

fn macro_command_list(
    tile_map: &TileMapContext,
    update: &mut TransTilesUpdate,
    macro_update: &mut MacroTilesUpdate,
    brush: &TileMapBrushResource,
    macros: &mut BrushMacroList,
    macro_instances: &mut Vec<(Uuid, Option<UntypedResource>)>,
) -> Vec<Command> {
    let brush_guard = brush.data_ref();
    macro_instances.clear();
    macro_instances.extend(
        brush_guard
            .macros
            .iter()
            .map(|m| (m.macro_id, m.settings.clone())),
    );
    if macro_instances.is_empty() {
        return Vec::default();
    }
    let tile_set = brush_guard.tile_set();
    let mut tile_set_guard = tile_set.as_ref().map(|ts| ts.data_ref());
    let tile_set = OptionTileSet(tile_set_guard.as_deref_mut());
    update.fill_macro_tiles_update(&tile_set, macro_update);
    drop(tile_set_guard);
    drop(brush_guard);
    let mut context = BrushMacroInstance {
        brush: brush.clone(),
        settings: None,
    };
    let mut commands = Vec::default();
    for (id, settings) in macro_instances.drain(..) {
        let Some(brush_macro) = macros.get_by_uuid_mut(&id) else {
            continue;
        };
        context.settings = settings;
        if let Some(command) = brush_macro.create_command(&context, macro_update, tile_map) {
            commands.push(command);
        }
    }
    macro_update.fill_trans_tiles_update(update);
    commands
}

fn draw(
    update: &mut TransTilesUpdate,
    tiles: &TileMapData,
    tool: DrawingMode,
    state: &TileDrawStateGuard<'_>,
    start: Vector2<i32>,
    end: Vector2<i32>,
) {
    let stamp = &state.stamp;
    match tool {
        DrawingMode::Pick => (),
        DrawingMode::Editor => (),
        DrawingMode::Draw => update.draw_tiles(end, stamp),
        DrawingMode::Erase => {
            if stamp.is_empty() {
                update.erase(end);
            } else {
                update.erase_stamp(end, stamp);
            }
        }
        DrawingMode::RectFill => {
            update.clear();
            if state.random_mode {
                update.rect_fill_random(start, end, stamp);
            } else {
                update.rect_fill(start, end, stamp);
            }
        }
        DrawingMode::NineSlice => {
            update.clear();
            if state.random_mode {
                update.nine_slice_random(start, end, stamp);
            } else {
                update.nine_slice(start, end, stamp);
            }
        }
        DrawingMode::Line => {
            update.clear();
            if state.random_mode {
                update.draw_line(start, end, &RandomTileSource(stamp));
            } else {
                update.draw_line(start, end, &stamp.repeat(start, end));
            }
        }
        DrawingMode::FloodFill => {
            if state.random_mode {
                update.flood_fill(tiles, end, &RandomTileSource(stamp));
            } else {
                update.flood_fill(tiles, end, &stamp.repeat_anywhere());
            }
        }
    }
}

impl InteractionMode for TileMapInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };
        let scene_handle = game_scene.scene;
        let scene = &mut engine.scenes[scene_handle];
        let mods = engine.user_interfaces.first().keyboard_modifiers();
        let state = self.state.lock();
        self.current_tool = state.drawing_mode;
        let grid_coord = self.pick_grid(scene, game_scene, mouse_position, frame_size);
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
            return;
        };
        let Some(tiles_guard) = tile_map.tiles().map(|r| r.data_ref()) else {
            return;
        };
        let Some(tiles) = tiles_guard.as_loaded_ref() else {
            return;
        };
        let mut overlay = self.overlay_effect.lock();
        let mut erase_overlay = self.erase_select_effect.lock();
        if let Some(grid_coord) = grid_coord {
            overlay.active = true;
            overlay.offset = grid_coord;
            erase_overlay.offset = Some(grid_coord);
        } else {
            overlay.active = false;
            erase_overlay.offset = None;
        }
        self.click_grid_position = grid_coord;
        self.current_grid_position = grid_coord;
        if let Some(grid_coord) = grid_coord {
            match state.drawing_mode {
                DrawingMode::Pick => {
                    if mods.alt {
                        self.mouse_mode = MouseMode::Dragging;
                        let erased_area = &mut self.erase_effect.lock().positions;
                        overlay.tiles.clear();
                        erased_area.clear();
                        let selected = &self.select_effect.lock().positions;
                        for pos in selected.iter() {
                            let Some(handle) = tiles.get(*pos) else {
                                continue;
                            };
                            let _ = erased_area.insert(*pos);
                            let _ = overlay.tiles.insert(*pos - grid_coord, handle);
                        }
                    } else {
                        drop(tiles_guard);
                        self.mouse_mode = MouseMode::Drawing;
                        if !mods.shift {
                            self.selecting.clear();
                        }
                        let mut state = state.into_mut("TileMap start select");
                        state.set_node(self.tile_map);
                        update_select(
                            tile_map,
                            &mut self.select_effect.lock().positions,
                            &self.selecting,
                            &mut state,
                            grid_coord,
                            grid_coord,
                        );
                    }
                }
                mode => {
                    self.mouse_mode = MouseMode::Drawing;
                    let update = &mut self.update_effect.lock().update;
                    draw(update, tiles, mode, &state, grid_coord, grid_coord);
                    drop(tiles_guard);
                    if state.stamp.brush().is_some() {
                        let tile_map = TileMapContext {
                            node: self.tile_map,
                            scene: scene_handle,
                            engine,
                        };
                        macro_begin(
                            &tile_map,
                            update,
                            &mut self.macro_update,
                            &state.stamp,
                            &mut self.brush_macro_list.lock(),
                            &mut self.macro_instance_list,
                        );
                    }
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _mouse_position: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };
        let scene_handle = game_scene.scene;
        let scene = &mut engine.scenes[scene_handle];
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
            return;
        };
        let start = self.click_grid_position;
        let end = self.current_grid_position;
        self.click_grid_position = None;
        self.current_grid_position = None;

        let tile_map_handle = self.tile_map;
        match self.mouse_mode {
            MouseMode::None => (),
            MouseMode::Dragging => {
                let overlay = &mut self.overlay_effect.lock().tiles;
                if let (Some(start), Some(end)) = (start, end) {
                    let offset = end - start;
                    if offset != Vector2::new(0, 0) {
                        let tiles = overlay
                            .keys()
                            .copied()
                            .map(|p| p + start)
                            .collect::<Vec<_>>();
                        let selected = &mut self.select_effect.lock().positions;
                        selected.clear();
                        selected.extend(tiles.iter().map(|p| p + offset));
                        self.sender.do_command(MoveMapTileCommand::new(
                            tile_map_handle,
                            tiles,
                            offset,
                        ));
                    }
                }
                overlay.clear();
                self.erase_effect.lock().positions.clear();
            }
            MouseMode::Drawing => {
                let state = self.state.lock();
                if let DrawingMode::Pick = self.current_tool {
                    self.selecting
                        .clone_from(&self.select_effect.lock().positions);
                } else if let Some(tile_set) =
                    state.tile_set.as_ref().or(tile_map.tile_set()).cloned()
                {
                    let update_source = &mut self.update_effect.lock().update;
                    let tile_map_context = TileMapContext {
                        node: tile_map_handle,
                        scene: scene_handle,
                        engine,
                    };
                    let mut commands = if let Some(brush) = state.stamp.brush() {
                        macro_command_list(
                            &tile_map_context,
                            update_source,
                            &mut self.macro_update,
                            brush,
                            &mut self.brush_macro_list.lock(),
                            &mut self.macro_instance_list,
                        )
                    } else {
                        Vec::default()
                    };
                    let update =
                        update_source.build_tiles_update(&TileSetRef::new(&tile_set).as_loaded());
                    let command = SetMapTilesCommand {
                        tile_map: tile_map_handle,
                        tiles: update,
                    };
                    if !commands.is_empty() {
                        commands.push(Command::new(command));
                        self.sender.do_command(
                            CommandGroup::from(commands).with_custom_name("Draw Tiles"),
                        );
                    } else {
                        self.sender.do_command(command);
                    }
                    update_source.clear();
                }
            }
        }
        self.mouse_mode = MouseMode::None;
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        let grid_coord = self.pick_grid(scene, game_scene, mouse_position, frame_size);

        let mut overlay = self.overlay_effect.lock();
        let mut erase_overlay = self.erase_select_effect.lock();
        if let Some(grid_coord) = grid_coord {
            overlay.active = true;
            overlay.offset = grid_coord;
            erase_overlay.offset = Some(grid_coord);
        } else {
            overlay.active = false;
            erase_overlay.offset = None;
        }
        self.cursor_effect.lock().position = grid_coord;

        let Some(grid_coord) = grid_coord else {
            return;
        };
        let Some(start) = self.click_grid_position else {
            return;
        };
        let Some(end) = self.current_grid_position else {
            return;
        };

        if end == grid_coord {
            return;
        }

        let end = grid_coord;
        self.current_grid_position = Some(grid_coord);

        let tile_map_handle = self.tile_map;
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(tile_map_handle) else {
            return;
        };
        let Some(tiles_guard) = tile_map.tiles().map(|r| r.data_ref()) else {
            return;
        };
        let Some(tiles) = tiles_guard.as_loaded_ref() else {
            return;
        };

        let state = self.state.lock();

        match self.mouse_mode {
            MouseMode::None => (),
            MouseMode::Dragging => (),
            MouseMode::Drawing => {
                if let DrawingMode::Pick = self.current_tool {
                    drop(tiles_guard);
                    update_select(
                        tile_map,
                        &mut self.select_effect.lock().positions,
                        &self.selecting,
                        &mut state.into_mut("TileMap select"),
                        start,
                        end,
                    );
                } else {
                    let update = &mut self.update_effect.lock().update;
                    draw(update, tiles, self.current_tool, &state, start, end);
                    drop(tiles_guard);
                    if let Some(brush) = state.stamp.brush() {
                        macro_amend_update(
                            tile_map,
                            update,
                            &mut self.macro_update,
                            brush,
                            &mut self.brush_macro_list.lock(),
                            &mut self.macro_instance_list,
                        );
                    }
                }
            }
        }
    }

    fn on_mouse_leave(
        &mut self,
        _mouse_position: Vector2<f32>,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        self.overlay_effect.lock().active = false;
        self.erase_select_effect.lock().offset = None;
        self.cursor_effect.lock().position = None;
    }

    fn update(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
            return;
        };

        let transform = tile_map.global_transform();
        let ctx = &mut scene.drawing_context;

        let mut draw_line = |begin: Vector2<i32>, end: Vector2<i32>, color: Color| {
            ctx.add_line(Line {
                begin: transform
                    .transform_point(&Vector3::new(begin.x as f32, begin.y as f32, -0.01).into())
                    .coords,
                end: transform
                    .transform_point(&Vector3::new(end.x as f32, end.y as f32, -0.01).into())
                    .coords,
                color,
            });
        };

        // TODO: Is there a better way to make a grid?
        let size = 1000i32;
        for y in -size..size {
            draw_line(Vector2::new(-size, y), Vector2::new(size, y), Color::WHITE);
        }
        for x in -size..size {
            draw_line(Vector2::new(x, -size), Vector2::new(x, size), Color::WHITE);
        }
    }

    fn activate(&mut self, _controller: &dyn SceneController, _engine: &mut Engine) {}

    fn deactivate(&mut self, _controller: &dyn SceneController, _engine: &mut Engine) {}

    fn make_button(&mut self, ctx: &mut BuildContext, selected: bool) -> Handle<UiNode> {
        make_interaction_mode_button(
            ctx,
            include_bytes!("../../../resources/tile.png"),
            "Edit Tile Map",
            selected,
        )
    }

    fn uuid(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_hot_key_pressed(
        &mut self,
        hotkey: &HotKey,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _settings: &Settings,
    ) -> bool {
        if let HotKey::Some { code, .. } = hotkey {
            match *code {
                PICK_KEY => {
                    let state = self.state.lock();
                    if state.drawing_mode != DrawingMode::Pick {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Pick;
                    }
                    return true;
                }
                ERASE_KEY => {
                    let state = self.state.lock();
                    if state.drawing_mode != DrawingMode::Erase {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Erase;
                    }
                    return true;
                }
                RECT_KEY => {
                    let state = self.state.lock();
                    if state.drawing_mode != DrawingMode::RectFill {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::RectFill;
                    }
                    return true;
                }
                DEL_KEY => {
                    self.delete();
                    return true;
                }
                _ => (),
            }
        }
        false
    }

    fn on_hot_key_released(
        &mut self,
        hotkey: &HotKey,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _settings: &Settings,
    ) -> bool {
        if let HotKey::Some { code, .. } = hotkey {
            match *code {
                PICK_KEY => {
                    let state = self.state.lock();
                    if state.drawing_mode == DrawingMode::Pick {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                ERASE_KEY => {
                    let state = self.state.lock();
                    if state.drawing_mode == DrawingMode::Erase {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                RECT_KEY => {
                    let state = self.state.lock();
                    if state.drawing_mode == DrawingMode::RectFill {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                _ => (),
            }
        }
        false
    }
}
