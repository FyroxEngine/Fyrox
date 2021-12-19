//! Contains all structures and methods to create and manage 3D transforms.
//!
//! `Transform` allows you to combine spatial properties into a single matrix in
//! an easy manner. It contains many methods that can be used to modify a single
//! property of a transform which then will be "baked" into the single matrix.
//!
//! # Complexity
//!
//! fyrox uses a complex transform model inherited from the [FBX transform formulae](http://download.autodesk.com/us/fbx/20112/FBX_SDK_HELP/index.html?url=WS1a9193826455f5ff1f92379812724681e696651.htm,topicNumber=d0e7429):
//!
//! `Transform = T * Roff * Rp * Rpre * R * Rpost * Rp⁻¹ * Soff * Sp * S * Sp⁻¹`
//!
//! where  
//! `T`     - Translation  
//! `Roff`  - Rotation offset  
//! `Rp`    - Rotation pivot  
//! `Rpre`  - Pre-rotation  
//! `R`     - Rotation  
//! `Rpost` - Post-rotation  
//! `Rp⁻¹`  - Inverse of the rotation pivot  
//! `Soff`  - Scaling offset  
//! `Sp`    - Scaling pivot  
//! `S`     - Scaling  
//! `Sp⁻¹`  - Inverse of the scaling pivot  
//!
//! It is very flexible, however it can be slow to computate. To solve possible
//! performance issues, fyrox tries to precache every possible component. This means
//! that we use lazy evaluation: you can setup all the required properties, and the actual
//! calculations will be delayed until you try to get the matrix from the transform. This makes
//! calculations faster, but increases the required amount of memory.
//!
//! In most cases you don't need to worry about all those properties, you need just `T`, `R`, `S` -
//! those will cover 99% of your requirements.
//!
//! Fun fact: the transform format was dictated by the use of the monster called FBX file format.
//! Some libraries (like assimp) decompose this complex formula into a set of smaller transforms
//! which contain only T R S components and then combine them to get the final result, I find
//! this approach very bug prone and it is still heavy from a computation perspective. It is much
//! easier to use it as is.
//!
//! # Decomposition
//!
//! Once the transform is baked into a matrix, it is *almost* impossible to decompose it back into
//! its initial components, thats why the engine does not provide any methods to get those
//! properties back.

use crate::{
    core::{
        algebra::{Matrix3, Matrix4, UnitQuaternion, Vector3},
        inspect::{Inspect, PropertyInfo},
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::variable::TemplateVariable,
    utils::log::{Log, MessageKind},
};
use std::{any::TypeId, cell::Cell};

/// See module docs.
#[derive(Clone, Debug)]
pub struct Transform {
    /// Indicates that some property has changed and matrix must be
    /// recalculated before use. This is some sort of lazy evaluation.
    dirty: Cell<bool>,
    local_scale: TemplateVariable<Vector3<f32>>,
    local_position: TemplateVariable<Vector3<f32>>,
    local_rotation: TemplateVariable<UnitQuaternion<f32>>,
    pre_rotation: TemplateVariable<UnitQuaternion<f32>>,
    post_rotation: TemplateVariable<UnitQuaternion<f32>>,
    rotation_offset: TemplateVariable<Vector3<f32>>,
    rotation_pivot: TemplateVariable<Vector3<f32>>,
    scaling_offset: TemplateVariable<Vector3<f32>>,
    scaling_pivot: TemplateVariable<Vector3<f32>>,
    /// Combined transform. Final result of combination of other properties.
    matrix: Cell<Matrix4<f32>>,
    post_rotation_matrix: Matrix3<f32>,
}

impl Inspect for Transform {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        vec![
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "local_scale",
                display_name: "Local Scale",

                value: &*self.local_scale,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Local scale of the transform".to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "local_position",
                display_name: "Local Position",

                value: &*self.local_position,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Local position of the transform".to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "local_rotation",
                display_name: "Local Rotation",

                value: &*self.local_rotation,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Local rotation of the transform".to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "pre_rotation",
                display_name: "Pre Rotation",

                value: &*self.pre_rotation,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Pre rotation of the transform. Applied before local rotation."
                    .to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "post_rotation",
                display_name: "Post Rotation",

                value: &*self.post_rotation,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Post rotation of the transform. Applied after local rotation."
                    .to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "rotation_offset",
                display_name: "Rotation Offset",

                value: &*self.rotation_offset,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Rotation offset of the transform.".to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "rotation_pivot",
                display_name: "Rotation Pivot",

                value: &*self.rotation_pivot,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Rotation pivot of the transform.".to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "scaling_offset",
                display_name: "Scaling Offset",

                value: &*self.scaling_offset,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Scale offset of the transform.".to_string(),
            },
            PropertyInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "scaling_pivot",
                display_name: "Scaling Pivot",

                value: &*self.scaling_pivot,
                read_only: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                description: "Scale pivot of the transform.".to_string(),
            },
        ]
    }
}

