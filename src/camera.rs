use crate::{
    GameEngine,
    EditorScene
};
use rg3d::{
    scene::{
        base::BaseBuilder,
        transform::TransformBuilder,
        camera::CameraBuilder,
        node::Node
    },
    core::math::{
        vec3::Vec3,
        quat::Quat,
        vec2::Vec2,
        aabb::AxisAlignedBoundingBox
    },
    core::pool::Handle,
    event::{
        WindowEvent,
        DeviceEvent,
        VirtualKeyCode,
        ElementState,
        MouseScrollDelta,
        MouseButton,
    }
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher}
};

pub struct CameraController {
    pub pivot: Handle<Node>,
    pub camera: Handle<Node>,
    yaw: f32,
    pitch: f32,
    rotate: bool,
    move_left: bool,
    move_right: bool,
    move_forward: bool,
    move_backward: bool,
    stack: Vec<Handle<Node>>,
    pick_list: Vec<(Handle<Node>, f32)>,
    pick_index: usize,
    old_selection_hash: u64,
    old_cursor_pos: Vec2,
}

impl CameraController {
    pub fn new(editor_scene: &EditorScene, engine: &mut GameEngine) -> Self {
        let graph = &mut engine.scenes[editor_scene.scene].graph;

        let camera = CameraBuilder::new(BaseBuilder::new()
            .with_name("EditorCamera"))
            .build();

        let pivot = BaseBuilder::new()
            .with_name("EditorCameraPivot")
            .with_local_transform(TransformBuilder::new()
                .with_local_position(Vec3::new(0.0, 1.0, -3.0))
                .build())
            .build();

        let pivot = graph.add_node(Node::Base(pivot));
        let camera = graph.add_node(Node::Camera(camera));

        graph.link_nodes(pivot, editor_scene.root);
        graph.link_nodes(camera, pivot);

        Self {
            pivot,
            camera,
            yaw: 0.0,
            pitch: 0.0,
            rotate: false,
            move_left: false,
            move_right: false,
            move_forward: false,
            move_backward: false,
            stack: Default::default(),
            pick_list: Default::default(),
            pick_index: 0,
            old_selection_hash: 0,
            old_cursor_pos: Default::default(),
        }
    }

    pub fn handle_raw_input(&mut self, editor_scene: &EditorScene, device_event: &DeviceEvent, engine: &mut GameEngine) {
        match device_event {
            &DeviceEvent::MouseMotion { delta } => {
                if self.rotate {
                    self.yaw -= delta.0 as f32 * 0.01;
                    self.pitch += delta.1 as f32 * 0.01;
                }
            }
            &DeviceEvent::MouseWheel { delta } => {
                if let MouseScrollDelta::LineDelta(_, delta) = delta {
                    let scene = &mut engine.scenes[editor_scene.scene];
                    let camera = &mut scene.graph[self.camera];

                    let look = camera.global_transform().look();

                    if let Node::Base(pivot) = &mut scene.graph[self.pivot] {
                        pivot.local_transform_mut()
                            .offset(look.scale(delta));
                    }
                }
            }
            DeviceEvent::Key(keyboard) => {
                if let Some(code) = keyboard.virtual_keycode {
                    match code {
                        VirtualKeyCode::W => self.move_forward = keyboard.state == ElementState::Pressed,
                        VirtualKeyCode::S => self.move_backward = keyboard.state == ElementState::Pressed,
                        VirtualKeyCode::A => self.move_left = keyboard.state == ElementState::Pressed,
                        VirtualKeyCode::D => self.move_right = keyboard.state == ElementState::Pressed,
                        _ => ()
                    }
                }
            }
            _ => ()
        }
    }

