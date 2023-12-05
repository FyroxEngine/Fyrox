use crate::{scene::EditorScene, settings::Settings, Engine};
use fyrox::gui::key::HotKey;
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
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        editor_scene: &mut EditorScene,
        engine: &mut Engine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    );

    fn update(
        &mut self,
        #[allow(unused_variables)] editor_scene: &mut EditorScene,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) {
    }

    fn activate(
        &mut self,
        #[allow(unused_variables)] editor_scene: &EditorScene,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut Engine);

    /// Should return `true` if the `key` was handled in any way, otherwise you may mess up
    /// keyboard message routing. Return `false` if the `key` is unhandled.
    fn on_key_down(
        &mut self,
        #[allow(unused_variables)] key: KeyCode,
        #[allow(unused_variables)] editor_scene: &mut EditorScene,
        #[allow(unused_variables)] engine: &mut Engine,
    ) -> bool {
        false
    }

    /// Should return `true` if the `key` was handled in any way, otherwise you may mess up
    /// keyboard message routing. Return `false` if the `key` is unhandled.
    fn on_key_up(
        &mut self,
        #[allow(unused_variables)] key: KeyCode,
        #[allow(unused_variables)] editor_scene: &mut EditorScene,
        #[allow(unused_variables)] engine: &mut Engine,
    ) -> bool {
        false
    }

    fn handle_ui_message(
        &mut self,
        #[allow(unused_variables)] message: &UiMessage,
        #[allow(unused_variables)] editor_scene: &mut EditorScene,
        #[allow(unused_variables)] engine: &mut Engine,
    ) {
    }

    fn on_drop(&mut self, _engine: &mut Engine) {}

    fn on_hot_key(
        &mut self,
        #[allow(unused_variables)] hotkey: &HotKey,
        #[allow(unused_variables)] editor_scene: &mut EditorScene,
        #[allow(unused_variables)] engine: &mut Engine,
        #[allow(unused_variables)] settings: &Settings,
    ) -> bool {
        false
    }
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
#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Eq)]
#[repr(usize)]
pub enum InteractionModeKind {
    Select = 0,
    Move = 1,
    Scale = 2,
    Rotate = 3,
    Navmesh = 4,
    Terrain = 5,
}
