use crate::{scene::EditorScene, settings::Settings, GameEngine};
use fyrox::scene::camera::Projection;
use fyrox::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
    },
    gui::message::{KeyCode, UiMessage},
    scene::{graph::Graph, node::Node},
};
use std::any::Any;

pub mod gizmo;
pub mod move_mode;
pub mod navmesh;
pub mod plane;
pub mod rotate_mode;
pub mod scale_mode;
pub mod select_mode;
pub mod terrain;

pub trait BaseInteractionMode {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> BaseInteractionMode for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait InteractionMode: BaseInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn update(
        &mut self,
        _editor_scene: &mut EditorScene,
        _camera: Handle<Node>,
        _engine: &mut GameEngine,
    ) {
    }

    fn activate(&mut self, _editor_scene: &EditorScene, _engine: &mut GameEngine) {}

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine);

    fn on_key_down(
        &mut self,
        _key: KeyCode,
        _editor_scene: &mut EditorScene,
        _engine: &mut GameEngine,
    ) {
    }

    fn on_key_up(
        &mut self,
        _key: KeyCode,
        _editor_scene: &mut EditorScene,
        _engine: &mut GameEngine,
    ) {
    }

    fn handle_ui_message(
        &mut self,
        _message: &UiMessage,
        _editor_scene: &mut EditorScene,
        _engine: &mut GameEngine,
    ) {
    }

    fn on_drop(&mut self, _engine: &mut GameEngine) {}
}

pub fn calculate_gizmo_distance_scaling(
    graph: &Graph,
    camera: Handle<Node>,
    gizmo_origin: Handle<Node>,
) -> Vector3<f32> {
    let s = match graph[camera].as_camera().projection() {
        Projection::Perspective(proj) => {
            distance_scale_factor(proj.fov)
                * graph[gizmo_origin]
                    .global_position()
                    .metric_distance(&graph[camera].global_position())
        }
        Projection::Orthographic(ortho) => 0.4 * ortho.vertical_size,
    };

    Vector3::new(s, s, s)
}

fn distance_scale_factor(fov: f32) -> f32 {
    fov.tan() * 0.1
}

/// Helper enum to be able to access interaction modes in array directly.
#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
#[repr(usize)]
pub enum InteractionModeKind {
    Select = 0,
    Move = 1,
    Scale = 2,
    Rotate = 3,
    Navmesh = 4,
    Terrain = 5,
}
