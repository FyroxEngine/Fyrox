use crate::{
    gui::UiNode,
    interaction::{
        move_mode::MoveInteractionMode, navmesh::EditNavmeshMode,
        rotate_mode::RotateInteractionMode, scale_mode::ScaleInteractionMode,
        terrain::TerrainInteractionMode,
    },
    scene::{
        commands::{ChangeSelectionCommand, SceneCommand},
        EditorScene, GraphSelection, Selection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{Vector2, Vector3},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        scope_profile,
    },
    gui::message::{KeyCode, MessageDirection, WidgetMessage},
    scene::{graph::Graph, node::Node},
};
use std::sync::mpsc::Sender;

pub mod move_mode;
pub mod navmesh;
pub mod rotate_mode;
pub mod scale_mode;
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

pub struct SelectInteractionMode {
    preview: Handle<UiNode>,
    selection_frame: Handle<UiNode>,
    message_sender: Sender<Message>,
    stack: Vec<Handle<Node>>,
    click_pos: Vector2<f32>,
}

impl SelectInteractionMode {
    pub fn new(
        preview: Handle<UiNode>,
        selection_frame: Handle<UiNode>,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            preview,
            selection_frame,
            message_sender,
            stack: Vec::new(),
            click_pos: Vector2::default(),
        }
    }
}

impl InteractionModeTrait for SelectInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        _editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        _frame_size: Vector2<f32>,
    ) {
        self.click_pos = mouse_pos;
        let ui = &mut engine.user_interface;
        ui.send_message(WidgetMessage::visibility(
            self.selection_frame,
            MessageDirection::ToWidget,
            true,
        ));
        ui.send_message(WidgetMessage::desired_position(
            self.selection_frame,
            MessageDirection::ToWidget,
            mouse_pos,
        ));
        ui.send_message(WidgetMessage::width(
            self.selection_frame,
            MessageDirection::ToWidget,
            0.0,
        ));
        ui.send_message(WidgetMessage::height(
            self.selection_frame,
            MessageDirection::ToWidget,
            0.0,
        ));
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        _mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        let scene = &engine.scenes[editor_scene.scene];
        let camera = scene.graph[editor_scene.camera_controller.camera].as_camera();
        let preview_screen_bounds = engine.user_interface.node(self.preview).screen_bounds();
        let frame_screen_bounds = engine
            .user_interface
            .node(self.selection_frame)
            .screen_bounds();
        let relative_bounds = frame_screen_bounds.translate(-preview_screen_bounds.position);
        self.stack.clear();
        self.stack.push(scene.graph.get_root());
        let mut graph_selection = GraphSelection::default();
        while let Some(handle) = self.stack.pop() {
            let node = &scene.graph[handle];
            if handle == editor_scene.root {
                continue;
            }
            if handle == scene.graph.get_root() {
                self.stack.extend_from_slice(node.children());
                continue;
            }
            let aabb = match node {
                Node::Base(_) => AxisAlignedBoundingBox::unit(),
                Node::Light(_) => AxisAlignedBoundingBox::unit(),
                Node::Camera(_) => AxisAlignedBoundingBox::unit(),
                Node::Mesh(mesh) => mesh.bounding_box(),
                Node::Sprite(_) => AxisAlignedBoundingBox::unit(),
                Node::ParticleSystem(_) => AxisAlignedBoundingBox::unit(),
                Node::Terrain(ref terrain) => terrain.bounding_box(),
            };

            for screen_corner in aabb
                .corners()
                .iter()
                .filter_map(|&p| camera.project(p + node.global_position(), frame_size))
            {
                if relative_bounds.contains(screen_corner) {
                    graph_selection.insert_or_exclude(handle);
                    break;
                }
            }

            self.stack.extend_from_slice(node.children());
        }

        let new_selection = Selection::Graph(graph_selection);

        if !new_selection.is_empty() && new_selection != editor_scene.selection {
            self.message_sender
                .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                    ChangeSelectionCommand::new(new_selection, editor_scene.selection.clone()),
                )))
                .unwrap();
        }
        engine
            .user_interface
            .send_message(WidgetMessage::visibility(
                self.selection_frame,
                MessageDirection::ToWidget,
                false,
            ));
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _camera: Handle<Node>,
        _editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        _frame_size: Vector2<f32>,
    ) {
        let ui = &mut engine.user_interface;
        let width = mouse_position.x - self.click_pos.x;
        let height = mouse_position.y - self.click_pos.y;

        let position = Vector2::new(
            if width < 0.0 {
                mouse_position.x
            } else {
                self.click_pos.x
            },
            if height < 0.0 {
                mouse_position.y
            } else {
                self.click_pos.y
            },
        );
        ui.send_message(WidgetMessage::desired_position(
            self.selection_frame,
            MessageDirection::ToWidget,
            position,
        ));
        ui.send_message(WidgetMessage::width(
            self.selection_frame,
            MessageDirection::ToWidget,
            width.abs(),
        ));
        ui.send_message(WidgetMessage::height(
            self.selection_frame,
            MessageDirection::ToWidget,
            height.abs(),
        ));
    }

    fn update(
        &mut self,
        _editor_scene: &mut EditorScene,
        _camera: Handle<Node>,
        _engine: &mut GameEngine,
    ) {
    }

    fn deactivate(&mut self, _editor_scene: &EditorScene, _engine: &mut GameEngine) {}
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
            frame_size
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
