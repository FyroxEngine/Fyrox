use crate::utils::built_in_skybox;
use fyrox::{
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
        math::{plane::Plane, ray::Ray, Matrix4Ext, TriangleDefinition, Vector3Ext},
        pool::Handle,
    },
    gui::message::{KeyCode, MouseButton},
    scene::{
        base::BaseBuilder,
        camera::{Camera, CameraBuilder, Exposure, Projection},
        graph::Graph,
        mesh::{
            buffer::{VertexAttributeUsage, VertexReadTrait},
            surface::SurfaceData,
            Mesh,
        },
        node::Node,
        pivot::PivotBuilder,
        sound::listener::ListenerBuilder,
        transform::TransformBuilder,
    },
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

const DEFAULT_Z_OFFSET: f32 = -3.0;

pub struct CameraController {
    pub pivot: Handle<Node>,
    pub camera: Handle<Node>,
    yaw: f32,
    pitch: f32,
    rotate: bool,
    drag_side: f32,
    drag_up: f32,
    drag: bool,
    move_left: bool,
    move_right: bool,
    move_forward: bool,
    move_backward: bool,
    move_up: bool,
    move_down: bool,
    speed_factor: f32,
    stack: Vec<Handle<Node>>,
    editor_context: PickContext,
    scene_context: PickContext,
}

#[derive(Clone)]
pub struct CameraPickResult {
    pub position: Vector3<f32>,
    pub node: Handle<Node>,
    pub toi: f32,
}

#[derive(Default)]
struct PickContext {
    pick_list: Vec<CameraPickResult>,
    pick_index: usize,
    old_selection_hash: u64,
    old_cursor_pos: Vector2<f32>,
}

pub struct PickingOptions<'a, F>
where
    F: FnMut(Handle<Node>, &Node) -> bool,
{
    pub cursor_pos: Vector2<f32>,
    pub graph: &'a Graph,
    pub editor_objects_root: Handle<Node>,
    pub screen_size: Vector2<f32>,
    pub editor_only: bool,
    pub filter: F,
    pub ignore_back_faces: bool,
}

