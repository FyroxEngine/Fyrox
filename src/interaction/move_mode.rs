use crate::{
    camera::CameraController,
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::move_gizmo::MoveGizmo, plane::PlaneKind,
        InteractionModeTrait,
    },
    scene::{
        commands::{graph::MoveNodeCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, GraphSelection, Selection,
    },
    settings::Settings,
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        math::plane::Plane,
        pool::Handle,
    },
    scene::{graph::Graph, node::Node},
};
use std::sync::mpsc::Sender;

struct Entry {
    node_handle: Handle<Node>,
    initial_offset_gizmo_space: Vector3<f32>,
    initial_local_position: Vector3<f32>,
    initial_parent_inv_global_transform: Matrix4<f32>,
    new_local_position: Vector3<f32>,
}

struct MoveContext {
    plane: Plane,
    objects: Vec<Entry>,
    plane_kind: PlaneKind,
    gizmo_inv_transform: Matrix4<f32>,
    gizmo_local_transform: Matrix4<f32>,
}

impl MoveContext {
    pub fn from_graph_selection(
        selection: &GraphSelection,
        graph: &Graph,
        move_gizmo: &MoveGizmo,
        camera_controller: &CameraController,
        plane_kind: PlaneKind,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) -> Self {
        let gizmo_origin = &graph[move_gizmo.origin];

        let gizmo_inv_transform = gizmo_origin
            .global_transform()
            .try_inverse()
            .unwrap_or_default();

        let look_direction =
            gizmo_inv_transform.transform_vector(&graph[camera_controller.camera].look_vector());

        let plane = plane_kind.make_plane_from_view(look_direction);

        let plane_point = plane_kind.project_point(
            camera_controller
                .pick_on_plane(plane, graph, mouse_pos, frame_size, gizmo_inv_transform)
                .unwrap_or_default(),
        );

        Self {
            plane,
            objects: selection
                .root_nodes(graph)
                .iter()
                .map(|&node_handle| {
                    let node = &graph[node_handle];
                    Entry {
                        node_handle,
                        initial_offset_gizmo_space: gizmo_inv_transform
                            .transform_point(&Point3::from(node.global_position()))
                            .coords
                            - plane_point
                            - gizmo_inv_transform.transform_vector(
                                &(node.global_position() - gizmo_origin.global_position()),
                            ),
                        new_local_position: **node.local_transform().position(),
                        initial_local_position: **node.local_transform().position(),
                        initial_parent_inv_global_transform: if node.parent().is_some() {
                            graph[node.parent()]
                                .global_transform()
                                .try_inverse()
                                .unwrap_or_default()
                        } else {
                            Matrix4::identity()
                        },
                    }
                })
                .collect(),
            gizmo_local_transform: gizmo_origin.local_transform().matrix(),
            gizmo_inv_transform,
            plane_kind,
        }
    }

    pub fn update(
        &mut self,
        graph: &Graph,
        camera_controller: &CameraController,
        settings: &Settings,
        mouse_position: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        if let Some(picked_position_gizmo_space) = camera_controller
            .pick_on_plane(
                self.plane,
                graph,
                mouse_position,
                frame_size,
                self.gizmo_inv_transform,
            )
            .map(|p| self.plane_kind.project_point(p))
        {
            for entry in self.objects.iter_mut() {
                let mut new_local_position = entry.initial_local_position
                    + entry.initial_parent_inv_global_transform.transform_vector(
                        &self.gizmo_local_transform.transform_vector(
                            &(picked_position_gizmo_space + entry.initial_offset_gizmo_space),
                        ),
                    );

                // Snap to grid if needed.
                if settings.move_mode_settings.grid_snapping {
                    fn round_to_step(x: f32, step: f32) -> f32 {
                        x - x % step
                    }

                    new_local_position = Vector3::new(
                        round_to_step(
                            new_local_position.x,
                            settings.move_mode_settings.x_snap_step,
                        ),
                        round_to_step(
                            new_local_position.y,
                            settings.move_mode_settings.y_snap_step,
                        ),
                        round_to_step(
                            new_local_position.z,
                            settings.move_mode_settings.z_snap_step,
                        ),
                    );
                }

                entry.new_local_position = new_local_position;
            }
        }
    }
}

pub struct MoveInteractionMode {
    move_context: Option<MoveContext>,
    move_gizmo: MoveGizmo,
    message_sender: Sender<Message>,
}

impl MoveInteractionMode {
    pub fn new(
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
    ) -> Self {
        Self {
            move_context: None,
            move_gizmo: MoveGizmo::new(editor_scene, engine),
            message_sender,
        }
    }
}

impl InteractionModeTrait for MoveInteractionMode {
    fn on_left_mouse_button_down(
        &mut self,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        mouse_pos: Vector2<f32>,
        frame_size: Vector2<f32>,
    ) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        let camera = editor_scene.camera_controller.camera;
        let camera_pivot = editor_scene.camera_controller.pivot;
        if let Some(result) = editor_scene.camera_controller.pick(
            mouse_pos,
            graph,
            editor_scene.root,
            frame_size,
            true,
            |handle, _| {
                handle != camera && handle != camera_pivot && handle != self.move_gizmo.origin
            },
        ) {
            if let Some(plane_kind) = self.move_gizmo.handle_pick(result.node, graph) {
                if let Selection::Graph(selection) = &editor_scene.selection {
                    self.move_context = Some(MoveContext::from_graph_selection(
                        selection,
                        graph,
                        &self.move_gizmo,
                        &editor_scene.camera_controller,
                        plane_kind,
                        mouse_pos,
                        frame_size,
                    ));
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

        if let Some(move_context) = self.move_context.take() {
            let mut changed = false;

            for initial_state in move_context.objects.iter() {
                if **graph[initial_state.node_handle]
                    .local_transform()
                    .position()
                    != initial_state.initial_local_position
                {
                    changed = true;
                    break;
                }
            }

            if changed {
                let commands = CommandGroup::from(
                    move_context
                        .objects
                        .iter()
                        .map(|initial_state| {
                            SceneCommand::MoveNode(MoveNodeCommand::new(
                                initial_state.node_handle,
                                initial_state.initial_local_position,
                                **graph[initial_state.node_handle]
                                    .local_transform()
                                    .position(),
                            ))
                        })
                        .collect::<Vec<SceneCommand>>(),
                );

                // Commit changes.
                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                        commands,
                    )))
                    .unwrap();
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
                    .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                        ChangeSelectionCommand::new(new_selection, editor_scene.selection.clone()),
                    )))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(
        &mut self,
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        if let Some(move_context) = self.move_context.as_mut() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;

            move_context.update(
                graph,
                &editor_scene.camera_controller,
                settings,
                mouse_position,
                frame_size,
            );

            for entry in move_context.objects.iter() {
                graph[entry.node_handle]
                    .local_transform_mut()
                    .set_position(entry.new_local_position);
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
            if !editor_scene.selection.is_empty() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let scale = calculate_gizmo_distance_scaling(graph, camera, self.move_gizmo.origin);
                self.move_gizmo.sync_transform(graph, selection, scale);
                self.move_gizmo.set_visible(graph, true);
            } else {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                self.move_gizmo.set_visible(graph, false);
            }
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.move_gizmo.set_visible(graph, false);
    }
}
