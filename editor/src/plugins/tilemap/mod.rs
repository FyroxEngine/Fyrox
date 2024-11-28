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

#![allow(clippy::collapsible_match)] // STFU

mod colliders_tab;
mod commands;
mod handle_editor;
mod misc;
pub mod palette;
pub mod panel;
mod panel_preview;
mod preview;
mod properties_tab;
mod tile_bounds_editor;
mod tile_inspector;
mod tile_prop_editor;
pub mod tile_set_import;
pub mod tileset;
use colliders_tab::*;
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use handle_editor::*;
use palette::PaletteWidget;
use panel::TileMapPanel;
use panel_preview::*;
use properties_tab::*;
pub use tile_bounds_editor::*;
use tile_inspector::*;
pub use tile_prop_editor::*;

use crate::plugins::inspector::InspectorPlugin;
use crate::{
    command::SetPropertyCommand,
    fyrox::{
        core::{
            algebra::{Matrix4, Vector2, Vector3},
            color::Color,
            math::{plane::Plane, Matrix4Ext, Rect},
            parking_lot::Mutex,
            pool::Handle,
            reflect::prelude::*,
            type_traits::prelude::*,
            visitor::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{
            button::ButtonBuilder,
            inspector::editors::inherit::InheritablePropertyEditorDefinition,
            key::HotKey,
            message::{KeyCode, MessageDirection, UiMessage},
            utils::make_simple_tooltip,
            widget::{WidgetBuilder, WidgetMessage},
            BuildContext, Thickness, UiNode,
        },
        scene::{
            debug::Line,
            node::Node,
            tilemap::{
                brush::TileMapBrush,
                tileset::{TileSet, TileSetResource},
                RandomTileSource, RepeatTileSource, Stamp, Tile, TileMap, TileResource, Tiles,
                TilesUpdate,
            },
            Scene,
        },
    },
    interaction::{make_interaction_mode_button, InteractionMode},
    load_image,
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::tilemap::{
        misc::TilesPropertyEditorDefinition, palette::PaletteMessage, preview::TileSetPreview,
        tileset::TileSetEditor,
    },
    scene::{commands::GameSceneContext, controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
use fyrox::{
    asset::untyped::UntypedResource,
    core::{log::Log, parking_lot::MutexGuard, ImmutableString},
    fxhash::FxHashSet,
    gui::{
        border::BorderBuilder, brush::Brush, decorator::DecoratorBuilder, image::ImageBuilder,
        UserInterface,
    },
    scene::tilemap::{
        tileset::{TileCollider, TileMaterialBounds},
        TileDefinitionHandle, TilePaletteStage, TransTilesUpdate,
    },
};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

lazy_static! {
    static ref BRUSH_IMAGE: Option<UntypedResource> = load_image!("../../../resources/brush.png");
    static ref ERASER_IMAGE: Option<UntypedResource> = load_image!("../../../resources/eraser.png");
    static ref FILL_IMAGE: Option<UntypedResource> = load_image!("../../../resources/fill.png");
    static ref PICK_IMAGE: Option<UntypedResource> = load_image!("../../../resources/pipette.png");
    static ref RECT_FILL_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/rect_fill.png");
    static ref NINE_SLICE_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/nine_slice.png");
    static ref LINE_IMAGE: Option<UntypedResource> = load_image!("../../../resources/line.png");
    static ref TURN_LEFT_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/turn_left.png");
    static ref TURN_RIGHT_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/turn_right.png");
    static ref FLIP_X_IMAGE: Option<UntypedResource> = load_image!("../../../resources/flip_x.png");
    static ref FLIP_Y_IMAGE: Option<UntypedResource> = load_image!("../../../resources/flip_y.png");
    static ref RANDOM_IMAGE: Option<UntypedResource> = load_image!("../../../resources/die.png");
    static ref PALETTE_IMAGE: Option<UntypedResource> =
        load_image!("../../../resources/palette.png");
}

fn make_button(
    title: &str,
    tooltip: &str,
    enabled: bool,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_enabled(enabled)
            .with_width(100.0)
            .with_height(24.0)
            .with_margin(Thickness::uniform(1.0))
            .with_tooltip(make_simple_tooltip(ctx, tooltip)),
    )
    .with_text(title)
    .build(ctx)
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Visit, Reflect)]
pub enum DrawingMode {
    #[default]
    Draw,
    Erase,
    FloodFill,
    Pick,
    RectFill,
    NineSlice,
    Line,
    Material,
    Property,
    Color,
    Collider,
}

struct InteractionContext {
    changed_tiles: TilesUpdate,
}

#[derive(Clone, Default, Debug, PartialEq)]
enum MouseMode {
    #[default]
    None,
    Dragging {
        initial_position: Vector2<f32>,
        offset: Vector2<i32>,
    },
    Drawing {
        start_tile: Vector2<i32>,
        end_tile: Vector2<i32>,
    },
}

#[derive(Debug, PartialEq, Clone)]
struct OpenTilePanelMessage {
    resource: TileResource,
    center: Option<TileDefinitionHandle>,
}

impl OpenTilePanelMessage {
    fn message(resource: TileResource, center: Option<TileDefinitionHandle>) -> UiMessage {
        UiMessage::with_data(Self { resource, center })
    }
}

fn make_drawing_mode_button(
    ctx: &mut BuildContext,
    width: f32,
    height: f32,
    image: Option<UntypedResource>,
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
                WidgetBuilder::new().with_foreground(ctx.style.get_or_default(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius(4.0)
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
                .with_margin(Thickness::uniform(2.0))
                .with_width(width)
                .with_height(height),
        )
        .with_opt_texture(image)
        .build(ctx),
    )
    .build(ctx)
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "33fa8ef9-a29c-45d4-a493-79571edd870a")]
pub struct TileMapInteractionMode {
    tile_map: Handle<Node>,
    state: TileDrawStateRef,
    panel: Handle<UiNode>,
    brush_position: Option<Vector2<i32>>,
    click_grid_position: Option<Vector2<i32>>,
    sender: MessageSender,
    update: TransTilesUpdate,
    mode: MouseMode,
}