impl CameraController {
    pub fn new(graph: &mut Graph, root: Handle<Node>) -> Self {
        let camera;
        let pivot = PivotBuilder::new(
            BaseBuilder::new()
                .with_children(&[{
                    camera = CameraBuilder::new(
                        BaseBuilder::new()
                            .with_children(&[ListenerBuilder::new(BaseBuilder::new()).build(graph)])
                            .with_name("EditorCamera"),
                    )
                    .with_exposure(Exposure::Manual(std::f32::consts::E))
                    .with_skybox(built_in_skybox())
                    .with_z_far(512.0)
                    .build(graph);
                    camera
                }])
                .with_name("EditorCameraPivot")
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(0.0, 1.0, DEFAULT_Z_OFFSET))
                        .build(),
                ),
        )
        .build(graph);

        graph.link_nodes(pivot, root);

        Self {
            pivot,
            camera,
            yaw: 0.0,
            pitch: 0.0,
            rotate: false,
            drag_side: 0.0,
            drag_up: 0.0,
            drag: false,
            move_left: false,
            move_right: false,
            move_forward: false,
            move_backward: false,
            move_up: false,
            move_down: false,
            speed_factor: 1.0,
            stack: Default::default(),
            editor_context: Default::default(),
            scene_context: Default::default(),
        }
    }

    pub fn set_projection(&self, graph: &mut Graph, projection: Projection) {
        graph[self.camera]
            .as_camera_mut()
            .set_projection(projection);
    }

    pub fn on_mouse_move(&mut self, delta: Vector2<f32>) {
        if self.rotate {
            self.yaw -= delta.x as f32 * 0.01;
            self.pitch += delta.y as f32 * 0.01;
            if self.pitch > 90.0f32.to_radians() {
                self.pitch = 90.0f32.to_radians();
            }
            if self.pitch < -90.0f32.to_radians() {
                self.pitch = -90.0f32.to_radians();
            }
        }

        if self.drag {
            self.drag_side -= delta.x * 0.01;
            self.drag_up -= delta.y * 0.01;
        }
    }

    pub fn on_mouse_wheel(&mut self, delta: f32, graph: &mut Graph) {
        let camera = graph[self.camera].as_camera_mut();

        match *camera.projection_mut() {
            Projection::Perspective(_) => {
                let look = camera.global_transform().look();
                graph[self.pivot]
                    .local_transform_mut()
                    .offset(look.scale(delta));
            }
            Projection::Orthographic(ref mut ortho) => {
                ortho.vertical_size = (ortho.vertical_size - delta).max(f32::EPSILON);
            }
        }
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton) {
        match button {
            MouseButton::Right => {
                self.rotate = false;
            }
            MouseButton::Middle => {
                self.drag = false;
            }
            _ => (),
        }
    }

    pub fn on_mouse_button_down(&mut self, button: MouseButton) {
        match button {
            MouseButton::Right => {
                self.rotate = true;
            }
            MouseButton::Middle => {
                self.drag = true;
            }
            _ => (),
        }
    }

    #[must_use]
    pub fn on_key_up(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::W => {
                self.move_forward = false;
                true
            }
            KeyCode::S => {
                self.move_backward = false;
                true
            }
            KeyCode::A => {
                self.move_left = false;
                true
            }
            KeyCode::D => {
                self.move_right = false;
                true
            }
            KeyCode::Space | KeyCode::Q => {
                self.move_up = false;
                true
            }
            KeyCode::E => {
                self.move_down = false;
                true
            }
            KeyCode::LControl | KeyCode::LShift => {
                self.speed_factor = 1.0;
                true
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn on_key_down(&mut self, key: KeyCode) -> bool {
        if !self.rotate || self.drag {
            return false;
        }

        match key {
            KeyCode::W => {
                self.move_forward = true;
                true
            }
            KeyCode::S => {
                self.move_backward = true;
                true
            }
            KeyCode::A => {
                self.move_left = true;
                true
            }
            KeyCode::D => {
                self.move_right = true;
                true
            }
            KeyCode::Space | KeyCode::Q => {
                self.move_up = true;
                true
            }
            KeyCode::E => {
                self.move_down = true;
                true
            }
            KeyCode::LControl => {
                self.speed_factor = 2.0;
                true
            }
            KeyCode::LShift => {
                self.speed_factor = 0.25;
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, graph: &mut Graph, dt: f32) {
        let camera = graph[self.camera].as_camera_mut();

        match camera.projection_value() {
            Projection::Perspective(_) => {
                let global_transform = camera.global_transform();
                let look = global_transform.look();
                let side = global_transform.side();
                let up = global_transform.up();

                let mut move_vec = Vector3::default();

                if self.rotate {
                    if self.move_forward {
                        move_vec += look;
                    }
                    if self.move_backward {
                        move_vec -= look;
                    }
                    if self.move_left {
                        move_vec += side;
                    }
                    if self.move_right {
                        move_vec -= side;
                    }
                    if self.move_up {
                        move_vec += up;
                    }
                    if self.move_down {
                        move_vec -= up;
                    }
                }

                move_vec += side * self.drag_side;
                move_vec.y += self.drag_up;

                if let Some(v) = move_vec.try_normalize(std::f32::EPSILON) {
                    move_vec = v.scale(self.speed_factor * 10.0 * dt);
                }

                camera
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::x_axis(),
                        self.pitch,
                    ));

                graph[self.pivot]
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(
                        &Vector3::y_axis(),
                        self.yaw,
                    ))
                    .offset(move_vec);
            }
            Projection::Orthographic(_) => {
                let mut move_vec = Vector2::<f32>::default();

                if self.rotate {
                    if self.move_left {
                        move_vec.x += 1.0;
                    }
                    if self.move_right {
                        move_vec.x -= 1.0;
                    }
                    if self.move_forward {
                        move_vec.y += 1.0;
                    }
                    if self.move_backward {
                        move_vec.y -= 1.0;
                    }
                }

                move_vec.x += self.drag_side;
                move_vec.y += self.drag_up;

                if let Some(v) = move_vec.try_normalize(f32::EPSILON) {
                    move_vec = v.scale(self.speed_factor * 10.0 * dt);
                }

                camera
                    .local_transform_mut()
                    .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 0.0));

                let local_transform = graph[self.pivot].local_transform_mut();

                let mut new_position = **local_transform.position();
                new_position.z = DEFAULT_Z_OFFSET;
                new_position.x += move_vec.x;
                new_position.y += move_vec.y;

                local_transform
                    .set_rotation(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), 0.0))
                    .set_position(new_position);
            }
        }

        self.drag_side = 0.0;
        self.drag_up = 0.0;
    }

    pub fn pick<F>(&mut self, options: PickingOptions<'_, F>) -> Option<CameraPickResult>
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let PickingOptions {
            cursor_pos,
            graph,
            editor_objects_root,
            screen_size,
            editor_only,
            mut filter,
            ignore_back_faces,
        } = options;

        if let Some(camera) = graph[self.camera].cast::<Camera>() {
            let ray = camera.make_ray(cursor_pos, screen_size);

            self.stack.clear();
            let context = if editor_only {
                // In case if we want to pick stuff from editor scene only, we have to
                // start traversing graph from editor root.
                self.stack.push(editor_objects_root);
                &mut self.editor_context
            } else {
                self.stack.push(graph.get_root());
                &mut self.scene_context
            };

            context.pick_list.clear();

            while let Some(handle) = self.stack.pop() {
                // Ignore editor nodes if we picking scene stuff only.
                if !editor_only && handle == editor_objects_root {
                    continue;
                }

                let node = &graph[handle];

                self.stack.extend_from_slice(node.children());

                if !node.global_visibility() || !filter(handle, node) {
                    continue;
                }

                if handle != graph.get_root() {
                    let object_space_ray =
                        ray.transform(node.global_transform().try_inverse().unwrap_or_default());

                    let aabb = node.local_bounding_box();
                    // Do coarse, but fast, intersection test with bounding box first.
                    if let Some(points) = object_space_ray.aabb_intersection_points(&aabb) {
                        if has_hull(node) {
                            if let Some((closest_distance, position)) =
                                precise_ray_test(node, &ray, ignore_back_faces)
                            {
                                context.pick_list.push(CameraPickResult {
                                    position,
                                    node: handle,
                                    toi: closest_distance,
                                });
                            }
                        } else {
                            // Hull-less objects (light sources, cameras, etc.) can still be selected
                            // by coarse intersection test results.
                            let da = points[0].metric_distance(&object_space_ray.origin);
                            let db = points[1].metric_distance(&object_space_ray.origin);
                            let closest_distance = da.min(db);
                            context.pick_list.push(CameraPickResult {
                                position: transform_vertex(
                                    if da < db { points[0] } else { points[1] },
                                    &node.global_transform(),
                                ),
                                node: handle,
                                toi: closest_distance,
                            });
                        }
                    }
                }
            }

            // Make sure closest will be selected first.
            context
                .pick_list
                .sort_by(|a, b| a.toi.partial_cmp(&b.toi).unwrap());

            let mut hasher = DefaultHasher::new();
            for result in context.pick_list.iter() {
                result.node.hash(&mut hasher);
            }
            let selection_hash = hasher.finish();
            if selection_hash == context.old_selection_hash && cursor_pos == context.old_cursor_pos
            {
                context.pick_index += 1;

                // Wrap picking loop.
                if context.pick_index >= context.pick_list.len() {
                    context.pick_index = 0;
                }
            } else {
                // Select is different, start from beginning.
                context.pick_index = 0;
            }
            context.old_selection_hash = selection_hash;
            context.old_cursor_pos = cursor_pos;

            if !context.pick_list.is_empty() {
                if let Some(result) = context.pick_list.get(context.pick_index) {
                    return Some(result.clone());
                }
            }
        }

        None
    }

    pub fn pick_on_plane(
        &self,
        plane: Plane,
        graph: &Graph,
        mouse_position: Vector2<f32>,
        viewport_size: Vector2<f32>,
        transform: Matrix4<f32>,
    ) -> Option<Vector3<f32>> {
        graph[self.camera]
            .as_camera()
            .make_ray(mouse_position, viewport_size)
            .transform(transform)
            .plane_intersection_point(&plane)
    }
}