    pub fn handle_input(&mut self, window_event: &WindowEvent, _engine: &mut GameEngine) {
        match window_event {
            &WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    self.rotate = state == ElementState::Pressed;
                }
            }
            _ => ()
        }
    }

    pub fn update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine, dt: f32) {
        let scene = &mut engine.scenes[editor_scene.scene];
        let camera = &mut scene.graph[self.camera];

        let global_transform = camera.global_transform();
        let look = global_transform.look();
        let side = global_transform.side();

        let mut move_vec = Vec3::ZERO;
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
        if let Some(v) = move_vec.normalized() {
            move_vec = v.scale(10.0 * dt);
        }

        if let Node::Camera(camera) = camera {
            let pitch = Quat::from_axis_angle(Vec3::RIGHT, self.pitch);
            camera.local_transform_mut().set_rotation(pitch);
        }
        if let Node::Base(pivot) = &mut scene.graph[self.pivot] {
            let yaw = Quat::from_axis_angle(Vec3::UP, self.yaw);
            pivot.local_transform_mut()
                .set_rotation(yaw)
                .offset(move_vec);
        }
    }

    pub fn pick<F>(&mut self, cursor_pos: Vec2, editor_scene: &EditorScene, engine: &GameEngine, editor_only: bool, mut filter: F) -> Handle<Node>
        where F: FnMut(Handle<Node>, &Node) -> bool {
        let scene = &engine.scenes[editor_scene.scene];
        let camera = &scene.graph[self.camera];
        if let Node::Camera(camera) = camera {
            let screen_size = engine.renderer.get_frame_size();
            let ray = camera.make_ray(cursor_pos, Vec2::new(screen_size.0 as f32, screen_size.1 as f32));

            self.pick_list.clear();

            self.stack.clear();
            if editor_only {
                // In case if we want to pick stuff from editor scene only, we have to
                // start traversing graph from editor root.
                self.stack.push(editor_scene.root);
            } else {
                self.stack.push(scene.graph.get_root());
            }
            while let Some(handle) = self.stack.pop() {
                // Ignore editor nodes if we picking scene stuff only.
                if !editor_only && handle == editor_scene.root {
                    continue;
                }

                let node = &scene.graph[handle];

                if !node.global_visibility() || !filter(handle, node) {
                    continue;
                }

                let (aabb, surfaces) = match node {
                    Node::Base(_) => (AxisAlignedBoundingBox::default(), None), // Non-pickable. TODO: Maybe better filter out such nodes?
                    Node::Light(_) => (AxisAlignedBoundingBox::UNIT, None),
                    Node::Camera(_) => (AxisAlignedBoundingBox::UNIT, None),
                    Node::Mesh(mesh) => (mesh.bounding_box(), Some(mesh.surfaces())),
                    Node::Sprite(_) => (AxisAlignedBoundingBox::UNIT, None),
                    Node::ParticleSystem(_) => (AxisAlignedBoundingBox::UNIT, None),
                };

                if handle != scene.graph.get_root() {
                    let object_space_ray = ray.transform(node.global_transform().inverse().unwrap_or_default());
                    // Do coarse intersection test with bounding box.
                    if let Some(points) = object_space_ray.aabb_intersection_points(&aabb) {
                        // Do fine intersection test with surfaces if any
                        if let Some(_surfaces) = surfaces {
                            // TODO
                        }

                        let da = points[0].distance(&object_space_ray.origin);
                        let db = points[1].distance(&object_space_ray.origin);
                        let closest_distance = da.min(db);
                        self.pick_list.push((handle, closest_distance));
                    }
                }

                for &child in node.children() {
                    self.stack.push(child);
                }
            }

            // Make sure closest will be selected first.
            self.pick_list.sort_by(|&(_, a), (_, b)| a.partial_cmp(b).unwrap());

            let mut hasher = DefaultHasher::new();
            for (handle, _) in self.pick_list.iter() {
                handle.hash(&mut hasher);
            }
            let selection_hash = hasher.finish();
            if selection_hash == self.old_selection_hash && cursor_pos == self.old_cursor_pos {
                self.pick_index += 1;

                // Wrap picking loop.
                if self.pick_index >= self.pick_list.len() {
                    self.pick_index = 0;
                }
            } else {
                // Select is different, start from beginning.
                self.pick_index = 0;
            }
            self.old_selection_hash = selection_hash;
            self.old_cursor_pos = cursor_pos;

            if !self.pick_list.is_empty() {
                if let Some(&(handle, _)) = self.pick_list.get(self.pick_index) {
                    return handle;
                }
            }
        }

        Handle::NONE
    }
}
