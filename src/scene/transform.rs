use std::cell::Cell;
use rg3d_core::{
    math::{
        vec3::Vec3,
        quat::Quat,
        mat4::Mat4
    },
    visitor::{Visit, VisitResult, Visitor},
};

#[derive(Clone)]
pub struct Transform {
    /// Indicates that some property has changed and matrix must be
    /// recalculated before use. This is some sort of lazy evaluation.
    dirty: Cell<bool>,
    local_scale: Vec3,
    local_position: Vec3,
    local_rotation: Quat,
    pre_rotation: Quat,
    post_rotation: Quat,
    rotation_offset: Vec3,
    rotation_pivot: Vec3,
    scaling_offset: Vec3,
    scaling_pivot: Vec3,
    /// Combined transform. Final result of combination of other properties.
    matrix: Cell<Mat4>
}

impl Visit for Transform {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.local_scale.visit("LocalScale", visitor)?;
        self.local_position.visit("LocalPosition", visitor)?;
        self.local_rotation.visit("LocalRotation", visitor)?;
        self.pre_rotation.visit("PreRotation", visitor)?;
        self.post_rotation.visit("PostRotation", visitor)?;
        self.rotation_offset.visit("RotationOffset", visitor)?;
        self.rotation_pivot.visit("RotationPivot", visitor)?;
        self.scaling_offset.visit("ScalingOffset", visitor)?;
        self.scaling_pivot.visit("ScalingPivot", visitor)?;