fn read_vertex_position(data: &SurfaceData, i: u32) -> Option<Vector3<f32>> {
    data.vertex_buffer
        .get(i as usize)
        .and_then(|v| v.read_3_f32(VertexAttributeUsage::Position).ok())
}

fn transform_vertex(vertex: Vector3<f32>, transform: &Matrix4<f32>) -> Vector3<f32> {
    transform.transform_point(&Point3::from(vertex)).coords
}

fn read_triangle(
    data: &SurfaceData,
    triangle: &TriangleDefinition,
    transform: &Matrix4<f32>,
) -> Option<[Vector3<f32>; 3]> {
    let a = transform_vertex(read_vertex_position(data, triangle[0])?, transform);
    let b = transform_vertex(read_vertex_position(data, triangle[1])?, transform);
    let c = transform_vertex(read_vertex_position(data, triangle[2])?, transform);
    Some([a, b, c])
}

fn has_hull(node: &Node) -> bool {
    node.query_component_ref::<Mesh>().is_some()
}

fn precise_ray_test(
    node: &Node,
    ray: &Ray,
    ignore_back_faces: bool,
) -> Option<(f32, Vector3<f32>)> {
    let mut closest_distance = f32::MAX;
    let mut closest_point = None;

    if let Some(mesh) = node.query_component_ref::<Mesh>() {
        let transform = mesh.global_transform();

        for surface in mesh.surfaces().iter() {
            let data = surface.data();
            let data = data.lock();

            for triangle in data
                .geometry_buffer
                .iter()
                .filter_map(|t| read_triangle(&data, t, &transform))
            {
                if ignore_back_faces {
                    // If normal of the triangle is facing in the same direction as ray's direction,
                    // then we skip such triangle.
                    let normal = (triangle[1] - triangle[0]).cross(&(triangle[2] - triangle[0]));
                    if normal.dot(&ray.dir) >= 0.0 {
                        continue;
                    }
                }

                if let Some(pt) = ray.triangle_intersection_point(&triangle) {
                    let distance = ray.origin.sqr_distance(&pt);

                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_point = Some(pt);
                    }
                }
            }
        }
    }

    closest_point.map(|pt| (closest_distance, pt))
}
