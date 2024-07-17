mod commands;
pub mod palette;
pub mod panel;
pub mod tile_set_import;
pub mod tileset;

use crate::{
    command::SetPropertyCommand,
    fyrox::{
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
            math::{plane::Plane, Matrix4Ext},
            parking_lot::Mutex,
            pool::Handle,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::{
            button::ButtonBuilder, message::UiMessage, utils::make_simple_tooltip,
            widget::WidgetBuilder, BuildContext, Thickness, UiNode, UserInterface,
        },
        scene::{
            debug::Line,
            node::Node,
            tilemap::{brush::TileMapBrush, TileMap, Tiles},
            Scene,
        },
    },
    interaction::{make_interaction_mode_button, InteractionMode},
    message::MessageSender,
    plugin::EditorPlugin,
    plugins::tilemap::{palette::PaletteMessage, panel::TileMapPanel, tileset::TileSetEditor},
    scene::{commands::GameSceneContext, controller::SceneController, GameScene, Selection},
    settings::Settings,
    Editor, Message,
};
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

struct InteractionContext {
    previous_tiles: Tiles,
}

#[derive(TypeUuidProvider)]
#[type_uuid(id = "33fa8ef9-a29c-45d4-a493-79571edd870a")]
pub struct TileMapInteractionMode {
    #[allow(dead_code)]
    tile_map: Handle<Node>,
    brush: Arc<Mutex<TileMapBrush>>,
    brush_position: Vector2<i32>,
    interaction_context: Option<InteractionContext>,
    sender: MessageSender,
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

    fn draw_with_current_brush(
        &mut self,
        scene: &mut Scene,
        game_scene: &GameScene,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
        ui: &UserInterface,
    ) {
        let modifiers = ui.keyboard_modifiers();

        if let Some(grid_coord) = self.pick_grid(scene, game_scene, mouse_position, frame_size) {
            self.brush_position = grid_coord;

            let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(self.tile_map) else {
                return;
            };

            if self.interaction_context.is_some() {
                let brush = self.brush.lock();

                if modifiers.shift {
                    tile_map.erase(grid_coord, &brush);
                } else {
                    tile_map.draw(grid_coord, &brush)
                }
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

        let Some(tile_map) = scene.graph.try_get_of_type::<TileMap>(self.tile_map) else {
            return;
        };

        self.interaction_context = Some(InteractionContext {
            previous_tiles: tile_map.tiles().clone(),
        });

        self.draw_with_current_brush(
            scene,
            game_scene,
            mouse_position,
            frame_size,
            engine.user_interfaces.first(),
        );
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        controller: &mut dyn SceneController,
        engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        let Some(game_scene) = controller.downcast_mut::<GameScene>() else {
            return;
        };

        let scene = &mut engine.scenes[game_scene.scene];

        let tile_map_handle = self.tile_map;
        let Some(tile_map) = scene.graph.try_get_mut_of_type::<TileMap>(tile_map_handle) else {
            return;
        };

        if let Some(interaction_context) = self.interaction_context.take() {
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

        self.draw_with_current_brush(
            scene,
            game_scene,
            mouse_position,
            frame_size,
            engine.user_interfaces.first(),
        );
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
        self.brush.lock().draw_outline(
            &mut scene.drawing_context,
            self.brush_position,
            &transform,
            Color::RED,
        );
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
}

#[derive(Default)]
pub struct TileMapEditorPlugin {
    tile_set_editor: Option<TileSetEditor>,
    brush: Arc<Mutex<TileMapBrush>>,
    panel: Option<TileMapPanel>,
    tile_map: Handle<Node>,
}

impl EditorPlugin for TileMapEditorPlugin {
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

            let tile_map = editor
                .scenes
                .current_scene_entry_mut()
                .and_then(|entry| entry.controller.downcast_mut::<GameScene>())
                .and_then(|scene| {
                    editor.engine.scenes[scene.scene]
                        .graph
                        .try_get_of_type::<TileMap>(self.tile_map)
                });

            self.panel = panel.handle_ui_message(
                message,
                ui,
                &editor.engine.resource_manager,
                self.tile_map,
                tile_map,
                &editor.message_sender,
            );
        }
    }

    fn on_update(&mut self, _editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.update();
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
