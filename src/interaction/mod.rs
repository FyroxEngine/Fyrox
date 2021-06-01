use crate::settings::Settings;
use crate::{
    interaction::{
        move_mode::MoveInteractionMode, navmesh::EditNavmeshMode,
        rotate_mode::RotateInteractionMode, scale_mode::ScaleInteractionMode,
        select_mode::SelectInteractionMode, terrain::TerrainInteractionMode,
    },
    scene::EditorScene,
    GameEngine,
};
use rg3d::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
        scope_profile,
    },
    gui::message::KeyCode,
    scene::{graph::Graph, node::Node},
};

pub mod gizmo;
pub mod move_mode;
pub mod navmesh;
pub mod plane;
pub mod rotate_mode;
pub mod scale_mode;
pub mod select_mode;
pub mod terrain;

pub trait InteractionModeTrait {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    );

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
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
        editor_scene: &mut EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    );

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
}

pub fn calculate_gizmo_distance_scaling(
    graph: &Graph,
    camera: Handle<Node>,
    gizmo_origin: Handle<Node>,
) -> Vector3<f32> {
    let distance = distance_scale_factor(graph[camera].as_camera().fov())
        * graph[gizmo_origin]
            .global_position()
            .metric_distance(&graph[camera].global_position());
    Vector3::new(distance, distance, distance)
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

pub enum InteractionMode {
    Select(SelectInteractionMode),
    Move(MoveInteractionMode),
    Scale(ScaleInteractionMode),
    Rotate(RotateInteractionMode),
    Navmesh(EditNavmeshMode),
    Terrain(TerrainInteractionMode),
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            InteractionMode::Select(v) => v.$func($($args),*),
            InteractionMode::Move(v) => v.$func($($args),*),
            InteractionMode::Scale(v) => v.$func($($args),*),
            InteractionMode::Rotate(v) => v.$func($($args),*),
            InteractionMode::Navmesh(v) => v.$func($($args),*),
            InteractionMode::Terrain(v) => v.$func($($args),*),
        }
    }
}

impl InteractionModeTrait for InteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        scope_profile!();

        static_dispatch!(
            self,
            on_left_mouse_button_down,
            editor_scene,
            engine,
            mouse_pos,
            frame_size
        )
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        scope_profile!();

        static_dispatch!(
            self,
            on_left_mouse_button_up,
            editor_scene,
            engine,
            mouse_pos,
            frame_size
        )
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        scope_profile!();

        static_dispatch!(
            self,
            on_mouse_move,
            mouse_offset,
            mouse_position,
            camera,
            editor_scene,
            engine,
            frame_size,
            settings
        )
    }

    fn update(
        &mut self,
        editor_scene: &mut EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        scope_profile!();

        static_dispatch!(self, update, editor_scene, camera, engine)
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        static_dispatch!(self, deactivate, editor_scene, engine)
    }

    fn on_key_down(
        &mut self,
        key: KeyCode,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
    ) {
        scope_profile!();

        static_dispatch!(self, on_key_down, key, editor_scene, engine)
    }

    fn on_key_up(&mut self, key: KeyCode, editor_scene: &mut EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        static_dispatch!(self, on_key_up, key, editor_scene, engine)
    }
}
