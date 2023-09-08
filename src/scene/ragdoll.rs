#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    impl_query_component,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, UpdateContext},
        rigidbody::{RigidBody, RigidBodyType},
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Reflect, Visit, Debug, PartialEq, Default)]
pub struct Limb {
    pub bone: Handle<Node>,
    pub physical_bone: Handle<Node>,
}

#[derive(Clone, Reflect, Visit, Debug, Default)]
pub struct Ragdoll {
    base: Base,
    character_rigid_body: InheritableVariable<Handle<Node>>,
    is_active: InheritableVariable<bool>,
    limbs: InheritableVariable<Vec<Limb>>,
    hips: InheritableVariable<Handle<Node>>,
    #[reflect(hidden)]
    prev_enabled: bool,
}

impl Deref for Ragdoll {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Ragdoll {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl TypeUuidProvider for Ragdoll {
    fn type_uuid() -> Uuid {
        uuid!("f4441683-dcef-472d-9d7d-4adca4579107")
    }
}

impl NodeTrait for Ragdoll {
    impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, ctx: &mut UpdateContext) {
        // Get linear and angular velocities of the character rigid body and transfer it onto rag doll bodies when it is just activated.
        let mut new_lin_vel = None;
        let mut new_ang_vel = None;
        if *self.is_active && !self.prev_enabled {
            if let Some(character_rigid_body) = ctx
                .nodes
                .try_borrow_mut(*self.character_rigid_body)
                .and_then(|n| n.query_component_mut::<RigidBody>())
            {
                new_lin_vel = Some(character_rigid_body.lin_vel());
                new_ang_vel = Some(character_rigid_body.ang_vel());
            }
        }
        self.prev_enabled = *self.is_active;

        for limb in self.limbs.iter() {
            if let Some(limb_body) = ctx
                .nodes
                .try_borrow_mut(limb.physical_bone)
                .and_then(|n| n.query_component_mut::<RigidBody>())
            {
                if *self.is_active {
                    // Transfer linear and angular velocities to rag doll bodies.
                    if let Some(lin_vel) = new_lin_vel {
                        limb_body.set_lin_vel(lin_vel);
                    }
                    if let Some(ang_vel) = new_ang_vel {
                        limb_body.set_ang_vel(ang_vel);
                    }

                    if limb_body.body_type() != RigidBodyType::Dynamic {
                        limb_body.set_body_type(RigidBodyType::Dynamic);
                    }
                    let body_transform = limb_body.global_transform();

                    // Sync transform of the bone with respective body.
                    let bone_parent = ctx.nodes[limb.bone].parent();
                    let transform: Matrix4<f32> = ctx.nodes[bone_parent]
                        .global_transform()
                        .try_inverse()
                        .unwrap_or_else(Matrix4::identity)
                        * body_transform;

                    ctx.nodes[limb.bone]
                        .local_transform_mut()
                        .set_position(Vector3::new(transform[12], transform[13], transform[14]))
                        .set_rotation(UnitQuaternion::from_matrix_eps(
                            &transform.basis(),
                            f32::EPSILON,
                            16,
                            Default::default(),
                        ));
                } else {
                    limb_body.set_body_type(RigidBodyType::KinematicPositionBased);
                    limb_body.set_lin_vel(Default::default());
                    limb_body.set_ang_vel(Default::default());

                    // Sync transform of the physical body with respective bone.
                    if let Some(bone) = ctx.nodes.try_borrow(limb.bone) {
                        let position = bone.global_position();
                        let rotation = UnitQuaternion::from_matrix_eps(
                            &bone.global_transform().basis(),
                            f32::EPSILON,
                            16,
                            Default::default(),
                        );
                        ctx.nodes[limb.physical_bone]
                            .local_transform_mut()
                            .set_position(position)
                            .set_rotation(rotation);
                    }
                }
            }
        }

        if *self.is_active {
            if let Some(hips_body) = ctx.nodes.try_borrow(*self.hips) {
                let position = hips_body.global_position();
                if let Some(capsule) = ctx
                    .nodes
                    .try_borrow_mut(*self.character_rigid_body)
                    .and_then(|n| n.query_component_mut::<RigidBody>())
                {
                    capsule.set_lin_vel(Default::default());
                    capsule.set_ang_vel(Default::default());
                    capsule.local_transform_mut().set_position(position);
                }
            }
        }
    }
}

impl Ragdoll {
    pub fn set_active(&mut self, active: bool) {
        self.is_active.set_value_and_mark_modified(active);
    }

    pub fn is_active(&self) -> bool {
        *self.is_active
    }

    pub fn limbs(&self) -> &[Limb] {
        self.limbs.as_slice()
    }

    pub fn set_limbs(&mut self, limbs: Vec<Limb>) {
        self.limbs.set_value_and_mark_modified(limbs);
    }

    pub fn hips(&self) -> Handle<Node> {
        *self.hips
    }

    pub fn set_hips(&mut self, hips: Handle<Node>) {
        self.hips.set_value_and_mark_modified(hips);
    }
}

pub struct RagdollBuilder {
    base_builder: BaseBuilder,
    character_rigid_body: Handle<Node>,
    is_active: bool,
    limbs: Vec<Limb>,
    hips: Handle<Node>,
}

impl RagdollBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            character_rigid_body: Default::default(),
            is_active: true,
            limbs: Default::default(),
            hips: Default::default(),
        }
    }

    pub fn with_character_rigid_body(mut self, handle: Handle<Node>) -> Self {
        self.character_rigid_body = handle;
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    pub fn with_limbs(mut self, limbs: Vec<Limb>) -> Self {
        self.limbs = limbs;
        self
    }

    pub fn with_hips(mut self, hips: Handle<Node>) -> Self {
        self.hips = hips;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        let ragdoll = Ragdoll {
            base: self.base_builder.build_base(),
            character_rigid_body: self.character_rigid_body.into(),
            is_active: self.is_active.into(),
            limbs: self.limbs.into(),
            hips: self.hips.into(),
            prev_enabled: self.is_active,
        };

        graph.add_node(Node::new(ragdoll))
    }
}
