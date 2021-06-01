use crate::camera::CameraController;
use crate::{
    interaction::{calculate_gizmo_distance_scaling, InteractionModeTrait},
    scene::{
        commands::{graph::MoveNodeCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, GraphSelection, Selection,
    },
    settings::Settings,
    GameEngine, Message,
};
use rg3d::core::algebra::Point3;
use rg3d::core::math::Matrix4Ext;
use rg3d::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::plane::Plane,
        pool::Handle,
    },
    scene::{
        base::BaseBuilder,
        graph::Graph,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder, RenderPath,
        },
        node::Node,
        transform::{Transform, TransformBuilder},
    },
};
use std::sync::{mpsc::Sender, Arc, RwLock};

#[derive(Copy, Clone, Debug)]
pub enum MovePlaneKind {
    X,
    Y,
    Z,
    XY,
    YZ,
    ZX,
}

impl MovePlaneKind {
    fn make_plane(self, look_direction: Vector3<f32>) -> Plane {
        match self {
            MovePlaneKind::X => Plane::from_normal_and_point(
                &Vector3::new(0.0, look_direction.y, look_direction.z),
                &Default::default(),
            ),
            MovePlaneKind::Y => Plane::from_normal_and_point(
                &Vector3::new(look_direction.x, 0.0, look_direction.z),
                &Default::default(),
            ),
            MovePlaneKind::Z => Plane::from_normal_and_point(
                &Vector3::new(look_direction.x, look_direction.y, 0.0),
                &Default::default(),
            ),
            MovePlaneKind::YZ => Plane::from_normal_and_point(&Vector3::x(), &Default::default()),
            MovePlaneKind::ZX => Plane::from_normal_and_point(&Vector3::y(), &Default::default()),
            MovePlaneKind::XY => Plane::from_normal_and_point(&Vector3::z(), &Default::default()),
        }
        .unwrap_or_default()
    }

    fn project_point(self, point: Vector3<f32>) -> Vector3<f32> {
        match self {
            MovePlaneKind::X => Vector3::new(point.x, 0.0, 0.0),
            MovePlaneKind::Y => Vector3::new(0.0, point.y, 0.0),
            MovePlaneKind::Z => Vector3::new(0.0, 0.0, point.z),
            MovePlaneKind::XY => Vector3::new(point.x, point.y, 0.0),
            MovePlaneKind::YZ => Vector3::new(0.0, point.y, point.z),
            MovePlaneKind::ZX => Vector3::new(point.x, 0.0, point.z),
        }
    }
}

pub struct MoveGizmo {
    pub origin: Handle<Node>,
    x_arrow: Handle<Node>,
    y_arrow: Handle<Node>,
    z_arrow: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
    xy_plane: Handle<Node>,
    yz_plane: Handle<Node>,
    zx_plane: Handle<Node>,
}

fn make_move_axis(
    graph: &mut Graph,
    rotation: UnitQuaternion<f32>,
    color: Color,
    name_prefix: &str,
) -> (Handle<Node>, Handle<Node>) {
    let arrow;
    let axis = MeshBuilder::new(
        BaseBuilder::new()
            .with_children(&[{
                arrow = MeshBuilder::new(
                    BaseBuilder::new()
                        .with_name(name_prefix.to_owned() + "Arrow")
                        .with_depth_offset(0.5)
                        .with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(Vector3::new(0.0, 1.0, 0.0))
                                .build(),
                        ),
                )
                .with_render_path(RenderPath::Forward)
                .with_cast_shadows(false)
                .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
                    SurfaceData::make_cone(10, 0.05, 0.1, &Matrix4::identity()),
                )))
                .with_color(color)
                .build()])
                .build(graph);
                arrow
            }])
            .with_name(name_prefix.to_owned() + "Axis")
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
        SurfaceData::make_cylinder(10, 0.015, 1.0, true, &Matrix4::identity()),
    )))
    .with_color(color)
    .build()])
    .build(graph);

    (axis, arrow)
}