/// Helper to load old versions.
fn compatibility_visit<T: Default + Visit>(
    value: &mut TemplateVariable<T>,
    name: &str,
    visitor: &mut Visitor,
) -> VisitResult {
    if value.visit(name, visitor).is_err() {
        // Try visit inner value.
        let mut inner = T::default();
        inner.visit(name, visitor)?;
        *value = TemplateVariable::new_custom(inner);
    }
    Ok(())
}

impl Visit for Transform {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        compatibility_visit(&mut self.local_scale, "LocalScale", visitor)?;
        compatibility_visit(&mut self.local_position, "LocalPosition", visitor)?;
        compatibility_visit(&mut self.local_rotation, "LocalRotation", visitor)?;
        compatibility_visit(&mut self.pre_rotation, "PreRotation", visitor)?;
        compatibility_visit(&mut self.post_rotation, "PostRotation", visitor)?;
        compatibility_visit(&mut self.rotation_offset, "RotationOffset", visitor)?;
        compatibility_visit(&mut self.rotation_pivot, "RotationPivot", visitor)?;
        compatibility_visit(&mut self.scaling_offset, "ScalingOffset", visitor)?;
        compatibility_visit(&mut self.scaling_pivot, "ScalingPivot", visitor)?;

        if visitor.is_reading() {
            self.post_rotation_matrix =
                build_post_rotation_matrix(self.post_rotation.clone_inner());
        }

        visitor.leave_region()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

fn build_post_rotation_matrix(post_rotation: UnitQuaternion<f32>) -> Matrix3<f32> {
    post_rotation
        .to_rotation_matrix()
        .matrix()
        .try_inverse()
        .unwrap_or_else(|| {
            Log::writeln(
                MessageKind::Warning,
                "Unable to inverse post rotation matrix! Fallback to identity matrix.".to_owned(),
            );
            Matrix3::identity()
        })
}

impl Transform {
    /// Creates new transform that has no effect, in other words any vector
    /// or matrix will remain unchanged if combined with identity transform.
    pub fn identity() -> Self {
        Self {
            dirty: Cell::new(true),
            local_position: TemplateVariable::new(Vector3::default()),
            local_scale: TemplateVariable::new(Vector3::new(1.0, 1.0, 1.0)),
            local_rotation: TemplateVariable::new(UnitQuaternion::identity()),
            pre_rotation: TemplateVariable::new(UnitQuaternion::identity()),
            post_rotation: TemplateVariable::new(UnitQuaternion::identity()),
            rotation_offset: TemplateVariable::new(Vector3::default()),
            rotation_pivot: TemplateVariable::new(Vector3::default()),
            scaling_offset: TemplateVariable::new(Vector3::default()),
            scaling_pivot: TemplateVariable::new(Vector3::default()),
            matrix: Cell::new(Matrix4::identity()),
            post_rotation_matrix: Matrix3::identity(),
        }
    }

    /// Returns current position of transform.
    #[inline]
    pub fn position(&self) -> &TemplateVariable<Vector3<f32>> {
        &self.local_position
    }