impl TileMapInteractionMode {
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
    fn sync_to_state(&mut self, ui: &mut UserInterface) {
        // TODO
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
        /* TODO
                let brush = self.brush.lock();

                if let Some(grid_coord) = self.pick_grid(scene, game_scene, mouse_position, frame_size) {
                    let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
                        return;
                    };

                    self.interaction_context = Some(InteractionContext {
                        previous_tiles: tile_map.tiles().clone(),
                    });

                    self.brush_position = grid_coord;
                    self.click_grid_position = Some(grid_coord);

                    match self.drawing_mode {
                        DrawingMode::Draw => tile_map.tiles.draw(grid_coord, &brush),
                        DrawingMode::Erase => {
                            tile_map.tiles.erase(grid_coord, &brush);
                        }
                        DrawingMode::FloodFill => {
                            tile_map.tiles.flood_fill(grid_coord, &brush);
                        }
                        _ => (),
                    }
                }
        */
    }

    fn on_left_mouse_button_up(
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

        let grid_coord = self.pick_grid(scene, game_scene, mouse_position, frame_size);

        let tile_map_handle = self.tile_map;
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(tile_map_handle) else {
            return;
        };
        /* TODO
                if let Some(interaction_context) = self.interaction_context.take() {
                    if let Some(grid_coord) = grid_coord {
                        let mut brush = self.brush.lock();
                        match self.drawing_mode {
                            DrawingMode::Pick => {
                                if let Some(click_grid_position) = self.click_grid_position {
                                    brush.clear();
                                    let selected_rect = Rect::from_points(grid_coord, click_grid_position);
                                    for y in selected_rect.position.y
                                        ..(selected_rect.position.y + selected_rect.size.y)
                                    {
                                        for x in selected_rect.position.x
                                            ..(selected_rect.position.x + selected_rect.size.x)
                                        {
                                            let position = Vector2::new(x, y);
                                            if let Some(tile) = tile_map.tiles().get(&position) {
                                                let pos = position - selected_rect.position;
                                                brush.insert(&pos, tile.definition_handle);
                                            }
                                        }
                                    }
                                }
                            }
                            DrawingMode::RectFill => {
                                if let Some(click_grid_position) = self.click_grid_position {
                                    interaction_context.changed_tiles.clear();
                                    let source = if self.random_mode {
                                        interaction_context.changed_tiles.rect_fill(
                                            Rect::from_points(grid_coord, click_grid_position),
                                            &RandomTileSource(&*brush),
                                        );
                                    } else {
                                        interaction_context.changed_tiles.rect_fill(
                                            Rect::from_points(grid_coord, click_grid_position),
                                            &RepeatTileSource::new(&*brush),
                                        );
                                    };
                                }
                            }
                            DrawingMode::NineSlice => {
                                if let Some(click_grid_position) = self.click_grid_position {
                                    tile_map.tiles.nine_slice(
                                        Rect::from_points(grid_coord, click_grid_position),
                                        &brush,
                                    )
                                }
                            }
                            DrawingMode::Line => {
                                if let Some(click_grid_position) = self.click_grid_position {
                                    tile_map.tiles.draw_line_with_brush(
                                        self.brush_position,
                                        click_grid_position,
                                        &brush,
                                    );
                                }
                            }
                            _ => (),
                        }
                    }

                    if !matches!(self.drawing_mode, DrawingMode::Pick { .. }) {
                        let new_tiles = tile_map.tiles().clone();
                        tile_map.set_tiles(interaction_context.previous_tiles);
                        self.sender.do_command(SetPropertyCommand::new(
                            "tiles".to_string(),
                            Box::new(new_tiles),
                            move |ctx| {
                                ctx.get_mut::<GameSceneContext>()
                                    .scene
                                    .graph
                                    .node_mut(tile_map_handle)
                            },
                        ));
                    }
                }
        */
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
        /*
                let brush = self.brush.lock();

                if let Some(grid_coord) = self.pick_grid(scene, game_scene, mouse_position, frame_size) {
                    let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
                        return;
                    };

                    self.brush_position = grid_coord;

                    if self.interaction_context.is_some() {
                        match self.drawing_mode {
                            DrawingMode::Draw => tile_map.tiles.draw(grid_coord, &brush),
                            DrawingMode::Erase => {
                                tile_map.tiles.erase(grid_coord, &brush);
                            }
                            _ => {
                                // Do nothing
                            }
                        }
                    }
                }
        */
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

        let mut draw_line = |begin: Vector2<i32>, end: Vector2<i32>, color: Color| {
            scene.drawing_context.add_line(Line {
                begin: transform
                    .transform_point(&Vector3::new(begin.x as f32, begin.y as f32, -0.01).into())
                    .coords,
                end: transform
                    .transform_point(&Vector3::new(end.x as f32, end.y as f32, -0.01).into())
                    .coords,
                color,
            });
        };

        let size = 1000i32;
        for y in -size..size {
            draw_line(Vector2::new(-size, y), Vector2::new(size, y), Color::WHITE);
        }
        for x in -size..size {
            draw_line(Vector2::new(x, -size), Vector2::new(x, size), Color::WHITE);
        }
        /* TODO
                match self.drawing_mode {
                    DrawingMode::Draw | DrawingMode::Erase => {
                        self.brush.lock().draw_outline(
                            &mut scene.drawing_context,
                            self.brush_position,
                            &transform,
                            Color::RED,
                        );
                    }
                    DrawingMode::FloodFill => {
                        scene.drawing_context.draw_rectangle(
                            0.5,
                            0.5,
                            transform
                                * Matrix4::new_translation(
                                    &(self.brush_position.cast::<f32>().to_homogeneous()
                                        + Vector3::new(0.5, 0.5, 0.0)),
                                ),
                            Color::RED,
                        );
                    }
                    DrawingMode::Line {
                        click_grid_position,
                    } => {
                        if self.interaction_context.is_some() {
                            if let Some(click_grid_position) = click_grid_position {
                                for point in [click_grid_position, self.brush_position] {
                                    scene.drawing_context.draw_rectangle(
                                        0.5,
                                        0.5,
                                        transform
                                            * Matrix4::new_translation(
                                                &(point.cast::<f32>().to_homogeneous()
                                                    + Vector3::new(0.5, 0.5, 0.0)),
                                            ),
                                        Color::RED,
                                    );
                                }
                            }
                        }
                    }
                    DrawingMode::Pick {
                        click_grid_position,
                    }
                    | DrawingMode::RectFill {
                        click_grid_position,
                    }
                    | DrawingMode::NineSlice {
                        click_grid_position,
                    } => {
                        if self.interaction_context.is_some() {
                            if let Some(click_grid_position) = click_grid_position {
                                let rect = Rect::from_points(click_grid_position, self.brush_position);
                                let position = rect.position.cast::<f32>();
                                let half_size = rect.size.cast::<f32>().scale(0.5);

                                scene.drawing_context.draw_rectangle(
                                    half_size.x,
                                    half_size.y,
                                    transform
                                        * Matrix4::new_translation(
                                            &(position + half_size).to_homogeneous(),
                                        ),
                                    Color::RED,
                                );
                            }
                        }
                    }
                }

                let brush = self.brush.lock();

                tile_map.overlay_tiles.clear();
                match self.drawing_mode {
                    DrawingMode::Draw => {
                        for tile in brush.tiles.iter() {
                            tile_map.overlay_tiles.insert(Tile {
                                position: self.brush_position + tile.local_position,
                                definition_handle: tile.definition_handle,
                            });
                        }
                    }
                    DrawingMode::Erase => {}
                    DrawingMode::FloodFill => {
                        let tiles = tile_map
                            .tiles
                            .flood_fill_immutable(self.brush_position, &brush);
                        for tile in tiles {
                            tile_map.overlay_tiles.insert(tile);
                        }
                    }
                    DrawingMode::Pick { .. } => {}
                    DrawingMode::RectFill {
                        click_grid_position,
                    } => {
                        if self.interaction_context.is_some() {
                            if let Some(click_grid_position) = click_grid_position {
                                tile_map.overlay_tiles.rect_fill(
                                    Rect::from_points(self.brush_position, click_grid_position),
                                    &brush,
                                );
                            }
                        }
                    }
                    DrawingMode::NineSlice {
                        click_grid_position,
                    } => {
                        if self.interaction_context.is_some() {
                            if let Some(click_grid_position) = click_grid_position {
                                tile_map.overlay_tiles.nine_slice(
                                    Rect::from_points(self.brush_position, click_grid_position),
                                    &brush,
                                );
                            }
                        }
                    }
                    DrawingMode::Line {
                        click_grid_position,
                    } => {
                        if self.interaction_context.is_some() {
                            if let Some(click_grid_position) = click_grid_position {
                                tile_map.overlay_tiles.draw_line_with_brush(
                                    self.brush_position,
                                    click_grid_position,
                                    &brush,
                                );
                            }
                        }
                    }
                }
        */
    }

