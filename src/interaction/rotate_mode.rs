use crate::scene::commands::SceneCommand;
use crate::world::graph::selection::GraphSelection;
use crate::{
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::rotate_gizmo::RotationGizmo, InteractionModeTrait,
    },
    scene::{
        commands::{graph::RotateNodeCommand, ChangeSelectionCommand, CommandGroup},
        EditorScene, Selection,
    },
    settings::Settings,
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector2},
        pool::Handle,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct RotateInteractionMode {
    initial_rotations: Vec<UnitQuaternion<f32>>,
    rotation_gizmo: RotationGizmo,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl RotateInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            initial_rotations: Default::default(),
            rotation_gizmo: RotationGizmo::new(editor_scene, engine),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionModeTrait for RotateInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        // Pick gizmo nodes.
        let camera = editor_scene.camera_controller.camera;
        let camera_pivot = editor_scene.camera_controller.pivot;
        if let Some(result) = editor_scene.camera_controller.pick(
            mouse_pos,
            graph,
            editor_scene.root,
            frame_size,
            true,
            |handle, _| {
                handle != camera && handle != camera_pivot && handle != self.rotation_gizmo.origin
            },
        ) {
            if self
                .rotation_gizmo
                .handle_pick(result.node, editor_scene, engine)
            {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                if let Selection::Graph(selection) = &editor_scene.selection {
                    self.interacting = true;
                    self.initial_rotations = selection.local_rotations(graph);
                }
            }
        }
    }

    fn on_left_mouse_button_up(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if self.interacting {
            if let Selection::Graph(selection) = &editor_scene.selection {
                if !selection.is_empty() {
                    self.interacting = false;
                    let current_rotation = selection.local_rotations(graph);
                    if current_rotation != self.initial_rotations {
                        let commands = CommandGroup::from(
                            selection
                                .nodes()
                                .iter()
                                .zip(self.initial_rotations.iter().zip(current_rotation.iter()))
                                .map(|(&node, (&old_rotation, &new_rotation))| {
                                    SceneCommand::new(RotateNodeCommand::new(
                                        node,
                                        old_rotation,
                                        new_rotation,
                                    ))
                                })
                                .collect::<Vec<SceneCommand>>(),
                        );
                        // Commit changes.
                        self.message_sender
                            .send(Message::do_scene_command(commands))
                            .unwrap();
                    }
                }
            }
        } else {
            let new_selection = editor_scene
                .camera_controller
                .pick(
                    mouse_pos,
                    graph,
                    editor_scene.root,
                    frame_size,
                    false,
                    |_, _| true,
                )
                .map(|result| {
                    if let (Selection::Graph(selection), true) = (
                        &editor_scene.selection,
                        engine.user_interface.keyboard_modifiers().control,
                    ) {
                        let mut selection = selection.clone();
                        selection.insert_or_exclude(result.node);
                        Selection::Graph(selection)
                    } else {
                        Selection::Graph(GraphSelection::single_or_empty(result.node))
                    }
                })
                .unwrap_or_else(|| Selection::Graph(GraphSelection::default()));

            if new_selection != editor_scene.selection {
                self.message_sender
                    .send(Message::do_scene_command(ChangeSelectionCommand::new(
                        new_selection,
                        editor_scene.selection.clone(),
                    )))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        _settings: &Settings,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            if self.interacting {
                let rotation_delta = self.rotation_gizmo.calculate_rotation_delta(
                    editor_scene,
                    camera,
                    mouse_offset,
                    mouse_position,
                    engine,
                    frame_size,
                );
                for &node in selection.nodes().iter() {
                    let transform =
                        engine.scenes[editor_scene.scene].graph[node].local_transform_mut();
                    let rotation = **transform.rotation();
                    transform.set_rotation(rotation * rotation_delta);
                }
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &mut EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            if !editor_scene.selection.is_empty() {
                let scale =
                    calculate_gizmo_distance_scaling(graph, camera, self.rotation_gizmo.origin);
                self.rotation_gizmo.sync_transform(graph, selection, scale);
                self.rotation_gizmo.set_visible(graph, true);
            } else {
                self.rotation_gizmo.set_visible(graph, false);
            }
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.rotation_gizmo.set_visible(graph, false);
    }
}
