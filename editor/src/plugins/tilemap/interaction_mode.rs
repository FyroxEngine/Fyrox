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
    material::MaterialResource,
    scene::tilemap::{tileset::TileSetRef, OptionTileRect, TileRenderData, TileSource},
};

use crate::make_color_material;

use super::*;

const CURSOR_COLOR: Color = Color::from_rgba(255, 255, 255, 30);
const SELECT_COLOR: Color = Color::from_rgba(255, 255, 0, 200);
const ERASE_COLOR: Color = Color::from_rgba(255, 0, 0, 255);

const PICK_KEY: KeyCode = KeyCode::Digit1;
const ERASE_KEY: KeyCode = KeyCode::Digit2;
const RECT_KEY: KeyCode = KeyCode::Digit3;

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
    /// A reference to the same editor data that is stored in the currently edited [`TileMap`].
    /// This allows the interaction mode to chane how the tile map is rendered to reflect the current editing
    /// operation, such highlighting the currently selected tiles.
    editor_data: TileMapEditorDataRef,
    /// The material used to render the cursor position in the tile map.
    cursor_material: MaterialResource,
    /// The material used to render the highlight of the selected tiles.
    select_material: MaterialResource,
    /// The material used to render the highlight of tiles that might be erased by the user's current operation.
    erase_material: MaterialResource,
}

impl TileMapInteractionMode {
    pub fn new(
        tile_map: Handle<Node>,
        state: TileDrawStateRef,
        sender: MessageSender,
        editor_data: TileMapEditorDataRef,
    ) -> Self {
        Self {
            tile_map,
            state,
            current_tool: DrawingMode::Pick,
            click_grid_position: None,
            current_grid_position: None,
            sender,
            mouse_mode: MouseMode::None,
            selecting: FxHashSet::default(),
            editor_data,
            cursor_material: make_color_material(CURSOR_COLOR),
            select_material: make_color_material(SELECT_COLOR),
            erase_material: make_color_material(ERASE_COLOR),
        }
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
        let mut editor_data = self.editor_data.lock();
        if state.selection_node() != self.tile_map {
            editor_data.selected.clear();
            self.selecting.clear();
        }
        if editor_data.select_material.is_none() {
            editor_data.select_material = Some(make_color_material(SELECT_COLOR));
        }
        match state.drawing_mode {
            DrawingMode::Draw => {
                editor_data.overlay.clear();
                editor_data.erase_overlay.clear();
                let stamp = &state.stamp;
                let tile_set = state.tile_set.as_ref().map(TileSetRef::new);
                if let Some(mut tile_set) = tile_set {
                    let tile_set = tile_set.as_loaded();
                    for (pos, handle) in stamp.iter() {
                        let data = tile_set
                            .get_transformed_render_data(stamp.transformation(), *handle)
                            .unwrap_or_else(TileRenderData::missing_data);
                        let _ = editor_data.overlay.insert(pos, data);
                    }
                }
            }
            DrawingMode::Erase => {
                editor_data.overlay.clear();
                editor_data.erase_overlay.clear();
                if state.stamp.is_empty() {
                    let _ = editor_data.erase_overlay.insert(Vector2::new(0, 0));
                } else {
                    editor_data.erase_overlay.extend(state.stamp.keys());
                }
            }
            DrawingMode::Pick => {
                if self.mouse_mode == MouseMode::None {
                    editor_data.overlay.clear();
                    editor_data.erase_overlay.clear();
                }
            }
            _ => {
                editor_data.overlay.clear();
                editor_data.erase_overlay.clear();
            }
        }
    }
}

fn update_select(
    tile_map: &TileMap,
    selecting: &FxHashSet<Vector2<i32>>,
    state: &mut TileDrawStateGuardMut<'_>,
    start: Vector2<i32>,
    end: Vector2<i32>,
) {
    let Some(mut editor_data) = tile_map.editor_data.as_ref().map(|d| d.lock()) else {
        return;
    };
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
    editor_data.selected.clone_from(sel);
    drop(editor_data);
    state.update_stamp(tile_map.tile_set().cloned(), |p| {
        tile_map.tiles().get(&p).copied()
    });
}

