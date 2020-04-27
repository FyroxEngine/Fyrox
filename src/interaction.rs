use crate::{
    EditorScene,
    GameEngine,
    Message,
    command::{Command, MoveNodeCommand, ChangeSelectionCommand},
    camera::CameraController,
    command::ScaleNodeCommand,
};
use rg3d::{
    renderer::surface::{
        SurfaceBuilder,
        SurfaceSharedData,
    },
    scene::{
        base::BaseBuilder,
        node::Node,
        mesh::MeshBuilder,
        transform::{
            TransformBuilder,
            Transform,
        },
        graph::Graph,
    },
    core::{
        color::Color,
        pool::Handle,
        math::{
            vec3::Vec3,
            quat::Quat,
            vec2::Vec2,
            mat4::Mat4,
            plane::Plane,
        },
    },
};
use std::{
    sync::{
        Arc,
        Mutex,
        mpsc::Sender,
    }
};
use crate::command::RotateNodeCommand;

pub trait InteractionModeTrait {
    fn on_left_mouse_button_down(&mut self, editor_scene: &EditorScene, camera_controller: &mut CameraController, current_selection: Handle<Node>, engine: &mut GameEngine);
    fn on_left_mouse_button_up(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine);
    fn on_mouse_move(&mut self, mouse_offset: Vec2, camera: Handle<Node>, editor_scene: &EditorScene, engine: &mut GameEngine);
    fn update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine);
    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine);
    fn handle_message(&mut self, message: &Message);
}

#[derive(Copy, Clone, Debug)]
pub enum MoveGizmoMode {
    None,
    X,
    Y,
    Z,
    XY,
    YZ,
    ZX,
}

pub struct MoveGizmo {
    mode: MoveGizmoMode,
    origin: Handle<Node>,
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

fn make_move_axis(graph: &mut Graph, rotation: Quat, color: Color, name_prefix: &str) -> (Handle<Node>, Handle<Node>) {
    let axis = graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
        .with_name(name_prefix.to_owned() + "Axis")
        .with_depth_offset(0.5)
        .with_local_transform(TransformBuilder::new()
            .with_local_rotation(rotation)
            .build()))
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cylinder(10, 0.015, 1.0, true, Default::default()))))
            .with_color(color)
            .build()])
        .build()));
    let arrow = graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
        .with_name(name_prefix.to_owned() + "Arrow")
        .with_depth_offset(0.5)
        .with_local_transform(TransformBuilder::new()
            .with_local_position(Vec3::new(0.0, 1.0, 0.0))
            .build()))
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cone(10, 0.05, 0.1, Default::default()))))
            .with_color(color)
            .build()])
        .build()));
    graph.link_nodes(arrow, axis);
    (axis, arrow)
}

fn create_quad_plane(graph: &mut Graph, transform: Mat4, color: Color, name: &str) -> Handle<Node> {
    graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
        .with_name(name)
        .with_depth_offset(0.5)
        .with_local_transform(TransformBuilder::new()
            .with_local_scale(Vec3::new(0.15, 0.15, 0.15))
            .build()))
        .with_surfaces(vec![{
            SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_quad(transform))))
                .with_color(color)
                .build()
        }])
        .build()))
}