    /// Sets position of transform.
    #[inline]
    pub fn set_position(&mut self, local_position: Vector3<f32>) -> &mut Self {
        if self.dirty.get() || *self.local_position != local_position {
            self.local_position.set(local_position);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current rotation quaternion of transform.
    #[inline]
    pub fn rotation(&self) -> &TemplateVariable<UnitQuaternion<f32>> {
        &self.local_rotation
    }

    /// Sets rotation of transform.
    #[inline]
    pub fn set_rotation(&mut self, local_rotation: UnitQuaternion<f32>) -> &mut Self {
        if self.dirty.get() || *self.local_rotation != local_rotation {
            self.local_rotation.set(local_rotation);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current scale factor of transform.
    #[inline]
    pub fn scale(&self) -> &TemplateVariable<Vector3<f32>> {
        &self.local_scale
    }

    /// Sets scale of transform.
    #[inline]
    pub fn set_scale(&mut self, local_scale: Vector3<f32>) -> &mut Self {
        if self.dirty.get() || *self.local_scale != local_scale {
            self.local_scale.set(local_scale);
            self.dirty.set(true);
        }
        self
    }

    /// Sets pre-rotation of transform. Usually pre-rotation can be used to change
    /// "coordinate" system of transform. It is mostly for FBX compatibility, and
    /// never used in other places of engine.
    #[inline]
    pub fn set_pre_rotation(&mut self, pre_rotation: UnitQuaternion<f32>) -> &mut Self {
        if self.dirty.get() || *self.pre_rotation != pre_rotation {
            self.pre_rotation.set(pre_rotation);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current pre-rotation of transform.
    #[inline]
    pub fn pre_rotation(&self) -> &TemplateVariable<UnitQuaternion<f32>> {
        &self.pre_rotation
    }

    /// Sets post-rotation of transform. Usually post-rotation can be used to change
    /// "coordinate" system of transform. It is mostly for FBX compatibility, and
    /// never used in other places of engine.
    #[inline]
    pub fn set_post_rotation(&mut self, post_rotation: UnitQuaternion<f32>) -> &mut Self {
        if self.dirty.get() || *self.post_rotation != post_rotation {
            self.post_rotation.set(post_rotation);
            self.post_rotation_matrix =
                build_post_rotation_matrix(self.post_rotation.clone_inner());
            self.dirty.set(true);
        }
        self
    }

    /// Returns current post-rotation of transform.
    #[inline]
    pub fn post_rotation(&self) -> &TemplateVariable<UnitQuaternion<f32>> {
        &self.post_rotation
    }

    /// Sets rotation offset of transform. Moves rotation pivot using given vector,
    /// it results in rotation being performed around rotation pivot with some offset.
    #[inline]
    pub fn set_rotation_offset(&mut self, rotation_offset: Vector3<f32>) -> &mut Self {
        if self.dirty.get() || *self.rotation_offset != rotation_offset {
            self.rotation_offset.set(rotation_offset);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current rotation offset of transform.
    #[inline]
    pub fn rotation_offset(&self) -> &TemplateVariable<Vector3<f32>> {
        &self.rotation_offset
    }

    /// Sets rotation pivot of transform. This method sets a point around which all
    /// rotations will be performed. For example it can be used to rotate a cube around
    /// its vertex.
    #[inline]
    pub fn set_rotation_pivot(&mut self, rotation_pivot: Vector3<f32>) -> &mut Self {
        if self.dirty.get() || *self.rotation_pivot != rotation_pivot {
            self.rotation_pivot.set(rotation_pivot);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current rotation pivot of transform.
    #[inline]
    pub fn rotation_pivot(&self) -> &TemplateVariable<Vector3<f32>> {
        &self.rotation_pivot
    }

    /// Sets scaling offset. Scaling offset defines offset from position of scaling
    /// pivot.
    #[inline]
    pub fn set_scaling_offset(&mut self, scaling_offset: Vector3<f32>) -> &mut Self {
        if self.dirty.get() || *self.scaling_offset != scaling_offset {
            self.scaling_offset.set(scaling_offset);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current scaling offset of transform.
    #[inline]
    pub fn scaling_offset(&self) -> &TemplateVariable<Vector3<f32>> {
        &self.scaling_offset
    }

    /// Sets scaling pivot. Scaling pivot sets a point around which scale will be
    /// performed.
    #[inline]
    pub fn set_scaling_pivot(&mut self, scaling_pivot: Vector3<f32>) -> &mut Self {
        if self.dirty.get() || *self.scaling_pivot != scaling_pivot {
            self.scaling_pivot.set(scaling_pivot);
            self.dirty.set(true);
        }
        self
    }

    /// Returns current scaling pivot of transform.
    #[inline]
    pub fn scaling_pivot(&self) -> &TemplateVariable<Vector3<f32>> {
        &self.scaling_pivot
    }

    /// Shifts local position using given vector. It is a shortcut for:
    /// set_position(position() + offset)
    #[inline]
    pub fn offset(&mut self, vec: Vector3<f32>) -> &mut Self {
        self.local_position.set(*self.local_position + vec);
        self.dirty.set(true);
        self
    }

    fn calculate_local_transform(&self) -> Matrix4<f32> {
        // Make shortcuts to remove visual clutter.
        let por = &self.post_rotation_matrix;
        let pr = *self.pre_rotation.to_rotation_matrix().matrix();
        let r = *self.local_rotation.to_rotation_matrix().matrix();

        let sx = self.local_scale.x;
        let sy = self.local_scale.y;
        let sz = self.local_scale.z;

        let tx = self.local_position.x;
        let ty = self.local_position.y;
        let tz = self.local_position.z;

        let rpx = self.rotation_pivot.x;
        let rpy = self.rotation_pivot.y;
        let rpz = self.rotation_pivot.z;

        let rox = self.rotation_offset.x;
        let roy = self.rotation_offset.y;
        let roz = self.rotation_offset.z;

        let spx = self.scaling_pivot.x;
        let spy = self.scaling_pivot.y;
        let spz = self.scaling_pivot.z;

        let sox = self.scaling_offset.x;
        let soy = self.scaling_offset.y;
        let soz = self.scaling_offset.z;

        // Optimized multiplication of these matrices:
        //
        // Transform = T * Roff * Rp * Rpre * R * Rpost * Rp⁻¹ * Soff * Sp * S * Sp⁻¹
        //
        // where
        // T     - Translation
        // Roff  - Rotation offset
        // Rp    - Rotation pivot
        // Rpre  - Pre-rotation
        // R     - Rotation
        // Rpost - Post-rotation
        // Rp⁻¹  - Inverse of the rotation pivot
        // Soff  - Scaling offset
        // Sp    - Scaling pivot
        // S     - Scaling
        // Sp⁻¹  - Inverse of the scaling pivot
        let a0 = pr[0] * r[0] + pr[3] * r[1] + pr[6] * r[2];
        let a1 = pr[1] * r[0] + pr[4] * r[1] + pr[7] * r[2];
        let a2 = pr[2] * r[0] + pr[5] * r[1] + pr[8] * r[2];
        let a3 = pr[0] * r[3] + pr[3] * r[4] + pr[6] * r[5];
        let a4 = pr[1] * r[3] + pr[4] * r[4] + pr[7] * r[5];
        let a5 = pr[2] * r[3] + pr[5] * r[4] + pr[8] * r[5];
        let a6 = pr[0] * r[6] + pr[3] * r[7] + pr[6] * r[8];
        let a7 = pr[1] * r[6] + pr[4] * r[7] + pr[7] * r[8];
        let a8 = pr[2] * r[6] + pr[5] * r[7] + pr[8] * r[8];
        let f0 = por[0] * a0 + por[1] * a3 + por[2] * a6;
        let f1 = por[0] * a1 + por[1] * a4 + por[2] * a7;
        let f2 = por[0] * a2 + por[1] * a5 + por[2] * a8;
        let f3 = por[3] * a0 + por[4] * a3 + por[5] * a6;
        let f4 = por[3] * a1 + por[4] * a4 + por[5] * a7;
        let f5 = por[3] * a2 + por[4] * a5 + por[5] * a8;
        let f6 = por[6] * a0 + por[7] * a3 + por[8] * a6;
        let f7 = por[6] * a1 + por[7] * a4 + por[8] * a7;
        let f8 = por[6] * a2 + por[7] * a5 + por[8] * a8;
        let m0 = sx * f0;
        let m1 = sx * f1;
        let m2 = sx * f2;
        let m3 = 0.0;
        let m4 = sy * f3;
        let m5 = sy * f4;
        let m6 = sy * f5;
        let m7 = 0.0;
        let m8 = sz * f6;
        let m9 = sz * f7;
        let m10 = sz * f8;
        let m11 = 0.0;
        let k0 = spx * f0;
        let k1 = spy * f3;
        let k2 = spz * f6;
        let m12 = rox + rpx + tx - rpx * f0 - rpy * f3 - rpz * f6
            + sox * f0
            + k0
            + soy * f3
            + k1
            + soz * f6
            + k2
            - sx * k0
            - sy * k1
            - sz * k2;
        let k3 = spx * f1;
        let k4 = spy * f4;
        let k5 = spz * f7;
        let m13 = roy + rpy + ty - rpx * f1 - rpy * f4 - rpz * f7
            + sox * f1
            + k3
            + soy * f4
            + k4
            + soz * f7
            + k5
            - sx * k3
            - sy * k4
            - sz * k5;
        let k6 = spx * f2;
        let k7 = spy * f5;
        let k8 = spz * f8;
        let m14 = roz + rpz + tz - rpx * f2 - rpy * f5 - rpz * f8
            + sox * f2
            + k6
            + soy * f5
            + k7
            + soz * f8
            + k8
            - sx * k6
            - sy * k7
            - sz * k8;
        let m15 = 1.0;
        Matrix4::new(
            m0, m4, m8, m12, m1, m5, m9, m13, m2, m6, m10, m14, m3, m7, m11, m15,
        )
    }

    /// Returns matrix which is final result of transform. Matrix then can be used to transform
    /// a vector, or combine with other matrix, to make transform hierarchy for example.
    pub fn matrix(&self) -> Matrix4<f32> {
        if self.dirty.get() {
            self.matrix.set(self.calculate_local_transform());
            self.dirty.set(false)
        }
        self.matrix.get()
    }
}

/// Transform builder allows you to construct transform in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct TransformBuilder {
    local_scale: Vector3<f32>,
    local_position: Vector3<f32>,
    local_rotation: UnitQuaternion<f32>,
    pre_rotation: UnitQuaternion<f32>,
    post_rotation: UnitQuaternion<f32>,
    rotation_offset: Vector3<f32>,
    rotation_pivot: Vector3<f32>,
    scaling_offset: Vector3<f32>,
    scaling_pivot: Vector3<f32>,
}

impl Default for TransformBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformBuilder {
    /// Creates new transform builder. If it won't be modified then it will produce
    /// identity transform as result.
    pub fn new() -> Self {
        Self {
            local_scale: Vector3::new(1.0, 1.0, 1.0),
            local_position: Default::default(),
            local_rotation: UnitQuaternion::identity(),
            pre_rotation: UnitQuaternion::identity(),
            post_rotation: UnitQuaternion::identity(),
            rotation_offset: Default::default(),
            rotation_pivot: Default::default(),
            scaling_offset: Default::default(),
            scaling_pivot: Default::default(),
        }
    }

    /// Sets desired local scale.
    pub fn with_local_scale(mut self, scale: Vector3<f32>) -> Self {
        self.local_scale = scale;
        self
    }

    /// Sets desired local position.
    pub fn with_local_position(mut self, position: Vector3<f32>) -> Self {
        self.local_position = position;
        self
    }

    /// Sets desired local rotation.
    pub fn with_local_rotation(mut self, rotation: UnitQuaternion<f32>) -> Self {
        self.local_rotation = rotation;
        self
    }

    /// Sets desired pre-rotation.
    pub fn with_pre_rotation(mut self, rotation: UnitQuaternion<f32>) -> Self {
        self.pre_rotation = rotation;
        self
    }

    /// Sets desired post-rotation.
    pub fn with_post_rotation(mut self, rotation: UnitQuaternion<f32>) -> Self {
        self.post_rotation = rotation;
        self
    }

    /// Sets desired rotation offset.
    pub fn with_rotation_offset(mut self, offset: Vector3<f32>) -> Self {
        self.rotation_offset = offset;
        self
    }

    /// Sets desired rotation pivot.
    pub fn with_rotation_pivot(mut self, pivot: Vector3<f32>) -> Self {
        self.rotation_pivot = pivot;
        self
    }

    /// Sets desired scaling offset.
    pub fn with_scaling_offset(mut self, offset: Vector3<f32>) -> Self {
        self.scaling_offset = offset;
        self
    }

    /// Sets desired scaling pivot.
    pub fn with_scaling_pivot(mut self, pivot: Vector3<f32>) -> Self {
        self.scaling_pivot = pivot;
        self
    }

    /// Builds new Transform instance using provided values.
    pub fn build(self) -> Transform {
        Transform {
            dirty: Cell::new(true),
            local_scale: TemplateVariable::new(self.local_scale),
            local_position: TemplateVariable::new(self.local_position),
            local_rotation: TemplateVariable::new(self.local_rotation),
            pre_rotation: TemplateVariable::new(self.pre_rotation),
            post_rotation: TemplateVariable::new(self.post_rotation),
            rotation_offset: TemplateVariable::new(self.rotation_offset),
            rotation_pivot: TemplateVariable::new(self.rotation_pivot),
            scaling_offset: TemplateVariable::new(self.scaling_offset),
            scaling_pivot: TemplateVariable::new(self.scaling_pivot),
            matrix: Cell::new(Matrix4::identity()),
            post_rotation_matrix: build_post_rotation_matrix(self.post_rotation),
        }
    }
}