        visitor.leave_region()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::identity()
    }
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            dirty: Cell::new(true),
            local_position: Vec3::new(),
            local_scale: Vec3::unit(),
            local_rotation: Quat::new(),
            pre_rotation: Quat::new(),
            post_rotation: Quat::new(),
            rotation_offset: Vec3::new(),
            rotation_pivot: Vec3::new(),
            scaling_offset: Vec3::new(),
            scaling_pivot: Vec3::new(),
            matrix: Cell::new(Mat4::identity()),
        }
    }

    #[inline]
    pub fn get_position(&self) -> Vec3 {
        self.local_position
    }

    #[inline]
    pub fn set_position(&mut self, pos: Vec3) {
        self.local_position = pos;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_rotation(&self) -> Quat {
        self.local_rotation
    }

    #[inline]
    pub fn set_rotation(&mut self, rot: Quat) {
        self.local_rotation = rot;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_scale(&self) -> Vec3 {
        self.local_scale
    }

    #[inline]
    pub fn set_scale(&mut self, scl: Vec3) {
        self.local_scale = scl;
        self.dirty.set(true);
    }

    #[inline]
    pub fn set_pre_rotation(&mut self, pre_rotation: Quat) {
        self.pre_rotation = pre_rotation;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_pre_rotation(&self) -> Quat {
        self.pre_rotation
    }

    #[inline]
    pub fn set_post_rotation(&mut self, post_rotation: Quat) {
        self.post_rotation = post_rotation;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_post_rotation(&self) -> Quat {
        self.post_rotation
    }

    #[inline]
    pub fn set_rotation_offset(&mut self, rotation_offset: Vec3) {
        self.rotation_offset = rotation_offset;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_rotation_offset(&self) -> Vec3 {
        self.rotation_offset
    }

    #[inline]
    pub fn set_rotation_pivot(&mut self, rotation_pivot: Vec3) {
        self.rotation_pivot = rotation_pivot;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_rotation_pivot(&self) -> Vec3 {
        self.rotation_pivot
    }

    #[inline]
    pub fn set_scaling_offset(&mut self, scaling_offset: Vec3) {
        self.scaling_offset = scaling_offset;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_scaling_offset(&self) -> Vec3 {
        self.scaling_offset
    }

    #[inline]
    pub fn set_scaling_pivot(&mut self, scaling_pivot: Vec3) {
        self.scaling_pivot = scaling_pivot;
        self.dirty.set(true);
    }

    #[inline]
    pub fn get_scaling_pivot(&self) -> Vec3 {
        self.scaling_pivot
    }

    #[inline]
    pub fn offset(&mut self, vec: Vec3) {
        self.local_position += vec;
        self.dirty.set(true);
    }

    fn calculate_local_transform(&self) -> Mat4 {
        let pre_rotation = Mat4::from_quat(self.pre_rotation);
        let post_rotation = Mat4::from_quat(self.post_rotation).inverse().unwrap_or_else(|_| {
            println!("Unable to inverse post rotation matrix! Fallback to identity matrix.");
            Mat4::identity()
        });
        let rotation = Mat4::from_quat(self.local_rotation);
        let scale = Mat4::scale(self.local_scale);
        let translation = Mat4::translate(self.local_position);
        let rotation_offset = Mat4::translate(self.rotation_offset);
        let rotation_pivot = Mat4::translate(self.rotation_pivot);
        let rotation_pivot_inv = rotation_pivot.inverse().unwrap_or_else(|_| {
            println!("Unable to inverse rotation pivot matrix! Fallback to identity matrix.");
            Mat4::identity()
        });
        let scale_offset = Mat4::translate(self.scaling_offset);
        let scale_pivot = Mat4::translate(self.scaling_pivot);
        let scale_pivot_inv = scale_pivot.inverse().unwrap_or_else(|_| {
            println!("Unable to inverse scale pivot matrix! Fallback to identity matrix.");
            Mat4::identity()
        });

        translation * rotation_offset * rotation_pivot * pre_rotation * rotation * post_rotation *
            rotation_pivot_inv * scale_offset * scale_pivot * scale * scale_pivot_inv
    }

    pub fn get_matrix(&self) -> Mat4 {
        if self.dirty.get() {
            self.matrix.set(self.calculate_local_transform());
            self.dirty.set(false)
        }
        self.matrix.get()
    }
}

pub struct TransformBuilder {
    local_scale: Option<Vec3>,
    local_position: Option<Vec3>,
    local_rotation: Option<Quat>,
    pre_rotation: Option<Quat>,
    post_rotation: Option<Quat>,
    rotation_offset: Option<Vec3>,
    rotation_pivot: Option<Vec3>,
    scaling_offset: Option<Vec3>,
    scaling_pivot: Option<Vec3>,
}

impl TransformBuilder {
    pub fn new() -> Self {
        Self {
            local_scale: None,
            local_position: None,
            local_rotation: None,
            pre_rotation: None,
            post_rotation: None,
            rotation_offset: None,
            rotation_pivot: None,
            scaling_offset: None,
            scaling_pivot: None
        }
    }

    pub fn with_local_scale(mut self, scale: Vec3) -> Self {
        self.local_scale = Some(scale);
        self
    }

    pub fn with_local_position(mut self, position: Vec3) -> Self {
        self.local_position = Some(position);
        self
    }

    pub fn with_local_rotation(mut self, rotation: Quat) -> Self {
        self.local_rotation = Some(rotation);
        self
    }

    pub fn with_pre_rotation(mut self, rotation: Quat) -> Self {
        self.pre_rotation = Some(rotation);
        self
    }

    pub fn with_post_rotation(mut self, rotation: Quat) -> Self {
        self.post_rotation = Some(rotation);
        self
    }

    pub fn with_rotation_offset(mut self, offset: Vec3) -> Self {
        self.rotation_offset = Some(offset);
        self
    }

    pub fn with_rotation_pivot(mut self, pivot: Vec3) -> Self {
        self.rotation_pivot = Some(pivot);
        self
    }

    pub fn with_scaling_offset(mut self, offset: Vec3) -> Self {
        self.scaling_offset = Some(offset);
        self
    }

    pub fn with_scaling_pivot(mut self, pivot: Vec3) -> Self {
        self.scaling_pivot = Some(pivot);
        self
    }

    pub fn build(self) -> Transform {
        Transform {
            dirty: Cell::new(true),
            local_scale: self.local_scale.unwrap_or(Vec3::unit()),
            local_position: self.local_position.unwrap_or(Vec3::zero()),
            local_rotation: self.local_rotation.unwrap_or(Quat::new()),
            pre_rotation: self.pre_rotation.unwrap_or(Quat::new()),
            post_rotation: self.post_rotation.unwrap_or(Quat::new()),
            rotation_offset: self.rotation_offset.unwrap_or(Vec3::zero()),
            rotation_pivot: self.rotation_pivot.unwrap_or(Vec3::zero()),
            scaling_offset: self.scaling_offset.unwrap_or(Vec3::zero()),
            scaling_pivot: self.scaling_pivot.unwrap_or(Vec3::zero()),
            matrix: Cell::new(Mat4::identity())
        }
    }
}