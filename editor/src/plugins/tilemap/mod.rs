pub mod brush;
pub mod palette;
pub mod panel;
pub mod tile_set_import;
pub mod tileset;

use crate::plugins::tilemap::palette::PaletteMessage;
use crate::{
    fyrox::{
        core::{
            algebra::{Vector2, Vector3},
            color::Color,
            math::plane::Plane,
            parking_lot::Mutex,
            pool::Handle,
            type_traits::prelude::*,
            Uuid,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraphNode},
        gui::{
            button::ButtonBuilder, message::UiMessage, utils::make_simple_tooltip,
            widget::WidgetBuilder, BuildContext, Thickness, UiNode,
        },
        scene::{debug::Line, node::Node, tilemap::TileMap},
    },
    interaction::{make_interaction_mode_button, InteractionMode},
    plugin::EditorPlugin,
    plugins::tilemap::{brush::TileMapBrush, panel::TileMapPanel, tileset::TileSetEditor},
    scene::{controller::SceneController, GameScene, Selection},
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

#[derive(TypeUuidProvider)]
#[type_uuid(id = "33fa8ef9-a29c-45d4-a493-79571edd870a")]
pub struct TileMapInteractionMode {
    #[allow(dead_code)]
    tile_map: Handle<Node>,
    brush: Arc<Mutex<TileMapBrush>>,
    brush_position: Vector2<i32>,
}

impl InteractionMode for TileMapInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        // TODO
    }

    fn on_left_mouse_button_up(
        &mut self,
        _editor_selection: &Selection,
        _controller: &mut dyn SceneController,
        _engine: &mut Engine,
        _mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        // TODO
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

        let camera = scene.graph[game_scene.camera_controller.camera].as_camera();
        let ray = camera.make_ray(mouse_position, frame_size);

        // TODO: This does not take global transform of the tile map into account!
        let plane = Plane::from_normal_and_point(&Vector3::new(0.0, 0.0, 1.0), &Default::default())
            .unwrap_or_default();

        if let Some(intersection) = ray.plane_intersection_point(&plane) {
            let grid_coord = Vector2::new(intersection.x as i32, intersection.y as i32);
            self.brush_position = grid_coord;
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
                    .transform_point(&Vector3::new(begin.x as f32, begin.y as f32, 0.0).into())
                    .coords,
                end: transform
                    .transform_point(&Vector3::new(end.x as f32, end.y as f32, 0.0).into())
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
}

impl EditorPlugin for TileMapEditorPlugin {
    fn on_sync_to_model(&mut self, editor: &mut Editor) {
        if let Some(tile_set_editor) = self.tile_set_editor.as_mut() {
            tile_set_editor.sync_to_model(editor.engine.user_interfaces.first_mut());
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
            );
        }

        if let Some(panel) = self.panel.take() {
            if let Some(PaletteMessage::Brush(brush)) = message.data() {
                if message.destination() == panel.palette {
                    *self.brush.lock() = brush.clone();
                }
            }

            self.panel = panel.handle_ui_message(message, ui);
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

                    entry.interaction_modes.add(TileMapInteractionMode {
                        tile_map: *node_handle,
                        brush: self.brush.clone(),
                        brush_position: Default::default(),
                    });

                    self.panel = Some(TileMapPanel::new(
                        &mut ui.build_ctx(),
                        editor.scene_viewer.frame(),
                        tile_map.tile_set().cloned(),
                    ));

                    break;
                }
            }
        }
    }
}