    fn activate(&mut self, _controller: &dyn SceneController, engine: &mut Engine) {
        if self.panel.is_some() {
            let ui = engine.user_interfaces.first_mut();
            ui.send_message(WidgetMessage::visibility(
                self.panel,
                MessageDirection::ToWidget,
                true,
            ));
        }
    }

    fn deactivate(&mut self, _controller: &dyn SceneController, engine: &mut Engine) {
        if self.panel.is_some() {
            let ui = engine.user_interfaces.first_mut();
            ui.send_message(WidgetMessage::visibility(
                self.panel,
                MessageDirection::ToWidget,
                false,
            ));
        }
    }

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
        /*
                if let HotKey::Some { code, .. } = hotkey {
                    match *code {
                        KeyCode::AltLeft => {
                            self.drawing_mode = DrawingMode::Pick {
                                click_grid_position: None,
                            };
                            return true;
                        }
                        KeyCode::ShiftLeft => {
                            self.drawing_mode = DrawingMode::Erase;
                            return true;
                        }
                        KeyCode::ControlLeft => {
                            self.drawing_mode = DrawingMode::RectFill {
                                click_grid_position: None,
                            };
                            return true;
                        }
                        _ => (),
                    }
                }
        */
        false
    }

    fn on_hot_key_released(
        &mut self,
        hotkey: &HotKey,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _settings: &Settings,
    ) -> bool {
        /*
        if let HotKey::Some { code, .. } = hotkey {
            match *code {
                KeyCode::AltLeft => {
                    if matches!(self.drawing_mode, DrawingMode::Pick { .. }) {
                        self.drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                KeyCode::ShiftLeft => {
                    if matches!(self.drawing_mode, DrawingMode::Erase) {
                        self.drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                KeyCode::ControlLeft => {
                    if matches!(self.drawing_mode, DrawingMode::RectFill { .. }) {
                        self.drawing_mode = DrawingMode::Draw;
                        return true;
                    }
                }
                _ => (),
            }
        }
        */
        false
    }
}

