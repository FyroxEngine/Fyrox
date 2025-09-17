// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::settings::selection::SelectionSettings;
use crate::{
    fyrox::{
        core::{
            algebra::{clamp, Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
            math::{
                aabb::AxisAlignedBoundingBox, plane::Plane, ray::Ray, Matrix4Ext,
                TriangleDefinition, Vector3Ext,
            },
            pool::Handle,
        },
        graph::{BaseSceneGraph, SceneGraph, SceneGraphNode},
        gui::message::{KeyCode, KeyboardModifiers, MouseButton},
        renderer::bundle::{RenderContext, RenderDataBundleStorage},
        scene::{
            base::BaseBuilder,
            camera::{Camera, CameraBuilder, Exposure, FitParameters, Projection},
            collider::BitMask,
            graph::Graph,
            mesh::{
                buffer::{VertexAttributeUsage, VertexReadTrait},
                surface::SurfaceData,
            },
            node::Node,
            pivot::PivotBuilder,
            sound::listener::ListenerBuilder,
            transform::TransformBuilder,
            Scene,
        },
    },
    settings::{
        keys::KeyBindings,
        scene::{SceneCameraSettings, SceneSettings},
        Settings,
    },
};
use bitflags::bitflags;
use fyrox::renderer::cache::DynamicSurfaceCache;
use fyrox::renderer::observer::ObserverPosition;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
};

pub mod panel;

pub const DEFAULT_Z_OFFSET: f32 = -3.0;

#[derive(PartialEq, Copy, Clone)]
enum MouseControlMode {
    None,
    CenteredRotation {
        prev_z_offset: f32,
    },
    OrbitalRotation,
    Drag {
        initial_position: Vector3<f32>,
        initial_mouse_position: Vector2<f32>,
    },
}

pub struct CameraController {
    pub pivot: Handle<Node>,
    pub camera_hinge: Handle<Node>,
    pub camera: Handle<Node>,
    yaw: f32,
    pitch: f32,
    pub z_offset: f32,
    mouse_control_mode: MouseControlMode,
    move_left: bool,
    move_right: bool,
    move_forward: bool,
    move_backward: bool,
    move_up: bool,
    move_down: bool,
    speed_factor: f32,
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

bitflags! {
    pub struct PickMethod: u8 {
        const DEFAULT = 0b0000_0011;
        const PRECISE_HULL_RAY_TEST = 0b0000_0001;
        const COARSE_AABB_RAY_TEST = 0b0000_0010;
    }
}

impl Default for PickMethod {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub struct PickingOptions<'a> {
    pub cursor_pos: Vector2<f32>,
    pub editor_only: bool,
    pub filter: PickingFilter<'a>,
    pub ignore_back_faces: bool,
    pub use_picking_loop: bool,
    pub method: PickMethod,
    pub settings: &'a SelectionSettings,
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

        graph.link_nodes(grid, camera);
        graph.link_nodes(pivot, root);

        Self {
            pivot,
            camera_hinge,
            camera,
            yaw: settings.yaw,
            pitch: settings.pitch,
            mouse_control_mode: MouseControlMode::None,
            z_offset: DEFAULT_Z_OFFSET,
            move_left: false,
            move_right: false,
            move_forward: false,
            move_backward: false,
            move_up: false,
            move_down: false,
            speed_factor: 1.0,
            editor_context: Default::default(),
            scene_context: Default::default(),
            prev_interaction_state: false,
            grid,
            editor_objects_root,
            scene_content_root,
            screen_size: Default::default(),
        }
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp((-90.0f32).to_radians(), 90.0f32.to_radians());
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
    }

    pub fn is_interacting(&self) -> bool {
        self.move_backward
            || self.move_forward
            || self.move_left
            || self.move_right
            || self.mouse_control_mode != MouseControlMode::None
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
        if let Ok(relative_to) = graph.try_get_node(relative_to) {
            return relative_to
                .global_transform()
                .try_inverse()
                .unwrap_or_default()
                .transform_point(&world_space_position.into())
                .coords;
        }
        world_space_position
    }

