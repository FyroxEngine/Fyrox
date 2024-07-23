#![allow(clippy::collapsible_match)] // STFU

mod commands;
pub mod palette;
pub mod panel;
mod preview;
pub mod tile_set_import;
pub mod tileset;

use crate::{
    command::SetPropertyCommand,
    fyrox::{
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
            math::{plane::Plane, Matrix4Ext, Rect},
            parking_lot::Mutex,
            pool::Handle,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{
            button::ButtonBuilder, key::HotKey, message::KeyCode, message::UiMessage,
            utils::make_simple_tooltip, widget::WidgetBuilder, BuildContext, Thickness, UiNode,
        },
        scene::{
            debug::Line,
            node::Node,
            tilemap::{
                brush::{BrushTile, TileMapBrush},
                tileset::TileSet,
                TileMap, Tiles,
            },
            Scene,
        },
    },
    interaction::{make_interaction_mode_button, InteractionMode},
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::tilemap::{
        palette::PaletteMessage, panel::TileMapPanel, preview::TileSetPreview,
        tileset::TileSetEditor,
    },
    scene::{commands::GameSceneContext, controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
use fyrox::core::algebra::Matrix4;
use std::sync::Arc;

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

pub enum DrawingMode {
    Draw,
    Erase,
    FloodFill,
    Pick {
        click_grid_position: Option<Vector2<i32>>,
    },
    RectFill {
        click_grid_position: Option<Vector2<i32>>,
    },
}

struct InteractionContext {
    previous_tiles: Tiles,
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "33fa8ef9-a29c-45d4-a493-79571edd870a")]
pub struct TileMapInteractionMode {
    tile_map: Handle<Node>,
    brush: Arc<Mutex<TileMapBrush>>,
    brush_position: Vector2<i32>,
    interaction_context: Option<InteractionContext>,
    sender: MessageSender,
    drawing_mode: DrawingMode,
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
        let inv_global_transform = global_transform.try_inverse().unwrap_or_default();

        let camera = scene.graph[game_scene.camera_controller.camera].as_camera();
        let ray = camera.make_ray(mouse_position, frame_size);

        let plane =
            Plane::from_normal_and_point(&global_transform.look(), &global_transform.position())
                .unwrap_or_default();

        ray.plane_intersection_point(&plane).map(|intersection| {
            let local_intersection = inv_global_transform.transform_point(&intersection.into());
            Vector2::new(local_intersection.x as i32, local_intersection.y as i32)
        })
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

        let brush = self.brush.lock();

        if let Some(grid_coord) = self.pick_grid(scene, game_scene, mouse_position, frame_size) {
            let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
                return;
            };

            self.interaction_context = Some(InteractionContext {
                previous_tiles: tile_map.tiles().clone(),
            });

            self.brush_position = grid_coord;

            match self.drawing_mode {
                DrawingMode::Draw => tile_map.draw(grid_coord, &brush),
                DrawingMode::Erase => {
                    tile_map.erase(grid_coord, &brush);
                }
                DrawingMode::FloodFill => {
                    tile_map.flood_fill(grid_coord, &brush);
                }
                DrawingMode::RectFill {
                    ref mut click_grid_position,
                }
                | DrawingMode::Pick {
                    ref mut click_grid_position,
                } => {
                    *click_grid_position = Some(grid_coord);
                }
            }
        }
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

        if let Some(interaction_context) = self.interaction_context.take() {
            if let Some(grid_coord) = grid_coord {
                let mut brush = self.brush.lock();
                match self.drawing_mode {
                    DrawingMode::Pick {
                        click_grid_position,
                    } => {
                        if let Some(click_grid_position) = click_grid_position {
                            brush.tiles.clear();
                            let selected_rect = Rect::from_points(grid_coord, click_grid_position);
                            for y in selected_rect.position.y
                                ..(selected_rect.position.y + selected_rect.size.y)
                            {
                                for x in selected_rect.position.x
                                    ..(selected_rect.position.x + selected_rect.size.x)
                                {
                                    let position = Vector2::new(x, y);
                                    if let Some(tile) = tile_map.tiles().get(&position) {
                                        brush.tiles.push(BrushTile {
                                            definition_handle: tile.definition_handle,
                                            local_position: position - selected_rect.position,
                                            id: Uuid::new_v4(),
                                        })
                                    }
                                }
                            }
                        }
                    }
                    DrawingMode::RectFill {
                        click_grid_position,
                    } => {
                        if let Some(click_grid_position) = click_grid_position {
                            tile_map.rect_fill(
                                Rect::from_points(grid_coord, click_grid_position),
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

        let brush = self.brush.lock();

        if let Some(grid_coord) = self.pick_grid(scene, game_scene, mouse_position, frame_size) {
            let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
                return;
            };

            self.brush_position = grid_coord;

            if self.interaction_context.is_some() {
                match self.drawing_mode {
                    DrawingMode::Draw => tile_map.draw(grid_coord, &brush),
                    DrawingMode::Erase => {
                        tile_map.erase(grid_coord, &brush);
                    }
                    _ => {
                        // Do nothing
                    }
                }
            }
        }
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

        let Some(tile_map) = scene.graph.try_get(self.tile_map) else {
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
                            &self.brush_position.cast::<f32>().to_homogeneous(),
                        ),
                    Color::RED,
                );
            }
            DrawingMode::Pick {
                click_grid_position,
            }
            | DrawingMode::RectFill {
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
    }

    fn deactivate(&mut self, _controller: &dyn SceneController, _engine: &mut Engine) {
        // TODO
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
        false
    }
}

#[derive(Default)]
pub struct TileMapEditorPlugin {
    tile_set_editor: Option<TileSetEditor>,
    brush: Arc<Mutex<TileMapBrush>>,
    panel: Option<TileMapPanel>,
    tile_map: Handle<Node>,
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

        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.sync_to_model(ui);
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
                editor.inspector.property_editors.clone(),
                editor.engine.serialization_context.clone(),
            );
        }

        if let Some(panel) = self.panel.take() {
            if let Some(PaletteMessage::ActiveBrush(brush)) = message.data() {
                if message.destination() == panel.palette {
                    *self.brush.lock() = brush.clone();
                }
            }

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

        if let Some(panel) = self.panel.as_mut() {
            panel.update(
                editor.engine.user_interfaces.first(),
                editor.scenes.current_scene_entry_ref(),
            );
        }
    }

    fn on_message(&mut self, message: &Message, editor: &mut Editor) {
        let ui = editor.engine.user_interfaces.first_mut();

        if let Message::OpenTileSetEditor(tile_set) = message {
            let tile_set_editor = TileSetEditor::new(tile_set.clone(), &mut ui.build_ctx());
            self.tile_set_editor = Some(tile_set_editor);
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

            if let Some(panel) = self.panel.take() {
                panel.destroy(ui);
            }

            for node_handle in selection.nodes().iter() {
                if let Some(tile_map) = scene.graph.try_get(*node_handle) {
                    let Some(tile_map) = tile_map.component_ref::<TileMap>() else {
                        continue;
                    };

                    self.tile_map = *node_handle;

                    entry.interaction_modes.add(TileMapInteractionMode {
                        tile_map: *node_handle,
                        brush: self.brush.clone(),
                        brush_position: Default::default(),
                        interaction_context: None,
                        sender: editor.message_sender.clone(),
                        drawing_mode: DrawingMode::Draw,
                    });

                    self.panel = Some(TileMapPanel::new(
                        &mut ui.build_ctx(),
                        editor.scene_viewer.frame(),
                        tile_map,
                    ));

                    break;
                }
            }
        }
    }
}