fn draw(
    editor_data: &mut TileMapEditorData,
    tiles: &Tiles,
    tool: DrawingMode,
    state: &TileDrawStateGuard<'_>,
    start: Vector2<i32>,
    end: Vector2<i32>,
) {
    let update = &mut editor_data.update;
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
        let scene = &mut engine.scenes[game_scene.scene];
        let mods = engine.user_interfaces.first().keyboard_modifiers();
        let state = self.state.lock();
        self.current_tool = state.drawing_mode;
        let grid_coord = self.pick_grid(scene, game_scene, mouse_position, frame_size);
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
            return;
        };
        let Some(mut editor_data_lock) = tile_map.editor_data.as_ref().map(|d| d.lock()) else {
            return;
        };
        let editor_data = editor_data_lock.deref_mut();
        editor_data.overlay_offset = grid_coord;
        self.click_grid_position = grid_coord;
        self.current_grid_position = grid_coord;
        if let Some(grid_coord) = grid_coord {
            match state.drawing_mode {
                DrawingMode::Pick => {
                    if mods.alt {
                        self.mouse_mode = MouseMode::Dragging;
                        let mut tile_set = state.tile_set.as_ref().map(TileSetRef::new);
                        let tile_set = tile_set.as_mut().map(|t| t.as_loaded());
                        let overlay = &mut editor_data.overlay;
                        overlay.clear();
                        editor_data.erased_area.clear();
                        for pos in editor_data.selected.iter() {
                            let Some(&handle) = tile_map.tiles.get(pos) else {
                                continue;
                            };
                            editor_data.erased_area.insert(*pos);
                            let render_data = tile_set
                                .as_ref()
                                .and_then(|t| t.get_tile_render_data(handle.into()))
                                .unwrap_or_else(TileRenderData::missing_data);
                            let _ = overlay.insert(*pos - grid_coord, render_data);
                        }
                    } else {
                        self.mouse_mode = MouseMode::Drawing;
                        if !mods.shift {
                            self.selecting.clear();
                        }
                        drop(editor_data_lock);
                        let mut state = state.into_mut("TileMap start select");
                        state.set_node(self.tile_map);
                        update_select(
                            tile_map,
                            &self.selecting,
                            &mut state,
                            grid_coord,
                            grid_coord,
                        );
                    }
                }
                mode => {
                    self.mouse_mode = MouseMode::Drawing;
                    draw(
                        editor_data,
                        &tile_map.tiles,
                        mode,
                        &state,
                        grid_coord,
                        grid_coord,
                    );
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
        let scene = &mut engine.scenes[game_scene.scene];
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
            return;
        };
        let start = self.click_grid_position;
        let end = self.current_grid_position;
        self.click_grid_position = None;
        self.current_grid_position = None;

        let tile_map_handle = self.tile_map;
        let mut editor_data = self.editor_data.lock();
        let editor_data = editor_data.deref_mut();
        match self.mouse_mode {
            MouseMode::None => (),
            MouseMode::Dragging => {
                if let (Some(start), Some(end)) = (start, end) {
                    let offset = end - start;
                    if offset != Vector2::new(0, 0) {
                        let tiles = editor_data
                            .overlay
                            .keys()
                            .copied()
                            .map(|p| p + start)
                            .collect::<Vec<_>>();
                        editor_data.selected.clear();
                        editor_data
                            .selected
                            .extend(tiles.iter().map(|p| p + offset));
                        self.sender.do_command(MoveMapTileCommand::new(
                            tile_map_handle,
                            tiles,
                            offset,
                        ));
                    }
                }
                editor_data.overlay.clear();
                editor_data.erased_area.clear();
            }
            MouseMode::Drawing => {
                let state = self.state.lock();
                if let DrawingMode::Pick = self.current_tool {
                    self.selecting.clone_from(&editor_data.selected);
                } else if let Some(tile_set) = state.tile_set.as_ref().or(tile_map.tile_set()) {
                    let update = editor_data
                        .update
                        .build_tiles_update(&TileSetRef::new(tile_set).as_loaded());
                    self.sender.do_command(SetMapTilesCommand {
                        tile_map: tile_map_handle,
                        tiles: update,
                    });
                    editor_data.update.clear();
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

        let mut editor_data = self.editor_data.lock();
        editor_data.cursor_position = grid_coord;
        editor_data.overlay_offset = grid_coord;
        if editor_data.cursor_material.is_none() {
            editor_data.cursor_material = Some(self.cursor_material.clone());
        }
        if editor_data.erase_material.is_none() {
            editor_data.erase_material = Some(self.erase_material.clone());
        }
        if editor_data.select_material.is_none() {
            editor_data.select_material = Some(self.select_material.clone());
        }

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

        let state = self.state.lock();

        match self.mouse_mode {
            MouseMode::None => (),
            MouseMode::Dragging => (),
            MouseMode::Drawing => {
                if let DrawingMode::Pick = self.current_tool {
                    drop(editor_data);
                    update_select(
                        tile_map,
                        &self.selecting,
                        &mut state.into_mut("TileMap select"),
                        start,
                        end,
                    );
                } else {
                    draw(
                        &mut editor_data,
                        &tile_map.tiles,
                        self.current_tool,
                        &state,
                        start,
                        end,
                    );
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
        let mut editor_data = self.editor_data.lock();
        editor_data.cursor_position = None;
        editor_data.overlay_offset = None;
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
            let state = self.state.lock();
            match *code {
                PICK_KEY => {
                    if state.drawing_mode != DrawingMode::Pick {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Pick;
                    }
                    return true;
                }
                ERASE_KEY => {
                    if state.drawing_mode != DrawingMode::Erase {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Erase;
                    }
                    return true;
                }
                RECT_KEY => {
                    if state.drawing_mode != DrawingMode::RectFill {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::RectFill;
                    }
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
            let state = self.state.lock();
            match *code {
                PICK_KEY => {
                    if state.drawing_mode == DrawingMode::Pick {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                ERASE_KEY => {
                    if state.drawing_mode == DrawingMode::Erase {
                        state.into_mut("Hotkey").drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                RECT_KEY => {
                    println!("{:?}", state.drawing_mode);
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