impl MoveGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = graph.add_node(Node::Base(BaseBuilder::new()
            .with_name("Origin")
            .with_visibility(false)
            .build()));

        graph.link_nodes(origin, editor_scene.root);

        let (x_axis, x_arrow) = make_move_axis(graph, Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()), Color::RED, "X");
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_move_axis(graph, Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.0f32.to_radians()), Color::GREEN, "Y");
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_move_axis(graph, Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()), Color::BLUE, "Z");
        graph.link_nodes(z_axis, origin);

        let xy_transform = Mat4::translate(Vec3::new(-0.5, 0.5, 0.0)) * Mat4::from_quat(Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()));
        let xy_plane = create_quad_plane(graph, xy_transform, Color::BLUE, "XYPlane");
        graph.link_nodes(xy_plane, origin);

        let yz_transform = Mat4::translate(Vec3::new(0.0, 0.5, 0.5)) * Mat4::from_quat(Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()));
        let yz_plane = create_quad_plane(graph, yz_transform, Color::RED, "YZPlane");
        graph.link_nodes(yz_plane, origin);

        let zx_plane = create_quad_plane(graph, Mat4::translate(Vec3::new(-0.5, 0.0, 0.5)), Color::GREEN, "ZXPlane");
        graph.link_nodes(zx_plane, origin);

        Self {
            mode: MoveGizmoMode::None,
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

    pub fn set_mode(&mut self, mode: MoveGizmoMode, graph: &mut Graph) {
        self.mode = mode;

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

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            MoveGizmoMode::X => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
                graph[self.x_arrow].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::Y => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
                graph[self.y_arrow].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::Z => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
                graph[self.z_arrow].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::XY => {
                graph[self.xy_plane].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::YZ => {
                graph[self.yz_plane].as_mesh_mut().set_color(yellow);
            }
            MoveGizmoMode::ZX => {
                graph[self.zx_plane].as_mesh_mut().set_color(yellow);
            }
            _ => ()
        }
    }

    pub fn handle_pick(&mut self,
                       picked: Handle<Node>,
                       editor_scene: &EditorScene,
                       engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis || picked == self.x_arrow {
            self.set_mode(MoveGizmoMode::X, graph);
            true
        } else if picked == self.y_axis || picked == self.y_arrow {
            self.set_mode(MoveGizmoMode::Y, graph);
            true
        } else if picked == self.z_axis || picked == self.z_arrow {
            self.set_mode(MoveGizmoMode::Z, graph);
            true
        } else if picked == self.zx_plane {
            self.set_mode(MoveGizmoMode::ZX, graph);
            true
        } else if picked == self.xy_plane {
            self.set_mode(MoveGizmoMode::XY, graph);
            true
        } else if picked == self.yz_plane {
            self.set_mode(MoveGizmoMode::YZ, graph);
            true
        } else {
            self.set_mode(MoveGizmoMode::None, graph);
            false
        }
    }

    pub fn calculate_offset(&self,
                            editor_scene: &EditorScene,
                            camera: Handle<Node>,
                            mouse_offset: Vec2,
                            node: Handle<Node>,
                            engine: &GameEngine,
    ) -> Vec3 {
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;
        let mouse_position = engine.user_interface.cursor_position();
        let screen_size = engine.renderer.get_frame_size();
        let screen_size = Vec2::new(screen_size.0 as f32, screen_size.1 as f32);
        let node_global_transform = graph[node].global_transform();
        let node_local_transform = graph[node].local_transform().matrix();

        if let Node::Camera(camera) = &graph[camera] {
            let dlook = node_global_transform.position() - camera.global_position();
            let inv_node_transform = node_global_transform.inverse().unwrap_or_default();

            // Create two rays in object space.
            let initial_ray = camera.make_ray(mouse_position, screen_size).transform(inv_node_transform);
            let offset_ray = camera.make_ray(mouse_position + mouse_offset, screen_size).transform(inv_node_transform);

            // Select plane by current active mode.
            let plane = match self.mode {
                MoveGizmoMode::None => return Vec3::ZERO,
                MoveGizmoMode::X => Plane::from_normal_and_point(&Vec3::new(0.0, dlook.y, dlook.z), &Vec3::ZERO),
                MoveGizmoMode::Y => Plane::from_normal_and_point(&Vec3::new(dlook.x, 0.0, dlook.z), &Vec3::ZERO),
                MoveGizmoMode::Z => Plane::from_normal_and_point(&Vec3::new(dlook.x, dlook.y, 0.0), &Vec3::ZERO),
                MoveGizmoMode::YZ => Plane::from_normal_and_point(&Vec3::RIGHT, &Vec3::ZERO),
                MoveGizmoMode::ZX => Plane::from_normal_and_point(&Vec3::UP, &Vec3::ZERO),
                MoveGizmoMode::XY => Plane::from_normal_and_point(&Vec3::LOOK, &Vec3::ZERO),
            }.unwrap_or_default();

            // Get two intersection points with plane and use delta between them to calculate offset.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let delta = next_point - initial_point;
                    let offset = match self.mode {
                        MoveGizmoMode::None => unreachable!(),
                        MoveGizmoMode::X => Vec3::new(delta.x, 0.0, 0.0),
                        MoveGizmoMode::Y => Vec3::new(0.0, delta.y, 0.0),
                        MoveGizmoMode::Z => Vec3::new(0.0, 0.0, delta.z),
                        MoveGizmoMode::XY => Vec3::new(delta.x, delta.y, 0.0),
                        MoveGizmoMode::YZ => Vec3::new(0.0, delta.y, delta.z),
                        MoveGizmoMode::ZX => Vec3::new(delta.x, 0.0, delta.z),
                    };
                    // Make sure offset will be in local coordinates.
                    return node_local_transform.transform_vector_normal(offset);
                }
            }
        }

        Vec3::ZERO
    }

    pub fn set_transform(&self, graph: &mut Graph, local_transform: Transform) {
        let gizmo_origin = &mut graph[self.origin];
        gizmo_origin.set_visibility(true);
        *gizmo_origin.local_transform_mut() = local_transform;
        // Drop scale because we need to have gizmo at same size.
        gizmo_origin.local_transform_mut().set_scale(Vec3::new(1.0, 1.0, 1.0));
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

pub struct MoveInteractionMode {
    initial_position: Vec3,
    move_gizmo: MoveGizmo,
    node: Handle<Node>,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl MoveInteractionMode {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine, message_sender: Sender<Message>) -> Self {
        Self {
            initial_position: Default::default(),
            move_gizmo: MoveGizmo::new(editor_scene, engine),
            node: Default::default(),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionModeTrait for MoveInteractionMode {
    fn on_left_mouse_button_down(&mut self,
                                     editor_scene: &EditorScene,
                                     camera_controller: &mut CameraController,
                                     current_selection: Handle<Node>,
                                     engine: &mut GameEngine,
    ) {
        let mouse_pos = engine.user_interface.cursor_position();

        // Pick gizmo nodes.
        let camera = camera_controller.camera;
        let camera_pivot = camera_controller.pivot;
        let editor_node = camera_controller.pick(
            mouse_pos,
            editor_scene,
            engine,
            true,
            |handle, _| handle != camera && handle != camera_pivot);

        if self.move_gizmo.handle_pick(editor_node, editor_scene, engine) {
            self.interacting = true;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.initial_position = graph[self.node].local_transform().position();
        } else {
            let new_selection = camera_controller.pick(mouse_pos, editor_scene, engine, false, |_, _| true);
            if new_selection != current_selection {
                self.message_sender
                    .send(Message::ExecuteCommand(Command::ChangeSelection(ChangeSelectionCommand::new(new_selection, current_selection))))
                    .unwrap();
            }
        }
    }

    fn on_left_mouse_button_up(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            self.interacting = false;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let current_position = graph[self.node].local_transform().position();
            if current_position != self.initial_position {
                // Commit changes.
                let move_command = MoveNodeCommand::new(self.node, self.initial_position, current_position);
                self.message_sender
                    .send(Message::ExecuteCommand(Command::MoveNode(move_command)))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(&mut self, mouse_offset: Vec2, camera: Handle<Node>, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.interacting {
            let node_offset = self.move_gizmo.calculate_offset(editor_scene, camera, mouse_offset, self.node, engine);
            engine.scenes[editor_scene.scene].graph[self.node].local_transform_mut().offset(node_offset);
        }
    }

    fn update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let transform = graph[self.node].local_transform().clone();
            self.move_gizmo.set_transform(graph, transform);
            self.move_gizmo.set_visible(graph, true);
        } else {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.move_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.node = Default::default();
        self.move_gizmo.set_visible(graph, false);
    }

    fn handle_message(&mut self, message: &Message) {
        if let &Message::SetSelection(selection) = message {
            self.node = selection;
        }
    }
}

pub enum ScaleGizmoMode {
    None,
    X,
    Y,
    Z,
    Uniform,
}

pub struct ScaleGizmo {
    mode: ScaleGizmoMode,
    origin: Handle<Node>,
    x_arrow: Handle<Node>,
    y_arrow: Handle<Node>,
    z_arrow: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_scale_axis(graph: &mut Graph, rotation: Quat, color: Color, name_prefix: &str) -> (Handle<Node>, Handle<Node>) {
    let axis = graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
        .with_name(name_prefix.to_owned() + "Axis")
        .with_depth_offset(0.5)
        .with_local_transform(TransformBuilder::new()
            .with_local_rotation(rotation)
            .build()))
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cylinder(10, 0.015, 1.0, true, Default::default()))))
            .with_color(color)
            .build()])
        .build()));
    let arrow = graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
        .with_name(name_prefix.to_owned() + "Arrow")
        .with_depth_offset(0.5)
        .with_local_transform(TransformBuilder::new()
            .with_local_position(Vec3::new(0.0, 1.0, 0.0))
            .build()))
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cube(Mat4::scale(Vec3::new(0.1, 0.1, 0.1))))))
            .with_color(color)
            .build()])
        .build()));
    graph.link_nodes(arrow, axis);
    (axis, arrow)
}