fn create_quad_plane(
    graph: &mut Graph,
    transform: Matrix4<f32>,
    color: Color,
    name: &str,
) -> Handle<Node> {
    MeshBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_depth_offset(0.5)
            .with_local_transform(
                TransformBuilder::new()
                    .with_local_scale(Vector3::new(0.15, 0.15, 0.15))
                    .build(),
            ),
    )
    .with_render_path(RenderPath::Forward)
    .with_cast_shadows(false)
    .with_surfaces(vec![{
        SurfaceBuilder::new(Arc::new(RwLock::new(SurfaceData::make_quad(
            &(transform
                * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                    .to_homogeneous()),
        ))))
        .with_color(color)
        .build()
    }])
    .build(graph)
}

impl MoveGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = BaseBuilder::new()
            .with_name("Origin")
            .with_visibility(false)
            .build(graph);

        graph.link_nodes(origin, editor_scene.root);

        let (x_axis, x_arrow) = make_move_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 90.0f32.to_radians()),
            Color::RED,
            "X",
        );
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_move_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 0.0f32.to_radians()),
            Color::GREEN,
            "Y",
        );
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_move_axis(
            graph,
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians()),
            Color::BLUE,
            "Z",
        );
        graph.link_nodes(z_axis, origin);

        let xy_transform = Matrix4::new_translation(&Vector3::new(-0.5, 0.5, 0.0))
            * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                .to_homogeneous();
        let xy_plane = create_quad_plane(graph, xy_transform, Color::BLUE, "XYPlane");
        graph.link_nodes(xy_plane, origin);

        let yz_transform = Matrix4::new_translation(&Vector3::new(0.0, 0.5, 0.5))
            * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 90.0f32.to_radians())
                .to_homogeneous();
        let yz_plane = create_quad_plane(graph, yz_transform, Color::RED, "YZPlane");
        graph.link_nodes(yz_plane, origin);

        let zx_plane = create_quad_plane(
            graph,
            Matrix4::new_translation(&Vector3::new(-0.5, 0.0, 0.5)),
            Color::GREEN,
            "ZXPlane",
        );
        graph.link_nodes(zx_plane, origin);

        Self {
            origin,
            x_arrow,
            y_arrow,
            z_arrow,
            x_axis,
            y_axis,
            z_axis,
            zx_plane,
            yz_plane,
            xy_plane,
        }
    }

    pub fn apply_mode(&mut self, mode: Option<MovePlaneKind>, graph: &mut Graph) {
        // Restore initial colors first.
        graph[self.x_axis].as_mesh_mut().set_color(Color::RED);
        graph[self.x_arrow].as_mesh_mut().set_color(Color::RED);
        graph[self.y_axis].as_mesh_mut().set_color(Color::GREEN);
        graph[self.y_arrow].as_mesh_mut().set_color(Color::GREEN);
        graph[self.z_axis].as_mesh_mut().set_color(Color::BLUE);
        graph[self.z_arrow].as_mesh_mut().set_color(Color::BLUE);
        graph[self.zx_plane].as_mesh_mut().set_color(Color::GREEN);
        graph[self.yz_plane].as_mesh_mut().set_color(Color::RED);
        graph[self.xy_plane].as_mesh_mut().set_color(Color::BLUE);

        if let Some(mode) = mode {
            let yellow = Color::opaque(255, 255, 0);
            match mode {
                MovePlaneKind::X => {
                    graph[self.x_axis].as_mesh_mut().set_color(yellow);
                    graph[self.x_arrow].as_mesh_mut().set_color(yellow);
                }
                MovePlaneKind::Y => {
                    graph[self.y_axis].as_mesh_mut().set_color(yellow);
                    graph[self.y_arrow].as_mesh_mut().set_color(yellow);
                }
                MovePlaneKind::Z => {
                    graph[self.z_axis].as_mesh_mut().set_color(yellow);
                    graph[self.z_arrow].as_mesh_mut().set_color(yellow);
                }
                MovePlaneKind::XY => {
                    graph[self.xy_plane].as_mesh_mut().set_color(yellow);
                }
                MovePlaneKind::YZ => {
                    graph[self.yz_plane].as_mesh_mut().set_color(yellow);
                }
                MovePlaneKind::ZX => {
                    graph[self.zx_plane].as_mesh_mut().set_color(yellow);
                }
            }
        }
    }

    pub fn handle_pick(
        &mut self,
        picked: Handle<Node>,
        graph: &mut Graph,
    ) -> Option<MovePlaneKind> {
        let mode = if picked == self.x_axis || picked == self.x_arrow {
            Some(MovePlaneKind::X)
        } else if picked == self.y_axis || picked == self.y_arrow {
            Some(MovePlaneKind::Y)
        } else if picked == self.z_axis || picked == self.z_arrow {
            Some(MovePlaneKind::Z)
        } else if picked == self.zx_plane {
            Some(MovePlaneKind::ZX)
        } else if picked == self.xy_plane {
            Some(MovePlaneKind::XY)
        } else if picked == self.yz_plane {
            Some(MovePlaneKind::YZ)
        } else {
            None
        };

        self.apply_mode(mode, graph);

        mode
    }

    pub fn transform<'a>(&self, graph: &'a mut Graph) -> &'a mut Transform {
        graph[self.origin].local_transform_mut()
    }

    pub fn calculate_offset(
        &self,
        editor_scene: &EditorScene,
        camera: Handle<Node>,
        mouse_offset: Vector2<f32>,
        mouse_position: Vector2<f32>,
        engine: &GameEngine,
        frame_size: Vector2<f32>,
        plane_kind: MovePlaneKind,
    ) -> Vector3<f32> {
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;
        let node_global_transform = graph[self.origin].global_transform();
        let node_local_transform = graph[self.origin].local_transform().matrix();

        if let Node::Camera(camera) = &graph[camera] {
            let inv_node_transform = node_global_transform
                .try_inverse()
                .unwrap_or_else(Matrix4::identity);

            // Create two rays in object space.
            let initial_ray = camera
                .make_ray(mouse_position, frame_size)
                .transform(inv_node_transform);
            let offset_ray = camera
                .make_ray(mouse_position + mouse_offset, frame_size)
                .transform(inv_node_transform);

            let dlook = inv_node_transform
                .transform_vector(&(node_global_transform.position() - camera.global_position()));

            // Select plane by current active mode.
            let plane = match plane_kind {
                MovePlaneKind::X => Plane::from_normal_and_point(
                    &Vector3::new(0.0, dlook.y, dlook.z),
                    &Vector3::default(),
                ),
                MovePlaneKind::Y => Plane::from_normal_and_point(
                    &Vector3::new(dlook.x, 0.0, dlook.z),
                    &Vector3::default(),
                ),
                MovePlaneKind::Z => Plane::from_normal_and_point(
                    &Vector3::new(dlook.x, dlook.y, 0.0),
                    &Vector3::default(),
                ),
                MovePlaneKind::YZ => {
                    Plane::from_normal_and_point(&Vector3::x(), &Vector3::default())
                }
                MovePlaneKind::ZX => {
                    Plane::from_normal_and_point(&Vector3::y(), &Vector3::default())
                }
                MovePlaneKind::XY => {
                    Plane::from_normal_and_point(&Vector3::z(), &Vector3::default())
                }
            }
            .unwrap_or_default();

            // Get two intersection points with plane and use delta between them to calculate offset.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let delta = next_point - initial_point;
                    let offset = match plane_kind {
                        MovePlaneKind::X => Vector3::new(delta.x, 0.0, 0.0),
                        MovePlaneKind::Y => Vector3::new(0.0, delta.y, 0.0),
                        MovePlaneKind::Z => Vector3::new(0.0, 0.0, delta.z),
                        MovePlaneKind::XY => Vector3::new(delta.x, delta.y, 0.0),
                        MovePlaneKind::YZ => Vector3::new(0.0, delta.y, delta.z),
                        MovePlaneKind::ZX => Vector3::new(delta.x, 0.0, delta.z),
                    };
                    // Make sure offset will be in local coordinates.
                    return node_local_transform.transform_vector(&offset);
                }
            }
        }

        Vector3::default()
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
    plane_kind: MovePlaneKind,
    gizmo_inv_transform: Matrix4<f32>,
    gizmo_local_transform: Matrix4<f32>,
}

impl MoveContext {
    pub fn from_graph_selection(
        selection: &GraphSelection,
        graph: &Graph,
        move_gizmo: &MoveGizmo,
        camera_controller: &CameraController,
        plane_kind: MovePlaneKind,
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

        let plane = plane_kind.make_plane(look_direction);

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
                            - plane_point,
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