#[derive(Default)]
pub struct TileMapEditorPlugin {
    state: TileDrawStateRef,
    tile_set_editor: Option<TileSetEditor>,
    panel: Option<TileMapPanel>,
    tile_map: Handle<Node>,
}

#[derive(Default, Debug, Clone, Visit)]
pub struct TileDrawState {
    /// True if the state has been changed and the change has not yet caused the UI to update.
    dirty: bool,
    tile_set: Option<TileSetResource>,
    stamp: Stamp,
    drawing_mode: DrawingMode,
    active_prop: Option<Uuid>,
    draw_value: DrawValue,
    random_mode: bool,
    selection: TileDrawSelection,
}

#[derive(Debug, Clone, Visit, Reflect)]
pub enum DrawValue {
    I8(i8),
    I32(i32),
    F32(f32),
    String(ImmutableString),
    Color(Color),
    Material(TileMaterialBounds),
    Collider(TileCollider),
}

impl Default for DrawValue {
    fn default() -> Self {
        Self::I32(0)
    }
}

#[derive(Debug, Default, Clone, Visit)]
pub struct TileDrawStateRef(Arc<Mutex<TileDrawState>>);
pub struct TileDrawStateGuard<'a>(MutexGuard<'a, TileDrawState>);
pub struct TileDrawStateGuardMut<'a>(MutexGuard<'a, TileDrawState>);

