use std::{
    cell::RefCell,
    rc::Rc,
};
use crate::{
    utils::{
        visitor::{
            VisitError,
            Visit,
            VisitResult,
            Visitor,
        },
        pool::*,
    },
    math::{
        vec3::*,
        mat4::*,
        quat::*,
        *,
        vec2::*,
    },
    renderer::surface::*,
    physics::Body,
    resource::Resource,
    gui::draw::Color,
};

pub struct Light {
    radius: f32,
    color: Color,
    cone_angle: f32,
    cone_angle_cos: f32,
}

impl Default for Light {
    fn default() -> Light {
        Light {
            radius: 10.0,
            color: Color::white(),
            cone_angle: std::f32::consts::PI,
            cone_angle_cos: -1.0,
        }
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.color.visit("Color", visitor)?;
        self.cone_angle.visit("ConeAngle", visitor)?;
        self.cone_angle_cos.visit("ConeAngleCos", visitor)?;

        visitor.leave_region()
    }
}

impl Light {
    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    #[inline]
    pub fn get_radius(&self) -> f32 {
        self.radius
    }

    #[inline]
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    #[inline]
    pub fn get_color(&self) -> Color {
        self.color
    }

    #[inline]
    pub fn get_cone_angle_cos(&self) -> f32 {
        self.cone_angle_cos
    }

    pub fn set_cone_angle(&mut self, cone_angle: f32) {
        self.cone_angle = cone_angle;
        self.cone_angle_cos = cone_angle.cos();
    }

    #[inline]
    pub fn make_copy(&self) -> Light {
        Light {
            radius: self.radius,
            color: self.color,
            cone_angle: self.cone_angle,
            cone_angle_cos: self.cone_angle_cos,
        }
    }
}

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

impl Visit for Camera {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.fov.visit("Fov", visitor)?;
        self.z_near.visit("ZNear", visitor)?;
        self.z_far.visit("ZFar", visitor)?;
        self.viewport.visit("Viewport", visitor)?;
        visitor.leave_region()
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

pub struct Mesh {
    surfaces: Vec<Surface>,
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            surfaces: Vec::new()
        }
    }
}

impl Visit for Mesh {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        // No need to serialize surfaces, correct ones will be assigned on resolve stage.
        visitor.leave_region()
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
    pub fn make_copy(&self) -> Mesh {
        Mesh {
            surfaces: self.surfaces.iter().map(|surf| surf.make_copy()).collect()
        }
    }
}

pub enum NodeKind {
    Base,
    Light(Light),
    Camera(Camera),
    Mesh(Mesh),
}

impl Visit for NodeKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            NodeKind::Base => Ok(()),
            NodeKind::Light(light) => light.visit(name, visitor),
            NodeKind::Camera(camera) => camera.visit(name, visitor),
            NodeKind::Mesh(mesh) => mesh.visit(name, visitor),
        }
    }
}

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
    pub(in crate::scene) visibility: bool,
    pub(in crate::scene) global_visibility: bool,
    pub(in crate::scene) parent: Handle<Node>,
    pub(in crate::scene) children: Vec<Handle<Node>>,
    pub(in crate::scene) local_transform: Mat4,
    pub(in crate::scene) global_transform: Mat4,
    inv_bind_pose_transform: Mat4,
    body: Handle<Body>,
    resource: Option<Rc<RefCell<Resource>>>,
    /// Handle to node in scene of model resource from which this node
    /// was instantiated from.
    original: Handle<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            kind: NodeKind::Base,
            name: String::new(),
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
            inv_bind_pose_transform: Mat4::identity(),
            body: Handle::none(),
            resource: None,
            original: Handle::none(),
        }
    }
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
            inv_bind_pose_transform: Mat4::identity(),
            body: Handle::none(),
            resource: None,
            original: Handle::none(),
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
    pub fn make_copy(&self, original: Handle<Node>) -> Node {
        Node {
            kind: match &self.kind {
                NodeKind::Camera(camera) => NodeKind::Camera(camera.make_copy()),
                NodeKind::Light(light) => NodeKind::Light(light.make_copy()),
                NodeKind::Mesh(mesh) => NodeKind::Mesh(mesh.make_copy()),
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
            inv_bind_pose_transform: self.inv_bind_pose_transform,
            children: Vec::new(),
            parent: Handle::none(),
            body: Handle::none(),
            resource: match &self.resource {
                Some(resource) => Some(Rc::clone(resource)),
                None => None
            },
            original,
        }
    }

    #[inline]
    pub fn get_original_handle(&self) -> Handle<Node> {
        self.original
    }

    #[inline]
    pub fn set_body(&mut self, body: Handle<Body>) {
        self.body = body;
    }

    #[inline]
    pub fn get_body(&self) -> Handle<Body> {
        self.body
    }

    #[inline]
    pub fn borrow_kind(&self) -> &NodeKind {
        &self.kind
    }

    #[inline]
    pub fn set_resource(&mut self, resource_handle: Rc<RefCell<Resource>>) {
        self.resource = Some(resource_handle);
    }

    #[inline]
    pub fn get_resource(&mut self) -> Option<Rc<RefCell<Resource>>> {
        match &self.resource {
            Some(resource) => Some(Rc::clone(resource)),
            None => None
        }
    }

    #[inline]
    pub fn get_local_position(&self) -> Vec3 {
        self.local_position
    }

    #[inline]
    pub fn get_local_rotation(&self) -> Quat {
        self.local_rotation
    }

    #[inline]
    pub fn get_local_scale(&self) -> Vec3 {
        self.local_scale
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
    pub fn get_children(&self) -> &[Handle<Node>] {
        &self.children
    }

    #[inline]
    pub fn get_global_transform(&self) -> &Mat4 {
        &self.global_transform
    }

    pub fn set_inv_bind_pose_transform(&mut self, transform: Mat4) {
        self.inv_bind_pose_transform = transform;
    }

    pub fn get_inv_bind_pose_transform(&self) -> &Mat4 {
        &self.inv_bind_pose_transform
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

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id: u8 = match &self.kind {
            NodeKind::Base => 0,
            NodeKind::Light(_) => 1,
            NodeKind::Camera(_) => 2,
            NodeKind::Mesh(_) => 3,
        };
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = match kind_id {
                0 => NodeKind::Base,
                1 => NodeKind::Light(Default::default()),
                2 => NodeKind::Camera(Default::default()),
                3 => NodeKind::Mesh(Default::default()),
                _ => return Err(VisitError::User(format!("invalid node kind {}", kind_id)))
            }
        }

        self.kind.visit("Kind", visitor)?;
        self.name.visit("Name", visitor)?;
        self.local_scale.visit("LocalScale", visitor)?;
        self.local_position.visit("LocalPosition", visitor)?;
        self.local_rotation.visit("LocalRotation", visitor)?;
        self.pre_rotation.visit("PreRotation", visitor)?;
        self.post_rotation.visit("PostRotation", visitor)?;
        self.rotation_offset.visit("RotationOffset", visitor)?;
        self.rotation_pivot.visit("RotationPivot", visitor)?;
        self.scaling_offset.visit("ScalingOffset", visitor)?;
        self.scaling_pivot.visit("ScalingPivot", visitor)?;
        self.visibility.visit("Visibility", visitor)?;
        self.parent.visit("Parent", visitor)?;
        self.children.visit("Children", visitor)?;
        self.body.visit("Body", visitor)?;
        self.resource.visit("Resource", visitor)?;

        // TODO: Is this needed?
        self.inv_bind_pose_transform.visit("InvBindPoseTransform", visitor)?;

        visitor.leave_region()
    }
}