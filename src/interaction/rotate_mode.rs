use crate::{
    interaction::{calculate_gizmo_distance_scaling, InteractionModeTrait},
    scene::{
        commands::{graph::RotateNodeCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, GraphSelection, Selection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::{plane::Plane, Matrix4Ext},
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        graph::Graph,
        mesh::surface::{SurfaceBuilder, SurfaceData},
        mesh::{MeshBuilder, RenderPath},
        node::Node,
        transform::TransformBuilder,
    },
};
use std::sync::{mpsc::Sender, Arc, RwLock};

pub enum RotateGizmoMode {
    Pitch,
    Yaw,
    Roll,
}

pub struct RotationGizmo {
    mode: RotateGizmoMode,
    origin: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_rotation_ribbon(
    graph: &mut Graph,
    rotation: UnitQuaternion<f32>,
    color: Color,
    name: &str,
) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_depth_offset(0.5)
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_rotation(rotation)
                    .build(),
            ),
    )
    .with_render_path(RenderPath::Forward)
    .with_cast_shadows(false)
    .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
        SurfaceData::make_cylinder(
            30,
            0.5,
            0.05,
            false,
            &Matrix4::new_translation(&Vector3::new(0.0, -0.05, 0.0)),
        ),
    )))
    .with_color(color)
    .build()])
    .build(graph)
}

impl RotationGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = MeshBuilder::new(
            BaseBuilder::new()
                .with_name("Origin")
                .with_depth_offset(0.5)
                .with_visibility(false),
        )
        .with_render_path(RenderPath::Forward)
        .with_cast_shadows(false)
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
            SurfaceData::make_sphere(10, 10, 0.1, &Matrix4::identity()),
        )))
        .with_color(Color::opaque(100, 100, 100))
        .build()])
        .build(graph);

        graph.link_nodes(origin, editor_scene.root);

        let x_axis = make_rotation_ribbon(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let y_axis = make_rotation_ribbon(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let z_axis = make_rotation_ribbon(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        Self {
            mode: RotateGizmoMode::Pitch,
            origin,
            x_axis,
            y_axis,
            z_axis,
        }
    }

    pub fn set_mode(&mut self, mode: RotateGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        graph[self.origin].as_mesh_mut().set_color(Color::WHITE);
        graph[self.x_axis].as_mesh_mut().set_color(Color::RED);
        graph[self.y_axis].as_mesh_mut().set_color(Color::GREEN);
        graph[self.z_axis].as_mesh_mut().set_color(Color::BLUE);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            RotateGizmoMode::Pitch => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Yaw => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Roll => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
            }
        }
    }

    pub fn handle_pick(
        &mut self,
        picked: Handle<Node>,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis {
            self.set_mode(RotateGizmoMode::Pitch, graph);
            true
        } else if picked == self.y_axis {
            self.set_mode(RotateGizmoMode::Yaw, graph);
            true
        } else if picked == self.z_axis {
            self.set_mode(RotateGizmoMode::Roll, graph);
            true
        } else {
            false
        }
    }

    pub fn calculate_rotation_delta(
        &self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        engine: &GameEngine,
        frame_size: Vector2<f32>,
    ) -> UnitQuaternion<f32> {
        let graph = &engine.scenes[editor_scene.scene].graph;

        if let Node::Camera(camera) = &graph[camera] {
            let transform = graph[self.origin].global_transform();

            let initial_ray = camera.make_ray(mouse_position, frame_size);
            let offset_ray = camera.make_ray(mouse_position + mouse_offset, frame_size);

            let oriented_axis = match self.mode {
                RotateGizmoMode::Pitch => transform.side(),
                RotateGizmoMode::Yaw => transform.up(),
                RotateGizmoMode::Roll => transform.look(),
            };

            let plane = Plane::from_normal_and_point(&oriented_axis, &transform.position())
                .unwrap_or_default();

            if let Some(old_pos) = initial_ray.plane_intersection_point(&plane) {
                if let Some(new_pos) = offset_ray.plane_intersection_point(&plane) {
                    let center = transform.position();
                    let old = (old_pos - center)
                        .try_normalize(std::f32::EPSILON)
                        .unwrap_or_default();
                    let new = (new_pos - center)
                        .try_normalize(std::f32::EPSILON)
                        .unwrap_or_default();

                    let angle_delta = old.dot(&new).max(-1.0).min(1.0).acos();
                    let sign = old.cross(&new).dot(&oriented_axis).signum();

                    let static_axis = match self.mode {
                        RotateGizmoMode::Pitch => Vector3::x_axis(),
                        RotateGizmoMode::Yaw => Vector3::y_axis(),
                        RotateGizmoMode::Roll => Vector3::z_axis(),
                    };
                    return UnitQuaternion::from_axis_angle(&static_axis, sign * angle_delta);
                }
            }
        }

        UnitQuaternion::default()
    }

    pub fn sync_transform(
        &self,
        graph: &mut Graph,
        selection: &GraphSelection,
        scale: Vector3<f32>,
    ) {
        if let Some((rotation, position)) = selection.global_rotation_position(graph) {
            graph[self.origin]
                .set_visibility(true)
                .local_transform_mut()
                .set_rotation(rotation)
                .set_position(position)
                .set_scale(scale);
        }
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

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
        let editor_node = editor_scene.camera_controller.pick(
            mouse_pos,
            graph,
            editor_scene.root,
            frame_size,
            true,
            |handle, _| {
                handle != camera && handle != camera_pivot && handle != self.rotation_gizmo.origin
            },
        );

        if self
            .rotation_gizmo
            .handle_pick(editor_node, editor_scene, engine)
        {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            if let Selection::Graph(selection) = &editor_scene.selection {
                self.interacting = true;
                self.initial_rotations = selection.local_rotations(graph);
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
                                    SceneCommand::RotateNode(RotateNodeCommand::new(
                                        node,
                                        old_rotation,
                                        new_rotation,
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
                }
            }
        } else {
            let picked = editor_scene.camera_controller.pick(
                mouse_pos,
                graph,
                editor_scene.root,
                frame_size,
                false,
                |_, _| true,
            );
            let new_selection =
                if engine.user_interface.keyboard_modifiers().control && picked.is_some() {
                    if let Selection::Graph(selection) = &editor_scene.selection {
                        let mut selection = selection.clone();
                        selection.insert_or_exclude(picked);
                        Selection::Graph(selection)
                    } else {
                        Selection::Graph(GraphSelection::single_or_empty(picked))
                    }
                } else {
                    Selection::Graph(GraphSelection::single_or_empty(picked))
                };
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
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        camera: Handle<Node>,
        editor_scene: &mut EditorScene,
        engine: &mut GameEngine,
        frame_size: Vector2<f32>,
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
            if !editor_scene.selection.is_empty() {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                let scale =
                    calculate_gizmo_distance_scaling(graph, camera, self.rotation_gizmo.origin);
                self.rotation_gizmo.sync_transform(graph, selection, scale);
                self.rotation_gizmo.set_visible(graph, true);
            } else {
                let graph = &mut engine.scenes[editor_scene.scene].graph;
                self.rotation_gizmo.set_visible(graph, false);
            }
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.rotation_gizmo.set_visible(graph, false);
    }
}
