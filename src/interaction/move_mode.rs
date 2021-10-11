use crate::world::graph::selection::GraphSelection;
use crate::{
    camera::CameraController,
    interaction::{
        calculate_gizmo_distance_scaling, gizmo::move_gizmo::MoveGizmo, plane::PlaneKind,
        InteractionModeTrait,
    },
    scene::{
        commands::{
            graph::MoveNodeCommand, sound::MoveSpatialSoundSourceCommand, ChangeSelectionCommand,
            CommandGroup, SceneCommand,
        },
        EditorScene, Selection,
    },
    settings::Settings,
    world::sound::selection::SoundSelection,
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        math::plane::Plane,
        pool::Handle,
    },
    scene::{graph::Graph, node::Node, Scene},
    sound::{context::SoundContext, source::SoundSource},
};
use std::sync::mpsc::Sender;

#[derive(Copy, Clone)]
enum MovableEntity {
    Node(Handle<Node>),
    Sound(Handle<SoundSource>),
}

impl MovableEntity {
    fn position(&self, scene: &Scene) -> Vector3<f32> {
        match *self {
            MovableEntity::Node(node) => **scene.graph[node].local_transform().position(),
            MovableEntity::Sound(sound) => {
                let state = scene.sound_context.state();
                match state.source(sound) {
                    SoundSource::Generic(_) => Vector3::default(),
                    SoundSource::Spatial(spatial) => spatial.position(),
                }
            }
        }
    }

    fn set_position(&self, scene: &mut Scene, position: Vector3<f32>) {
        match *self {
            MovableEntity::Node(node) => {
                scene.graph[node]
                    .local_transform_mut()
                    .set_position(position);
            }
            MovableEntity::Sound(sound) => {
                let mut state = scene.sound_context.state();
                if let SoundSource::Spatial(spatial) = state.source_mut(sound) {
                    spatial.set_position(position);
                }
            }
        }
    }
}

struct Entry {
    entity: MovableEntity,
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
                        entity: MovableEntity::Node(node_handle),
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

    pub fn from_sound_selection(
        selection: &SoundSelection,
        sound_context: &SoundContext,
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

        let state = sound_context.state();

        Self {
            plane,
            objects: selection
                .sources()
                .iter()
                .map(|&source_handle| {
                    let source = state.source(source_handle);
                    match source {
                        SoundSource::Generic(_) => None,
                        SoundSource::Spatial(spatial) => Some(Entry {
                            entity: MovableEntity::Sound(source_handle),
                            initial_offset_gizmo_space: gizmo_inv_transform
                                .transform_point(&Point3::from(spatial.position()))
                                .coords
                                - plane_point
                                - gizmo_inv_transform.transform_vector(
                                    &(spatial.position() - gizmo_origin.global_position()),
                                ),
                            new_local_position: spatial.position(),
                            initial_local_position: spatial.position(),
                            initial_parent_inv_global_transform: Matrix4::identity(),
                        }),
                    }
                })
                .flatten()
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
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

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
                match &editor_scene.selection {
                    Selection::Graph(selection) => {
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
                    Selection::Sound(selection) => {
                        self.move_context = Some(MoveContext::from_sound_selection(
                            selection,
                            &scene.sound_context.clone(),
                            graph,
                            &self.move_gizmo,
                            &editor_scene.camera_controller,
                            plane_kind,
                            mouse_pos,
                            frame_size,
                        ));
                    }
                    _ => {}
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
        let scene = &mut engine.scenes[editor_scene.scene];

        if let Some(move_context) = self.move_context.take() {
            let mut changed = false;

            for initial_state in move_context.objects.iter() {
                if initial_state.entity.position(scene) != initial_state.initial_local_position {
                    changed = true;
                    break;
                }
            }

            if changed {
                let commands = CommandGroup::from(
                    move_context
                        .objects
                        .iter()
                        .map(|initial_state| match initial_state.entity {
                            MovableEntity::Node(node) => {
                                Some(SceneCommand::new(MoveNodeCommand::new(
                                    node,
                                    initial_state.initial_local_position,
                                    **scene.graph[node].local_transform().position(),
                                )))
                            }
                            MovableEntity::Sound(sound) => {
                                let state = scene.sound_context.state();
                                match state.source(sound) {
                                    SoundSource::Generic(_) => None,
                                    SoundSource::Spatial(spatial) => {
                                        Some(SceneCommand::new(MoveSpatialSoundSourceCommand::new(
                                            sound,
                                            initial_state.initial_local_position,
                                            spatial.position(),
                                        )))
                                    }
                                }
                            }
                        })
                        .flatten()
                        .collect::<Vec<_>>(),
                );

                // Commit changes.
                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::new(commands)))
                    .unwrap();
            }
        } else {
            let new_selection = editor_scene
                .camera_controller
                .pick(
                    mouse_pos,
                    &scene.graph,
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
        _mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        _camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
        settings: &Settings,
    ) {
        if let Some(move_context) = self.move_context.as_mut() {
            let scene = &mut engine.scenes[editor_scene.scene];
            let graph = &mut scene.graph;

            move_context.update(
                graph,
                &editor_scene.camera_controller,
                settings,
                mouse_position,
                frame_size,
            );

            for entry in move_context.objects.iter() {
                entry.entity.set_position(scene, entry.new_local_position);
            }
        }
    }

    fn update(
        &mut self,
        editor_scene: &mut EditorScene,
        camera: Handle<Node>,
        engine: &mut GameEngine,
    ) {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        if !editor_scene.selection.is_empty() {
            let scale = calculate_gizmo_distance_scaling(graph, camera, self.move_gizmo.origin);
            self.move_gizmo.set_visible(graph, true);
            self.move_gizmo
                .sync_transform(scene, &editor_scene.selection, scale);
        } else {
            self.move_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.move_gizmo.set_visible(graph, false);
    }
}