    pub fn fit_object(&mut self, scene: &mut Scene, handle: Handle<Node>, scale: Option<f32>) {
        // Combine AABBs from the descendants.
        let mut aabb = AxisAlignedBoundingBox::default();
        for (_, descendant) in scene.graph.traverse_iter(handle) {
            let descendant_aabb = descendant.local_bounding_box();
            if !descendant_aabb.is_invalid_or_degenerate() {
                aabb.add_box(descendant_aabb.transform(&descendant.global_transform()))
            }
        }

        if aabb.is_invalid_or_degenerate() {
            // To prevent the camera from flying away into abyss.
            aabb = AxisAlignedBoundingBox::from_point(scene.graph[handle].global_position());
        }

        if let Some(scale) = scale {
            aabb.scale(scale);
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
            1.0,
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

    pub fn on_mouse_move(
        &mut self,
        graph: &mut Graph,
        mouse_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        delta: Vector2<f32>,
        settings: &Settings,
    ) {
        match self.mouse_control_mode {
            MouseControlMode::None => {}
            MouseControlMode::CenteredRotation { .. } | MouseControlMode::OrbitalRotation => {
                const MAX_ANGLE_RAD: f32 = 90.0f32.to_radians();
                const GLOBAL_MOUSE_SENSITIVITY: f32 = 0.01f32;
                let mouse_sensitivity = GLOBAL_MOUSE_SENSITIVITY * settings.camera.sensitivity;
                self.yaw -= delta.x * mouse_sensitivity;
                self.pitch += delta.y * mouse_sensitivity;
                self.pitch = clamp(self.pitch, -MAX_ANGLE_RAD, MAX_ANGLE_RAD);
            }
            MouseControlMode::Drag {
                initial_position,
                initial_mouse_position,
            } => {
                let camera = &graph[self.camera].as_camera();
                let scale = match camera.projection() {
                    Projection::Perspective(perspective) => 2.0 * perspective.fov.tan(),
                    Projection::Orthographic(orthographic) => 2.0 * orthographic.vertical_size,
                };
                let side = camera
                    .side_vector()
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default();
                let up = camera
                    .up_vector()
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default();
                let delta = mouse_position - initial_mouse_position;
                let offset = side.scale(scale * delta.x / screen_size.x)
                    + up.scale(scale * delta.y / screen_size.y);
                graph[self.pivot]
                    .local_transform_mut()
                    .set_position(initial_position + offset);
            }
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
            MouseButton::Right => match self.mouse_control_mode {
                MouseControlMode::CenteredRotation { prev_z_offset } => {
                    self.z_offset = prev_z_offset;
                    self.move_along_look_vector(-self.z_offset, graph);

                    self.mouse_control_mode = MouseControlMode::None;
                }
                MouseControlMode::Drag { .. } => {
                    self.mouse_control_mode = MouseControlMode::None;
                }
                _ => {}
            },
            MouseButton::Middle => {
                self.mouse_control_mode = MouseControlMode::None;
            }
            _ => (),
        }
    }

    pub fn on_mouse_button_down(
        &mut self,
        mouse_position: Vector2<f32>,
        button: MouseButton,
        modifiers: KeyboardModifiers,
        graph: &mut Graph,
    ) {
        let is_perspective = graph[self.camera].as_camera().projection().is_perspective();

        match button {
            MouseButton::Right => {
                if is_perspective {
                    if modifiers.shift {
                        self.mouse_control_mode = MouseControlMode::Drag {
                            initial_position: self.position(graph),
                            initial_mouse_position: mouse_position,
                        };
                    } else {
                        self.mouse_control_mode = MouseControlMode::CenteredRotation {
                            prev_z_offset: self.z_offset,
                        };
                        self.move_along_look_vector(self.z_offset, graph);
                        self.z_offset = 0.0;
                    }
                }
            }
            MouseButton::Middle => {
                if is_perspective {
                    self.mouse_control_mode = MouseControlMode::OrbitalRotation;
                } else {
                    self.mouse_control_mode = MouseControlMode::Drag {
                        initial_position: self.position(graph),
                        initial_mouse_position: mouse_position,
                    };
                }
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
        if self.mouse_control_mode == MouseControlMode::None {
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

                if self.mouse_control_mode != MouseControlMode::None {
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

                if self.mouse_control_mode != MouseControlMode::None {
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

                if let Some(v) = move_vec.try_normalize(f32::EPSILON) {
                    move_vec = v.scale(self.speed_factor * settings.camera.speed * dt);
                }

                camera
                    .local_transform_mut()
                    .set_rotation(Default::default());

                graph[self.camera_hinge]
                    .local_transform_mut()
                    .set_rotation(Default::default());

                let pivot_local_transform = graph[self.pivot].local_transform_mut();

                let mut new_position = **pivot_local_transform.position();
                new_position.z = DEFAULT_Z_OFFSET;
                new_position.x += move_vec.x;
                new_position.y += move_vec.y;

                pivot_local_transform
                    .set_rotation(Default::default())
                    .set_position(new_position);
            }
        }

        if !self.is_interacting() && self.prev_interaction_state {
            self.on_interaction_ended(settings, scene_path, graph);
            self.prev_interaction_state = false;
        } else {
            self.prev_interaction_state = true;
        }
    }

    fn pick_recursive(
        &self,
        handle: Handle<Node>,
        ray: &Ray,
        camera: &Camera,
        graph: &Graph,
        picked_meshes: &mut Vec<CameraPickResult>,
        picked_non_meshes: &mut Vec<CameraPickResult>,
        mut toi_limit: f32,
        options: &mut PickingOptions,
    ) {
        if handle == self.grid
            || handle == self.camera
            || handle == self.camera_hinge
            || handle == self.pivot
        {
            return;
        }

        // Ignore editor nodes if we picking scene stuff only.
        if !options.editor_only && handle == self.editor_objects_root {
            return;
        }

        let node = &graph[handle];

        if node.global_visibility()
            && handle != self.scene_content_root
            && options
                .filter
                .as_mut()
                .is_none_or(|func| func(handle, node))
        {
            if node.is_resource_instance_root() {
                // Special case for prefab roots.
                if let Some(prefab_pick_result) =
                    probe_hierarchy_precise(handle, graph, camera, ray, options.ignore_back_faces)
                {
                    if let Some(position) = prefab_pick_result.pick_position {
                        picked_meshes.push(CameraPickResult {
                            position: position.closest_point,
                            node: handle,
                            toi: position.closest_distance.max(toi_limit),
                        });
                        // Limit selection toi for descendants to always prefer the
                        // prefab root in selection.
                        toi_limit = toi_limit.max(position.closest_distance) + f32::EPSILON;
                    }
                }
            } else {
                let aabb = node
                    .local_bounding_box()
                    .transform(&node.global_transform());

                if ray.aabb_intersection_points(&aabb).is_some() {
                    let result =
                        precise_ray_test(node, camera, graph, ray, options.ignore_back_faces);

                    let mut added = false;
                    if options.method.contains(PickMethod::PRECISE_HULL_RAY_TEST)
                        && result.has_hull()
                    {
                        if let Some(position) = result.pick_position {
                            picked_meshes.push(CameraPickResult {
                                position: position.closest_point,
                                node: handle,
                                toi: position.closest_distance.max(toi_limit),
                            });
                            added = true;
                        }
                    }

                    if !added && options.method.contains(PickMethod::COARSE_AABB_RAY_TEST) {
                        // Hull-less objects (light sources, cameras, etc.) can still be selected
                        // by coarse intersection test with a simplified bounding box.
                        let simple_aabb =
                            AxisAlignedBoundingBox::from_radius(if options.editor_only {
                                1.0
                            } else {
                                options.settings.hull_less_object_selection_radius
                            })
                            .transform(&node.global_transform());
                        if let Some(points) = ray.aabb_intersection_points(&simple_aabb) {
                            let da = points[0].metric_distance(&ray.origin);
                            let db = points[1].metric_distance(&ray.origin);
                            let closest_distance = da.min(db);
                            picked_non_meshes.push(CameraPickResult {
                                position: if da < db { points[0] } else { points[1] },
                                node: handle,
                                toi: closest_distance.max(toi_limit),
                            });
                        }
                    }
                }
            }
        }

        for child in node.children() {
            self.pick_recursive(
                *child,
                ray,
                camera,
                graph,
                picked_meshes,
                picked_non_meshes,
                toi_limit,
                options,
            )
        }
    }

    pub fn pick(&mut self, graph: &Graph, mut options: PickingOptions) -> Option<CameraPickResult> {
        if let Some(camera) = graph[self.camera].cast::<Camera>() {
            let ray = camera.make_ray(options.cursor_pos, self.screen_size);

            let root = if options.editor_only {
                // In case if we want to pick stuff from editor scene only, we have to
                // start traversing graph from editor root.
                self.editor_objects_root
            } else {
                self.scene_content_root
            };

            let mut picked_meshes = Vec::new();
            let mut picked_non_meshes = Vec::new();
            self.pick_recursive(
                root,
                &ray,
                camera,
                graph,
                &mut picked_meshes,
                &mut picked_non_meshes,
                0.0,
                &mut options,
            );

            fn sort_by_toi(list: &mut [CameraPickResult]) {
                list.sort_by(|a, b| a.toi.partial_cmp(&b.toi).unwrap());
            }
            sort_by_toi(&mut picked_meshes);
            sort_by_toi(&mut picked_non_meshes);

            let context = if options.editor_only {
                &mut self.editor_context
            } else {
                &mut self.scene_context
            };
            context.pick_list.clear();
            context.pick_list.append(&mut picked_meshes);
            context.pick_list.append(&mut picked_non_meshes);

            if options.use_picking_loop {
                let mut hasher = DefaultHasher::new();
                for result in context.pick_list.iter() {
                    result.node.hash(&mut hasher);
                }
                let selection_hash = hasher.finish();
                if selection_hash == context.old_selection_hash
                    && options.cursor_pos == context.old_cursor_pos
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
            context.old_cursor_pos = options.cursor_pos;

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

#[derive(Clone, Debug)]
struct PickPosition {
    closest_distance: f32,
    closest_point: Vector3<f32>,
}

#[derive(Clone, Debug)]
struct PreciseRayTestResult {
    pick_position: Option<PickPosition>,
    // Total number of the instances checked with ray test. This number will be zero for objects
    // without a "hull".
    instance_count: usize,
}

impl PreciseRayTestResult {
    fn has_hull(&self) -> bool {
        self.instance_count > 0
    }
}

fn probe_hierarchy_precise(
    handle: Handle<Node>,
    graph: &Graph,
    camera: &Camera,
    ray: &Ray,
    ignore_back_faces: bool,
) -> Option<PreciseRayTestResult> {
    let mut closest_result: Option<PreciseRayTestResult> = None;
    for (_, descendant) in graph.traverse_iter(handle) {
        let result = precise_ray_test(descendant, camera, graph, ray, ignore_back_faces);
        if let Some(ref pick_position) = result.pick_position {
            let closest_result = closest_result.get_or_insert(result.clone());
            let closest_distance = closest_result
                .pick_position
                .as_ref()
                .unwrap()
                .closest_distance;
            if pick_position.closest_distance < closest_distance {
                *closest_result = result;
            }
        }
    }
    closest_result
}

fn precise_ray_test(
    node: &Node,
    camera: &Camera,
    graph: &Graph,
    ray: &Ray,
    ignore_back_faces: bool,
) -> PreciseRayTestResult {
    let mut cache = DynamicSurfaceCache::new();
    let observer_position = ObserverPosition::from_camera(camera);
    let mut bundle_storage = RenderDataBundleStorage::new_empty(observer_position.clone());
    node.collect_render_data(&mut RenderContext {
        render_mask: BitMask::all(),
        elapsed_time: 0.0,
        observer_position: &observer_position,
        frustum: Some(&camera.frustum()),
        storage: &mut bundle_storage,
        graph,
        render_pass_name: &Default::default(),
        dynamic_surface_cache: &mut cache,
    });
    let mut closest_distance = f32::MAX;
    let mut closest_point = None;
    let mut instance_count = 0;
    for bundle in bundle_storage.bundles {
        let data = bundle.data.data_ref();

        for instance in bundle.instances {
            instance_count += 1;
            for triangle in data
                .geometry_buffer
                .iter()
                .filter_map(|t| read_triangle(&data, t, &instance.world_transform))
            {
                if ignore_back_faces {
                    // If normal of the triangle is facing in the same direction as ray's direction,
                    // then we skip such a triangle.
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
    PreciseRayTestResult {
        pick_position: closest_point.map(|pt| PickPosition {
            closest_distance,
            closest_point: pt,
        }),
        instance_count,
    }
}