impl ScaleGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
            .with_name("Origin")
            .with_visibility(false))
            .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cube(Mat4::scale(Vec3::new(0.1, 0.1, 0.1))))))
                .build()])
            .build()));

        graph.link_nodes(origin, editor_scene.root);

        let (x_axis, x_arrow) = make_scale_axis(graph, Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()), Color::RED, "X");
        graph.link_nodes(x_axis, origin);
        let (y_axis, y_arrow) = make_scale_axis(graph, Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.0f32.to_radians()), Color::GREEN, "Y");
        graph.link_nodes(y_axis, origin);
        let (z_axis, z_arrow) = make_scale_axis(graph, Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()), Color::BLUE, "Z");
        graph.link_nodes(z_axis, origin);

        Self {
            mode: ScaleGizmoMode::None,
            origin,
            x_arrow,
            y_arrow,
            z_arrow,
            x_axis,
            y_axis,
            z_axis,
        }
    }

    pub fn set_mode(&mut self, mode: ScaleGizmoMode, graph: &mut Graph) {
        self.mode = mode;

        // Restore initial colors first.
        graph[self.origin].as_mesh_mut().set_color(Color::WHITE);
        graph[self.x_axis].as_mesh_mut().set_color(Color::RED);
        graph[self.x_arrow].as_mesh_mut().set_color(Color::RED);
        graph[self.y_axis].as_mesh_mut().set_color(Color::GREEN);
        graph[self.y_arrow].as_mesh_mut().set_color(Color::GREEN);
        graph[self.z_axis].as_mesh_mut().set_color(Color::BLUE);
        graph[self.z_arrow].as_mesh_mut().set_color(Color::BLUE);

        let yellow = Color::opaque(255, 255, 0);
        match self.mode {
            ScaleGizmoMode::None => (),
            ScaleGizmoMode::X => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
                graph[self.x_arrow].as_mesh_mut().set_color(yellow);
            }
            ScaleGizmoMode::Y => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
                graph[self.y_arrow].as_mesh_mut().set_color(yellow);
            }
            ScaleGizmoMode::Z => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
                graph[self.z_arrow].as_mesh_mut().set_color(yellow);
            }
            ScaleGizmoMode::Uniform => {
                graph[self.origin].as_mesh_mut().set_color(yellow);
            }
        }
    }

    pub fn handle_pick(&mut self,
                       picked: Handle<Node>,
                       editor_scene: &EditorScene,
                       engine: &mut GameEngine,
    ) -> bool {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        if picked == self.x_axis || picked == self.x_arrow {
            self.set_mode(ScaleGizmoMode::X, graph);
            true
        } else if picked == self.y_axis || picked == self.y_arrow {
            self.set_mode(ScaleGizmoMode::Y, graph);
            true
        } else if picked == self.z_axis || picked == self.z_arrow {
            self.set_mode(ScaleGizmoMode::Z, graph);
            true
        } else if picked == self.origin {
            self.set_mode(ScaleGizmoMode::Uniform, graph);
            true
        } else {
            self.set_mode(ScaleGizmoMode::None, graph);
            false
        }
    }

    pub fn calculate_scale_delta(&self,
                                 editor_scene: &EditorScene,
                                 camera: Handle<Node>,
                                 mouse_offset: Vec2,
                                 node: Handle<Node>,
                                 engine: &GameEngine,
    ) -> Vec3 {
        let graph = &engine.scenes[editor_scene.scene].graph;
        let mouse_position = engine.user_interface.cursor_position();
        let screen_size = engine.renderer.get_frame_size();
        let screen_size = Vec2::new(screen_size.0 as f32, screen_size.1 as f32);
        let node_global_transform = graph[node].global_transform();

        if let Node::Camera(camera) = &graph[camera] {
            let dlook = node_global_transform.position() - camera.global_position();
            let inv_node_transform = node_global_transform.inverse().unwrap_or_default();

            // Create two rays in object space.
            let initial_ray = camera.make_ray(mouse_position, screen_size).transform(inv_node_transform);
            let offset_ray = camera.make_ray(mouse_position + mouse_offset, screen_size).transform(inv_node_transform);

            // Select plane by current active mode.
            let plane = match self.mode {
                ScaleGizmoMode::None => return Vec3::ZERO,
                ScaleGizmoMode::X => Plane::from_normal_and_point(&Vec3::new(0.0, dlook.y, dlook.z), &Vec3::ZERO),
                ScaleGizmoMode::Y => Plane::from_normal_and_point(&Vec3::new(dlook.x, 0.0, dlook.z), &Vec3::ZERO),
                ScaleGizmoMode::Z => Plane::from_normal_and_point(&Vec3::new(dlook.x, dlook.y, 0.0), &Vec3::ZERO),
                ScaleGizmoMode::Uniform => Plane::from_normal_and_point(&dlook, &Vec3::ZERO),
            }.unwrap_or_default();

            // Get two intersection points with plane and use delta between them to calculate scale delta.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let delta = next_point - initial_point;
                    return match self.mode {
                        ScaleGizmoMode::None => unreachable!(),
                        ScaleGizmoMode::X => Vec3::new(delta.x, 0.0, 0.0),
                        ScaleGizmoMode::Y => Vec3::new(0.0, delta.y, 0.0),
                        ScaleGizmoMode::Z => Vec3::new(0.0, 0.0, delta.z),
                        ScaleGizmoMode::Uniform => {
                            // TODO: Still may behave weird.
                            let amount = delta.len() * (delta.y + delta.x + delta.z).signum();
                            Vec3::new(amount, amount, amount)
                        }
                    };
                }
            }
        }

        Vec3::ZERO
    }

    pub fn set_transform(&self, graph: &mut Graph, local_transform: Transform) {
        let gizmo_origin = &mut graph[self.origin];
        gizmo_origin.set_visibility(true);
        *gizmo_origin.local_transform_mut() = local_transform;
        // Drop scale because we need to have gizmo at same size.
        gizmo_origin.local_transform_mut().set_scale(Vec3::new(1.0, 1.0, 1.0));
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

pub struct ScaleInteractionMode {
    initial_scale: Vec3,
    scale_gizmo: ScaleGizmo,
    node: Handle<Node>,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl ScaleInteractionMode {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine, message_sender: Sender<Message>) -> Self {
        Self {
            initial_scale: Default::default(),
            scale_gizmo: ScaleGizmo::new(editor_scene, engine),
            node: Default::default(),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionModeTrait for ScaleInteractionMode {
    fn on_left_mouse_button_down(&mut self,
                                     editor_scene: &EditorScene,
                                     camera_controller: &mut CameraController,
                                     current_selection: Handle<Node>,
                                     engine: &mut GameEngine,
    ) {
        let mouse_pos = engine.user_interface.cursor_position();

        // Pick gizmo nodes.
        let camera = camera_controller.camera;
        let camera_pivot = camera_controller.pivot;
        let editor_node = camera_controller.pick(
            mouse_pos,
            editor_scene,
            engine,
            true,
            |handle, _| handle != camera && handle != camera_pivot);

        if self.scale_gizmo.handle_pick(editor_node, editor_scene, engine) {
            self.interacting = true;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.initial_scale = graph[self.node].local_transform().scale();
        } else {
            let new_selection = camera_controller.pick(mouse_pos, editor_scene, engine, false, |_, _| true);
            if new_selection != current_selection {
                self.message_sender
                    .send(Message::ExecuteCommand(Command::ChangeSelection(ChangeSelectionCommand::new(new_selection, current_selection))))
                    .unwrap();
            }
        }
    }

    fn on_left_mouse_button_up(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            self.interacting = false;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let current_scale = graph[self.node].local_transform().scale();
            if current_scale != self.initial_scale {
                // Commit changes.
                let scale_command = ScaleNodeCommand::new(self.node, self.initial_scale, current_scale);
                self.message_sender
                    .send(Message::ExecuteCommand(Command::ScaleNode(scale_command)))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(&mut self, mouse_offset: Vec2, camera: Handle<Node>, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.interacting {
            let scale_delta = self.scale_gizmo.calculate_scale_delta(editor_scene, camera, mouse_offset, self.node, engine);
            let transform = engine.scenes[editor_scene.scene].graph[self.node].local_transform_mut();
            let scale = transform.scale();
            transform.set_scale(scale + scale_delta);
        }
    }

    fn update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let transform = graph[self.node].local_transform().clone();
            self.scale_gizmo.set_transform(graph, transform);
            self.scale_gizmo.set_visible(graph, true);
        } else {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.scale_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.node = Default::default();
        self.scale_gizmo.set_visible(graph, false);
    }

    fn handle_message(&mut self, message: &Message) {
        if let &Message::SetSelection(selection) = message {
            self.node = selection;
        }
    }
}

pub enum RotateGizmoMode {
    None,
    Pitch,
    Yaw,
    Roll,
    Arbitrary,
}

pub struct RotationGizmo {
    mode: RotateGizmoMode,
    origin: Handle<Node>,
    x_axis: Handle<Node>,
    y_axis: Handle<Node>,
    z_axis: Handle<Node>,
}

fn make_rotation_ribbon(graph: &mut Graph, rotation: Quat, color: Color, name: &str) -> Handle<Node> {
    graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
        .with_name(name)
        .with_depth_offset(0.5)
        .with_local_transform(TransformBuilder::new()
            .with_local_rotation(rotation)
            .build()))
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cylinder(30, 0.5, 0.05, false, Mat4::translate(Vec3::new(0.0, -0.05, 0.0))))))
            .with_color(color)
            .build()])
        .build()))
}