impl<'a> Deref for TileDrawStateGuard<'a> {
    type Target = TileDrawState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Deref for TileDrawStateGuardMut<'a> {
    type Target = TileDrawState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for TileDrawStateGuardMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TileDrawStateRef {
    pub fn lock(&self) -> TileDrawStateGuard {
        TileDrawStateGuard(self.0.lock())
    }
    pub fn lock_mut(&self) -> TileDrawStateGuardMut {
        self.lock().into_mut()
    }
    pub fn check_dirty(&self) -> bool {
        let mut state = self.0.lock();
        let dirty = state.dirty;
        state.dirty = false;
        dirty
    }
}

impl<'a> TileDrawStateGuard<'a> {
    pub fn into_mut(self) -> TileDrawStateGuardMut<'a> {
        let mut result = TileDrawStateGuardMut(self.0);
        result.dirty = true;
        result
    }
}

#[derive(Default, Debug, Clone, Visit)]
struct TileDrawSelection {
    pub source: SelectionSource,
    pub page: Vector2<i32>,
    pub positions: FxHashSet<Vector2<i32>>,
    pub tiles: Tiles,
}

impl TileDrawState {
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
    #[inline]
    pub fn set_palette(&mut self, handle: Handle<UiNode>) {
        self.selection.source = SelectionSource::Widget(handle);
    }
    #[inline]
    pub fn set_node(&mut self, handle: Handle<Node>) {
        self.selection.source = SelectionSource::Node(handle);
    }
    #[inline]
    pub fn selection_tiles(&self) -> &Tiles {
        &self.selection.tiles
    }
    #[inline]
    pub fn selection_tiles_mut(&mut self) -> &mut Tiles {
        &mut self.selection.tiles
    }
    #[inline]
    pub fn selection_positions(&self) -> &FxHashSet<Vector2<i32>> {
        &self.selection.positions
    }
    #[inline]
    pub fn selection_positions_mut(&mut self) -> &mut FxHashSet<Vector2<i32>> {
        &mut self.selection.positions
    }
    #[inline]
    pub fn clear_selection(&mut self) {
        self.stamp.clear();
        self.selection.positions.clear();
        self.selection.tiles.clear();
        self.selection.source = SelectionSource::None;
    }
    #[inline]
    pub fn update_stamp(&mut self, tile_set: Option<TileSetResource>) {
        self.tile_set = tile_set;
        self.stamp
            .build(self.selection.tiles.iter().map(|(&x, &y)| (x, y)));
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Visit)]
pub enum SelectionSource {
    #[default]
    None,
    Widget(Handle<UiNode>),
    Node(Handle<Node>),
}

impl TileMapEditorPlugin {
    fn initialize_tile_map_panel(
        &mut self,
        resource: TileResource,
        center: Option<TileDefinitionHandle>,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if let Some(panel) = &mut self.panel {
            panel.to_top(ui);
        } else if let Some(editor) = &self.tile_set_editor {
            let panel = TileMapPanel::new(&mut ui.build_ctx(), self.state.clone(), sender.clone());
            panel.align(editor.window, ui);
            self.panel = Some(panel);
        }
        if let Some(panel) = &mut self.panel {
            panel.set_resource(resource, ui);
            if let Some(focus) = center {
                panel.set_focus(focus, ui);
            }
        }
    }
    fn update_state(&mut self) {
        let state = self.state.lock();
        if match &state.drawing_mode {
            DrawingMode::Pick => false,
            DrawingMode::Color | DrawingMode::Property => self.tile_set_editor.is_none(),
            _ => self.panel.is_none(),
        } {
            state.into_mut().drawing_mode = DrawingMode::Pick;
        }
    }
}

