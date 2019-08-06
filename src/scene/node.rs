use crate::{
    math::{
        vec3::*,
        mat4::*,
        quat::*,
        *,
        vec2::*
    },
    renderer::surface::*,
    utils::pool::*,
    physics::Body,
    engine::State
};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Light {
    radius: f32,
    color: Vec3,
}

impl Default for Light {
    fn default() -> Light {
        Light {
            radius: 10.0,
            color: Vec3 { x: 1.0, y: 1.0, z: 1.0 },
        }
    }
}

impl Light {
    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    #[inline]
    pub fn set_color(&mut self, color: Vec3) {
        self.color = color;
    }

    #[inline]
    pub fn make_copy(&self) -> Light {
        Light {
            radius: self.radius,
            color: self.color
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Camera {
    fov: f32,
    z_near: f32,
    z_far: f32,
    viewport: Rect<f32>,
    view_matrix: Mat4,
    projection_matrix: Mat4,
}

impl Default for Camera {
    fn default() -> Camera {
        let fov: f32 = 75.0;
        let z_near: f32 = 0.025;
        let z_far: f32 = 2048.0;

        Camera {
            fov,
            z_near,
            z_far,
            view_matrix: Mat4::identity(),
            projection_matrix: Mat4::perspective(fov.to_radians(), 1.0, z_near, z_far),
            viewport: Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 },
        }
    }
}

impl Camera {
    #[inline]
    pub fn calculate_matrices(&mut self, pos: Vec3, look: Vec3, up: Vec3, aspect: f32) {
        if let Some(view_matrix) = Mat4::look_at(pos, pos + look, up) {
            self.view_matrix = view_matrix;
        } else {
            self.view_matrix = Mat4::identity();
        }
        self.projection_matrix = Mat4::perspective(self.fov.to_radians(), aspect, self.z_near, self.z_far);
    }

    #[inline]
    pub fn get_viewport_pixels(&self, client_size: Vec2) -> Rect<i32> {
        Rect {
            x: (self.viewport.x * client_size.x) as i32,
            y: (self.viewport.y * client_size.y) as i32,
            w: (self.viewport.w * client_size.x) as i32,
            h: (self.viewport.h * client_size.y) as i32,
        }
    }

    #[inline]
    pub fn get_view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix * self.view_matrix
    }

    #[inline]
    pub fn make_copy(&self) -> Camera {
        Camera {
            fov: self.fov,
            z_near: self.z_near,
            z_far: self.z_far,
            viewport: self.viewport,
            view_matrix: self.view_matrix,
            projection_matrix: self.projection_matrix,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Mesh {
    surfaces: Vec<Surface>
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            surfaces: Vec::new()
        }
    }
}

impl Mesh {
    #[inline]
    pub fn get_surfaces(&self) -> &Vec<Surface> {
        &self.surfaces
    }

    #[inline]
    pub fn get_surfaces_mut(&mut self) -> &mut Vec<Surface> {
        &mut self.surfaces
    }

    #[inline]
    pub fn add_surface(&mut self, surface: Surface) {
        self.surfaces.push(surface);
    }

    #[inline]
    pub fn make_copy(&self, state: &State) -> Mesh {
        Mesh {
            surfaces: self.surfaces.iter().map(|surf| surf.make_copy(state)).collect()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum NodeKind {
    Base,
    Light(Light),
    Camera(Camera),
    Mesh(Mesh)
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    name: String,
    kind: NodeKind,
    local_scale: Vec3,
    local_position: Vec3,
    local_rotation: Quat,
    pre_rotation: Quat,
    post_rotation: Quat,
    rotation_offset: Vec3,
    rotation_pivot: Vec3,
    scaling_offset: Vec3,
    scaling_pivot: Vec3,
    pub(super) visibility: bool,
    pub(super) global_visibility: bool,
    pub(super) parent: Handle<Node>,
    pub(crate) children: Vec<Handle<Node>>,
    pub(super) local_transform: Mat4,
    pub(crate) global_transform: Mat4,
    body: Handle<Body>
}

impl Node {
    pub fn new(kind: NodeKind) -> Self {
        Node {
            kind,
            name: String::from("Node"),
            children: Vec::new(),
            parent: Handle::none(),
            local_position: Vec3::new(),
            local_scale: Vec3 { x: 1.0, y: 1.0, z: 1.0 },
            local_rotation: Quat::new(),
            pre_rotation: Quat::new(),
            post_rotation: Quat::new(),
            rotation_offset: Vec3::new(),
            rotation_pivot: Vec3::new(),
            scaling_offset: Vec3::new(),
            scaling_pivot: Vec3::new(),
            visibility: true,
            global_visibility: true,
            local_transform: Mat4::identity(),
            global_transform: Mat4::identity(),
            body: Handle::none()
        }
    }

    pub fn calculate_local_transform(&mut self) {
        let pre_rotation = Mat4::from_quat(self.pre_rotation);
        let post_rotation = Mat4::from_quat(self.post_rotation).inverse().unwrap();
        let rotation = Mat4::from_quat(self.local_rotation);
        let scale = Mat4::scale(self.local_scale);
        let translation = Mat4::translate(self.local_position);
        let rotation_offset = Mat4::translate(self.rotation_offset);
        let rotation_pivot = Mat4::translate(self.rotation_pivot);
        let rotation_pivot_inv = rotation_pivot.inverse().unwrap();
        let scale_offset = Mat4::translate(self.scaling_offset);
        let scale_pivot = Mat4::translate(self.scaling_pivot);
        let scale_pivot_inv = scale_pivot.inverse().unwrap();

        self.local_transform = translation * rotation_offset * rotation_pivot *
            pre_rotation * rotation * post_rotation * rotation_pivot_inv *
            scale_offset * scale_pivot * scale * scale_pivot_inv;
    }

    /// Creates copy of node without copying children nodes and physics body.
    /// Children nodes has to be copied explicitly.
    pub fn make_copy(&self, state: &State) -> Node {
        Node {
            kind: match &self.kind {
                NodeKind::Camera(camera) => NodeKind::Camera(camera.make_copy()),
                NodeKind::Light(light) => NodeKind::Light(light.make_copy()),
                NodeKind::Mesh(mesh) => NodeKind::Mesh(mesh.make_copy(state)),
                NodeKind::Base => NodeKind::Base
            },
            name: self.name.clone(),
            local_position: self.local_position,
            local_scale: self.local_scale,
            local_rotation: self.local_rotation,
            pre_rotation: self.pre_rotation,
            post_rotation: self.post_rotation,
            rotation_offset: self.rotation_offset,
            rotation_pivot: self.rotation_pivot,
            scaling_offset: self.scaling_offset,
            scaling_pivot: self.scaling_pivot,
            local_transform: self.local_transform,
            global_transform: self.global_transform,
            visibility: self.visibility,
            global_visibility: self.global_visibility,
            children: Vec::new(),
            parent: Handle::none(),
            body: Handle::none()
        }
    }

    #[inline]
    pub fn set_body(&mut self, body: Handle<Body>) {
        self.body = body;
    }

    #[inline]
    pub fn get_body(&self) -> Handle<Body> {
        self.body.clone()
    }

    #[inline]
    pub fn borrow_kind(&self) -> &NodeKind {
        &self.kind
    }

    #[inline]
    pub fn borrow_kind_mut(&mut self) -> &mut NodeKind {
        &mut self.kind
    }

    #[inline]
    pub fn set_local_position(&mut self, pos: Vec3) {
        self.local_position = pos;
    }

    #[inline]
    pub fn set_local_rotation(&mut self, rot: Quat) {
        self.local_rotation = rot;
    }

    #[inline]
    pub fn set_pre_rotation(&mut self, pre_rotation: Quat) {
        self.pre_rotation = pre_rotation;
    }

    #[inline]
    pub fn set_post_rotation(&mut self, post_rotation: Quat) {
        self.post_rotation = post_rotation;
    }

    #[inline]
    pub fn set_rotation_offset(&mut self, rotation_offset: Vec3) {
        self.rotation_offset = rotation_offset;
    }

    #[inline]
    pub fn set_rotation_pivot(&mut self, rotation_pivot: Vec3) {
        self.rotation_pivot = rotation_pivot;
    }

    #[inline]
    pub fn set_scaling_offset(&mut self, scaling_offset: Vec3) {
        self.scaling_offset = scaling_offset;
    }

    #[inline]
    pub fn set_scaling_pivot(&mut self, scaling_pivot: Vec3) {
        self.scaling_pivot = scaling_pivot;
    }

    #[inline]
    pub fn set_visibility(&mut self, visibility: bool) {
        self.visibility = visibility;
    }

    #[inline]
    pub fn get_visibility(&self) -> bool {
        self.visibility
    }

    #[inline]
    pub fn get_global_visibility(&self) -> bool {
        self.global_visibility
    }

    #[inline]
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    #[inline]
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    pub fn set_local_scale(&mut self, scl: Vec3) {
        self.local_scale = scl;
    }

    #[inline]
    pub fn offset(&mut self, vec: Vec3) {
        self.local_position += vec;
    }

    #[inline]
    pub fn get_global_position(&self) -> Vec3 {
        Vec3 {
            x: self.global_transform.f[12],
            y: self.global_transform.f[13],
            z: self.global_transform.f[14],
        }
    }

    #[inline]
    pub fn get_look_vector(&self) -> Vec3 {
        Vec3 {
            x: self.global_transform.f[8],
            y: self.global_transform.f[9],
            z: self.global_transform.f[10],
        }
    }

    #[inline]
    pub fn get_side_vector(&self) -> Vec3 {
        Vec3 {
            x: self.global_transform.f[0],
            y: self.global_transform.f[1],
            z: self.global_transform.f[2],
        }
    }

    #[inline]
    pub fn get_up_vector(&self) -> Vec3 {
        Vec3 {
            x: self.global_transform.f[4],
            y: self.global_transform.f[5],
            z: self.global_transform.f[6],
        }
    }
}