impl RotationGizmo {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;

        let origin = graph.add_node(Node::Mesh(MeshBuilder::new(BaseBuilder::new()
            .with_name("Origin")
            .with_visibility(false))
            .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(SurfaceSharedData::make_cube(Mat4::scale(Vec3::new(0.1, 0.1, 0.1))))))
                .build()])
            .build()));

        graph.link_nodes(origin, editor_scene.root);

        let x_axis = make_rotation_ribbon(graph, Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 90.0f32.to_radians()), Color::RED, "X");
        graph.link_nodes(x_axis, origin);
        let y_axis = make_rotation_ribbon(graph, Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.0f32.to_radians()), Color::GREEN, "Y");
        graph.link_nodes(y_axis, origin);
        let z_axis = make_rotation_ribbon(graph, Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 90.0f32.to_radians()), Color::BLUE, "Z");
        graph.link_nodes(z_axis, origin);

        Self {
            mode: RotateGizmoMode::None,
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
            RotateGizmoMode::None => (),
            RotateGizmoMode::Pitch => {
                graph[self.x_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Yaw => {
                graph[self.y_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Roll => {
                graph[self.z_axis].as_mesh_mut().set_color(yellow);
            }
            RotateGizmoMode::Arbitrary => {
                graph[self.origin].as_mesh_mut().set_color(yellow);
            }
        }
    }

    pub fn handle_pick(&mut self,
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
        } else if picked == self.origin {
            self.set_mode(RotateGizmoMode::Arbitrary, graph);
            true
        } else {
            self.set_mode(RotateGizmoMode::None, graph);
            false
        }
    }

    pub fn calculate_rotation_delta(&self,
                                    editor_scene: &EditorScene,
                                    camera: Handle<Node>,
                                    mouse_offset: Vec2,
                                    node: Handle<Node>,
                                    engine: &GameEngine,
    ) -> Quat {
        let graph = &engine.scenes[editor_scene.scene].graph;
        let mouse_position = engine.user_interface.cursor_position();
        let screen_size = engine.renderer.get_frame_size();
        let screen_size = Vec2::new(screen_size.0 as f32, screen_size.1 as f32);

        if let Node::Camera(camera) = &graph[camera] {
            let node_global_transform = graph[node].global_transform();

            // Create two rays in object space.
            let initial_ray = camera.make_ray(mouse_position, screen_size);
            let offset_ray = camera.make_ray(mouse_position + mouse_offset, screen_size);

            let axis = match self.mode {
                RotateGizmoMode::None => return Default::default(),
                RotateGizmoMode::Pitch => Vec3::RIGHT,
                RotateGizmoMode::Yaw => Vec3::UP,
                RotateGizmoMode::Roll => Vec3::LOOK,
                RotateGizmoMode::Arbitrary => Vec3::ZERO,
            };

            let plane = Plane::from_normal_and_point(&node_global_transform.transform_vector(axis), &Vec3::ZERO).unwrap_or_default();

            // Get two intersection points with plane and use delta between them to calculate scale delta.
            // TODO: Still bugged and sometimes make unpredictable results.
            if let Some(initial_point) = initial_ray.plane_intersection_point(&plane) {
                if let Some(next_point) = offset_ray.plane_intersection_point(&plane) {
                    let v_prev = initial_point.normalized().unwrap_or_default();
                    let v_new = next_point.normalized().unwrap_or_default();
                    let sign = v_prev.cross(&v_new).dot(&plane.normal).signum();
                    let angle = v_prev.dot(&v_new).max(-0.999).min(0.999).acos();
                    return Quat::from_axis_angle(axis, sign * angle);
                }
            }
        }

        Quat::default()
    }

    pub fn set_transform(&self, graph: &mut Graph, local_transform: Transform) {
        let gizmo_origin = &mut graph[self.origin];
        gizmo_origin.set_visibility(true);
        *gizmo_origin.local_transform_mut() = local_transform;
        // Drop scale because we need to have gizmo at same size.
        gizmo_origin.local_transform_mut().set_scale(Vec3::new(1.0, 1.0, 1.0));
    }

    pub fn set_visible(&self, graph: &mut Graph, visible: bool) {
        graph[self.origin].set_visibility(visible);
    }
}

pub struct RotateInteractionMode {
    initial_rotation: Quat,
    rotation_gizmo: RotationGizmo,
    node: Handle<Node>,
    interacting: bool,
    message_sender: Sender<Message>,
}

impl RotateInteractionMode {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine, message_sender: Sender<Message>) -> Self {
        Self {
            initial_rotation: Default::default(),
            rotation_gizmo: RotationGizmo::new(editor_scene, engine),
            node: Default::default(),
            interacting: false,
            message_sender,
        }
    }
}

