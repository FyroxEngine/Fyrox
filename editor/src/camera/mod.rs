use crate::{
    fyrox::{
        core::{
            algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
            math::{
                aabb::AxisAlignedBoundingBox, plane::Plane, ray::Ray, Matrix4Ext,
                TriangleDefinition, Vector3Ext,
            },
            pool::Handle,
        },
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::message::{KeyCode, KeyboardModifiers, MouseButton},
        scene::{
            base::BaseBuilder,
            camera::{Camera, CameraBuilder, Exposure, FitParameters, Projection},
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
            Scene,
        },
    },
    settings::{
        camera::CameraSettings,
        keys::KeyBindings,
        scene::{SceneCameraSettings, SceneSettings},
        Settings,
    },
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
};

pub mod panel;

pub const DEFAULT_Z_OFFSET: f32 = -3.0;

#[derive(PartialEq, Copy, Clone)]
enum RotationMode {
    None,
    Center { prev_z_offset: f32 },
    Orbital,
}

pub struct CameraController {
    pub pivot: Handle<Node>,
    pub camera_hinge: Handle<Node>,
    pub camera: Handle<Node>,
    pub yaw: f32,
    pub pitch: f32,
    pub z_offset: f32,
    rotate: RotationMode,
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
    prev_interaction_state: bool,
    pub grid: Handle<Node>,
    pub editor_objects_root: Handle<Node>,
    pub scene_content_root: Handle<Node>,
    pub screen_size: Vector2<f32>,
}

#[derive(Clone, Debug)]
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

pub type PickingFilter<'a> = Option<&'a mut dyn FnMut(Handle<Node>, &Node) -> bool>;

#[derive(Default)]
pub struct PickingOptions<'a> {
    pub cursor_pos: Vector2<f32>,
    pub editor_only: bool,
    pub filter: PickingFilter<'a>,
    pub ignore_back_faces: bool,
    pub use_picking_loop: bool,
    pub only_meshes: bool,
}

impl CameraController {
    pub fn new(
        graph: &mut Graph,
        root: Handle<Node>,
        settings: Option<&SceneCameraSettings>,
        grid: Handle<Node>,
        editor_objects_root: Handle<Node>,
        scene_content_root: Handle<Node>,
    ) -> Self {
        let settings = settings.cloned().unwrap_or_default();

        let camera;
        let camera_hinge;
        let pivot = PivotBuilder::new(
            BaseBuilder::new()
                .with_children(&[{
                    camera_hinge = PivotBuilder::new(BaseBuilder::new().with_children(&[{
                        camera = CameraBuilder::new(
                            BaseBuilder::new()
                                .with_children(&[
                                    ListenerBuilder::new(BaseBuilder::new()).build(graph)
                                ])
                                .with_name("EditorCamera"),
                        )
                        .with_projection(settings.projection)
                        .with_exposure(Exposure::Manual(std::f32::consts::E))
                        .with_z_far(512.0)
                        .build(graph);
                        camera
                    }]))
                    .build(graph);
                    camera_hinge
                }])
                .with_name("EditorCameraPivot")
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(settings.position)
                        .build(),
                ),
        )
        .build(graph);

        graph.link_nodes(pivot, root);