impl EditorPlugin for TileMapEditorPlugin {
    fn on_start(&mut self, editor: &mut Editor) {
        editor
            .asset_browser
            .preview_generators
            .add(TileSet::type_uuid(), TileSetPreview);
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
            panel.sync_to_state(ui);
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

        let scene = &mut editor.engine.scenes[game_scene.scene];

        for node_handle in selection.nodes().iter() {
            if let Some(tile_map_node) = scene.graph.try_get(*node_handle) {
                let Some(tile_map) = tile_map_node.component_ref::<TileMap>() else {
                    continue;
                };

                if let Some(panel) = self.panel.as_mut() {
                    panel.sync_to_model(ui, tile_map);
                }
            }
        }
    }

    fn on_ui_message(&mut self, message: &mut UiMessage, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        if let Some(tile_set_editor) = self.tile_set_editor.take() {
            self.tile_set_editor = tile_set_editor.handle_ui_message(
                message,
                ui,
                &editor.engine.resource_manager,
                &editor.message_sender,
                editor.engine.serialization_context.clone(),
            );
        }

        if let Some(OpenTilePanelMessage { resource, center }) = message.data() {
            self.initialize_tile_map_panel(resource.clone(), *center, ui, &editor.message_sender);
        }

        if let Some(panel) = self.panel.take() {
            let editor_scene_entry = editor.scenes.current_scene_entry_mut();

            let tile_map = editor_scene_entry
                .as_ref()
                .and_then(|entry| entry.controller.downcast_ref::<GameScene>())
                .and_then(|scene| {
                    editor.engine.scenes[scene.scene]
                        .graph
                        .try_get_of_type::<TileMap>(self.tile_map)
                });

            self.panel = panel.handle_ui_message(
                message,
                ui,
                self.tile_map,
                tile_map,
                &editor.message_sender,
                editor_scene_entry,
            );
        }
    }

    fn on_update(&mut self, editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.update();
        }

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
                interaction_mode.sync_to_state(editor.engine.user_interfaces.first_mut());
            }
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        let tile_set: Option<TileResource> = if let Message::OpenTileSetEditor(tile_set) = message {
            Some(TileResource::TileSet(tile_set.clone()))
        } else if let Message::OpenTileMapBrushEditor(brush) = message {
            Some(TileResource::Brush(brush.clone()))
        } else {
            None
        };

        if let Some(tile_set) = tile_set {
            if self.tile_set_editor.is_none() {
                let tile_set_editor = TileSetEditor::new(
                    tile_set.clone(),
                    self.state.clone(),
                    editor.message_sender.clone(),
                    editor.engine.resource_manager.clone(),
                    &mut ui.build_ctx(),
                );
                self.tile_set_editor = Some(tile_set_editor);
            } else if let Some(editor) = &mut self.tile_set_editor {
                editor.set_tile_resource(tile_set.clone(), ui);
            }
            self.initialize_tile_map_panel(tile_set, None, ui, &editor.message_sender);
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

        let scene = &mut editor.engine.scenes[game_scene.scene];

        if let Message::SelectionChanged { .. } = message {
            entry
                .interaction_modes
                .remove_typed::<TileMapInteractionMode>();

            let mut tile_map: Option<&TileMap> = None;
            let mut handle = Handle::NONE;

            for node_handle in selection.nodes().iter() {
                if let Some(node) = scene.graph.try_get(*node_handle) {
                    let Some(t) = node.component_ref::<TileMap>() else {
                        continue;
                    };

                    tile_map = Some(t);
                    handle = *node_handle;

                    self.tile_map = *node_handle;

                    break;
                }
            }
            if let Some(tile_map) = tile_map {
                if let Some(panel) = &mut self.panel {
                    panel.set_tile_map(tile_map, ui);
                    panel.to_top(ui);
                } else {
                    let mut panel = TileMapPanel::new(
                        &mut ui.build_ctx(),
                        self.state.clone(),
                        editor.message_sender.clone(),
                    );
                    panel.align(editor.scene_viewer.frame(), ui);
                    panel.set_tile_map(tile_map, ui);
                    self.panel = Some(panel);
                }
                entry.interaction_modes.add(TileMapInteractionMode {
                    tile_map: handle,
                    panel: self.panel.as_ref().unwrap().window,
                    state: self.state.clone(),
                    click_grid_position: None,
                    brush_position: None,
                    sender: editor.message_sender.clone(),
                    mode: MouseMode::None,
                    update: TransTilesUpdate::default(),
                });
            } else if let Some(panel) = &mut self.panel {
                panel.set_visibility(false, ui);
            }
        }
    }
}