impl InteractionModeTrait for RotateInteractionMode {
    fn on_left_mouse_button_down(&mut self,
                                     editor_scene: &EditorScene,
                                     camera_controller: &mut CameraController,
                                     current_selection: Handle<Node>,
                                     engine: &mut GameEngine,
    ) {
        let mouse_pos = engine.user_interface.cursor_position();

        // Pick gizmo nodes.
        let camera = camera_controller.camera;
        let camera_pivot = camera_controller.pivot;
        let editor_node = camera_controller.pick(
            mouse_pos,
            editor_scene,
            engine,
            true,
            |handle, _| handle != camera && handle != camera_pivot);

        if self.rotation_gizmo.handle_pick(editor_node, editor_scene, engine) {
            self.interacting = true;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.initial_rotation = graph[self.node].local_transform().rotation();
        } else {
            let new_selection = camera_controller.pick(mouse_pos, editor_scene, engine, false, |_, _| true);
            if new_selection != current_selection {
                self.message_sender
                    .send(Message::ExecuteCommand(Command::ChangeSelection(ChangeSelectionCommand::new(new_selection, current_selection))))
                    .unwrap();
            }
        }
    }

    fn on_left_mouse_button_up(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            self.interacting = false;
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let current_rotation = graph[self.node].local_transform().rotation();
            if current_rotation != self.initial_rotation {
                // Commit changes.
                let rotate_command = RotateNodeCommand::new(self.node, self.initial_rotation, current_rotation);
                self.message_sender
                    .send(Message::ExecuteCommand(Command::RotateNode(rotate_command)))
                    .unwrap();
            }
        }
    }

    fn on_mouse_move(&mut self, mouse_offset: Vec2, camera: Handle<Node>, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.interacting {
            let rotation_delta = self.rotation_gizmo.calculate_rotation_delta(editor_scene, camera, mouse_offset, self.node, engine);
            let transform = engine.scenes[editor_scene.scene].graph[self.node].local_transform_mut();
            let rotation = transform.rotation();
            transform.set_rotation(rotation * rotation_delta);
        }
    }

    fn update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            let transform = graph[self.node].local_transform().clone();
            self.rotation_gizmo.set_transform(graph, transform);
            self.rotation_gizmo.set_visible(graph, true);
        } else {
            let graph = &mut engine.scenes[editor_scene.scene].graph;
            self.rotation_gizmo.set_visible(graph, false);
        }
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let graph = &mut engine.scenes[editor_scene.scene].graph;
        self.node = Default::default();
        self.rotation_gizmo.set_visible(graph, false);
    }

    fn handle_message(&mut self, message: &Message) {
        if let &Message::SetSelection(selection) = message {
            self.node = selection;
        }
    }
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            InteractionMode::Move(v) => v.$func($($args),*),
            InteractionMode::Scale(v) => v.$func($($args),*),
            InteractionMode::Rotate(v) => v.$func($($args),*),
        }
    };
}

pub enum InteractionMode {
    Move(MoveInteractionMode),
    Scale(ScaleInteractionMode),
    Rotate(RotateInteractionMode),
}

impl InteractionModeTrait for InteractionMode {
    fn on_left_mouse_button_down(&mut self, editor_scene: &EditorScene, camera_controller: &mut CameraController, current_selection: Handle<Node>, engine: &mut GameEngine) {
        static_dispatch!(self, on_left_mouse_button_down, editor_scene, camera_controller, current_selection, engine)
    }

    fn on_left_mouse_button_up(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        static_dispatch!(self, on_left_mouse_button_up, editor_scene, engine)
    }

    fn on_mouse_move(&mut self, mouse_offset: Vec2, camera: Handle<Node>, editor_scene: &EditorScene, engine: &mut GameEngine) {
        static_dispatch!(self, on_mouse_move, mouse_offset, camera, editor_scene, engine)
    }

    fn update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        static_dispatch!(self, update, editor_scene, engine)
    }

    fn deactivate(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        static_dispatch!(self, deactivate, editor_scene, engine)
    }

    fn handle_message(&mut self, message: &Message) {
        static_dispatch!(self, handle_message, message)
    }
}