        Self {
            pivot,
            camera_hinge,
            camera,
            yaw: settings.yaw,
            pitch: settings.pitch,
            rotate: RotationMode::None,
            drag_side: 0.0,
            drag_up: 0.0,
            z_offset: DEFAULT_Z_OFFSET,
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
            prev_interaction_state: false,
            grid,
            editor_objects_root,
            scene_content_root,
            screen_size: Default::default(),
        }
    }

    pub fn is_interacting(&self) -> bool {
        self.move_backward
            || self.move_forward
            || self.move_left
            || self.move_right
            || self.drag
            || self.rotate != RotationMode::None
            || self.move_down
            || self.move_up
    }

    pub fn placement_position(&self, graph: &Graph, relative_to: Handle<Node>) -> Vector3<f32> {
        let camera = &graph[self.camera];
        let world_space_position = camera.global_position()
            + camera
                .look_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_default()
                .scale(5.0);
        if let Some(relative_to) = graph.try_get(relative_to) {
            return relative_to
                .global_transform()
                .try_inverse()
                .unwrap_or_default()
                .transform_point(&world_space_position.into())
                .coords;
        }
        world_space_position
    }

    pub fn fit_object(&mut self, scene: &mut Scene, handle: Handle<Node>) {
        // Combine AABBs from the descendants.
        let mut aabb = AxisAlignedBoundingBox::default();
        for descendant in scene.graph.traverse_iter(handle) {
            let descendant_aabb = descendant.local_bounding_box();
            if !descendant_aabb.is_invalid_or_degenerate() {
                aabb.add_box(descendant_aabb.transform(&descendant.global_transform()))
            }
        }

        if aabb.is_invalid_or_degenerate() {
            // To prevent the camera from flying away into abyss.
            aabb = AxisAlignedBoundingBox::from_point(scene.graph[handle].global_position());
        }

        let fit_parameters = scene.graph[self.camera].as_camera().fit(
            &aabb,
            scene
                .rendering_options
                .render_target
                .as_ref()
                .and_then(|rt| rt.data_ref().kind().rectangle_size())
                .map(|rs| rs.x as f32 / rs.y as f32)
                .unwrap_or(1.0),
        );

        match fit_parameters {
            FitParameters::Perspective { distance, .. } => {
                scene.graph[self.pivot]
                    .local_transform_mut()
                    .set_position(aabb.center());
                self.z_offset = -distance;
            }
            FitParameters::Orthographic {
                position,
                vertical_size,
            } => {
                if let Projection::Orthographic(ortho) =
                    scene.graph[self.camera].as_camera_mut().projection_mut()
                {
                    ortho.vertical_size = vertical_size;
                }
                scene.graph[self.pivot]
                    .local_transform_mut()
                    .set_position(position);
                self.z_offset = 0.0;
            }
        }
    }

    pub fn set_projection(&self, graph: &mut Graph, projection: Projection) {
        graph[self.camera]
            .as_camera_mut()
            .set_projection(projection);
    }

    pub fn on_mouse_move(&mut self, delta: Vector2<f32>, settings: &CameraSettings) {
        if self.rotate != RotationMode::None {
            self.yaw -= delta.x * 0.01;
            self.pitch += delta.y * 0.01;
            if self.pitch > 90.0f32.to_radians() {
                self.pitch = 90.0f32.to_radians();
            }
            if self.pitch < -90.0f32.to_radians() {
                self.pitch = -90.0f32.to_radians();
            }
        }

        if self.drag {
            let sign = if settings.invert_dragging { 1.0 } else { -1.0 };
            self.drag_side += sign * delta.x * settings.drag_speed;
            self.drag_up += sign * delta.y * settings.drag_speed;
        }
    }

    pub fn on_mouse_wheel(&mut self, delta: f32, graph: &mut Graph, settings: &Settings) {
        let camera = graph[self.camera].as_camera_mut();

        match *camera.projection_mut() {
            Projection::Perspective(_) => {
                self.z_offset = (self.z_offset + delta).clamp(
                    -settings.camera.zoom_range.end,
                    -settings.camera.zoom_range.start,
                );
            }
            Projection::Orthographic(ref mut ortho) => {
                ortho.vertical_size = (ortho.vertical_size - delta).max(f32::EPSILON);
            }
        }
    }

    fn on_interaction_ended(
        &self,
        settings: &mut Settings,
        scene_path: Option<&Path>,
        graph: &Graph,
    ) {
        if let Some(path) = scene_path {
            // Save camera current camera settings for current scene to be able to load them
            // on next launch.
            let last_settings = SceneCameraSettings {
                position: self.position(graph),
                yaw: self.yaw,
                pitch: self.pitch,
                projection: graph[self.camera].as_camera().projection().clone(),
            };

            if let Some(scene_settings) = settings.scene_settings.get(path) {
                if scene_settings.camera_settings != last_settings {
                    settings
                        .scene_settings
                        .get_mut(path)
                        .unwrap()
                        .camera_settings = last_settings;
                }
            } else {
                settings.scene_settings.insert(
                    path.to_owned(),
                    SceneSettings {
                        camera_settings: last_settings,
                        ..Default::default()
                    },
                );
            };
        }
    }

    fn move_along_look_vector(&self, amount: f32, graph: &mut Graph) {
        let look = graph[self.camera].look_vector();
        let pivot = &mut graph[self.pivot];
        let current = pivot.global_position();
        pivot.local_transform_mut().set_position(
            current
                + look
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default()
                    .scale(amount),
        );
    }

    pub fn on_mouse_button_up(&mut self, button: MouseButton, graph: &mut Graph) {
        match button {
            MouseButton::Right => {
                if let RotationMode::Center { prev_z_offset } = self.rotate {
                    self.z_offset = prev_z_offset;
                    self.move_along_look_vector(-self.z_offset, graph);

                    self.rotate = RotationMode::None;
                } else {
                    self.drag = false;
                }
            }
            MouseButton::Middle => {
                self.rotate = RotationMode::None;
            }
            _ => (),
        }
    }

    pub fn on_mouse_button_down(
        &mut self,
        button: MouseButton,
        modifiers: KeyboardModifiers,
        graph: &mut Graph,
    ) {
        match button {
            MouseButton::Right => {
                if modifiers.shift {
                    self.drag = true;
                } else {
                    self.rotate = RotationMode::Center {
                        prev_z_offset: self.z_offset,
                    };
                    self.move_along_look_vector(self.z_offset, graph);
                    self.z_offset = 0.0;
                }
            }
            MouseButton::Middle => {
                self.rotate = RotationMode::Orbital;
            }
            _ => (),
        }
    }

    #[must_use]
    pub fn on_key_up(&mut self, key_bindings: &KeyBindings, key: KeyCode) -> bool {
        if key_bindings.move_forward == key {
            self.move_forward = false;
            true
        } else if key_bindings.move_back == key {
            self.move_backward = false;
            true
        } else if key_bindings.move_left == key {
            self.move_left = false;
            true
        } else if key_bindings.move_right == key {
            self.move_right = false;
            true
        } else if key_bindings.move_up == key {
            self.move_up = false;
            true
        } else if key_bindings.move_down == key {
            self.move_down = false;
            true
        } else if key_bindings.slow_down == key || key_bindings.speed_up == key {
            self.speed_factor = 1.0;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn on_key_down(&mut self, key_bindings: &KeyBindings, key: KeyCode) -> bool {
        if self.rotate == RotationMode::None || self.drag {
            return false;
        }

        if key_bindings.move_forward == key {
            self.move_forward = true;
            true
        } else if key_bindings.move_back == key {
            self.move_backward = true;
            true
        } else if key_bindings.move_left == key {
            self.move_left = true;
            true
        } else if key_bindings.move_right == key {
            self.move_right = true;
            true
        } else if key_bindings.move_up == key {
            self.move_up = true;
            true
        } else if key_bindings.move_down == key {
            self.move_down = true;
            true
        } else if key_bindings.speed_up == key {
            self.speed_factor = 2.0;
            true
        } else if key_bindings.slow_down == key {
            self.speed_factor = 0.25;
            true
        } else {
            false
        }
    }

    pub fn position(&self, graph: &Graph) -> Vector3<f32> {
        graph[self.pivot].global_position()
    }

    pub fn update(
        &mut self,
        graph: &mut Graph,
        settings: &mut Settings,
        scene_path: Option<&Path>,
        editor_objects_root: Handle<Node>,
        scene_content_root: Handle<Node>,
        screen_size: Vector2<f32>,
        dt: f32,
    ) {
        // These fields can be overwritten by commands, so we must keep them in sync with the actual
        // values.
        self.editor_objects_root = editor_objects_root;
        self.scene_content_root = scene_content_root;
        // Keep screen size in-sync.
        self.screen_size = screen_size;

        let camera = graph[self.camera].as_camera_mut();

        match camera.projection_value() {
            Projection::Perspective(_) => {
                let global_transform = camera.global_transform();
                let look = global_transform.look();
                let side = global_transform.side();
                let up = global_transform.up();

                let mut move_vec = Vector3::default();

                if self.rotate != RotationMode::None {
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

                if let Some(v) = move_vec.try_normalize(f32::EPSILON) {
                    move_vec = v.scale(self.speed_factor * settings.camera.speed * dt);
                }

                move_vec += side * self.drag_side;
                move_vec.y += self.drag_up;

                camera
                    .local_transform_mut()
                    .set_position(Vector3::new(0.0, 0.0, self.z_offset));

                graph[self.camera_hinge].local_transform_mut().set_rotation(
                    UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch),
                );

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

                if self.rotate != RotationMode::None {
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
                    move_vec = v.scale(self.speed_factor * settings.camera.speed * dt);
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

        if !self.is_interacting() && self.prev_interaction_state {
            self.on_interaction_ended(settings, scene_path, graph);
            self.prev_interaction_state = false;
        } else {
            self.prev_interaction_state = true;
        }
    }

    pub fn pick(&mut self, graph: &Graph, options: PickingOptions) -> Option<CameraPickResult> {
        let PickingOptions {
            cursor_pos,
            editor_only,
            mut filter,
            ignore_back_faces,
            use_picking_loop,
            only_meshes,
        } = options;

        if let Some(camera) = graph[self.camera].cast::<Camera>() {
            let ray = camera.make_ray(cursor_pos, self.screen_size);

            self.stack.clear();
            let context = if editor_only {
                // In case if we want to pick stuff from editor scene only, we have to
                // start traversing graph from editor root.
                self.stack.push(self.editor_objects_root);
                &mut self.editor_context
            } else {
                self.stack.push(self.scene_content_root);
                &mut self.scene_context
            };

            context.pick_list.clear();

            while let Some(handle) = self.stack.pop() {
                if handle == self.grid
                    || handle == self.camera
                    || handle == self.camera_hinge
                    || handle == self.pivot
                {
                    continue;
                }

                // Ignore editor nodes if we picking scene stuff only.
                if !editor_only && handle == self.editor_objects_root {
                    continue;
                }

                let node = &graph[handle];

                self.stack.extend_from_slice(node.children());

                if !node.global_visibility()
                    || !filter.as_mut().map_or(true, |func| func(handle, node))
                {
                    continue;
                }

                if handle != self.scene_content_root {
                    let aabb = if node.is_resource_instance_root() {
                        let mut aabb = graph.aabb_of_descendants(handle, |_, _| true).unwrap();
                        // Inflate the bounding box by a tiny amount to ensure that it will be
                        // larger than any inner bounding boxes all the times.
                        aabb.inflate(Vector3::repeat(10.0 * f32::EPSILON));
                        aabb
                    } else {
                        node.local_bounding_box()
                            .transform(&node.global_transform())
                    };
                    // Do coarse, but fast, intersection test with bounding box first.
                    if let Some(points) = ray.aabb_intersection_points(&aabb) {
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
                        } else if !only_meshes {
                            // Hull-less objects (light sources, cameras, etc.) can still be selected
                            // by coarse intersection test results.
                            let da = points[0].metric_distance(&ray.origin);
                            let db = points[1].metric_distance(&ray.origin);
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

            if use_picking_loop {
                let mut hasher = DefaultHasher::new();
                for result in context.pick_list.iter() {
                    result.node.hash(&mut hasher);
                }
                let selection_hash = hasher.finish();
                if selection_hash == context.old_selection_hash
                    && cursor_pos == context.old_cursor_pos
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
            } else {
                context.pick_index = 0;
            }
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
            let data = data.data_ref();

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
