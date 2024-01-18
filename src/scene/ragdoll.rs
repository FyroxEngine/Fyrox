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
use fyrox_core::uuid_provider;
use std::{
    any::{type_name, Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Limb {
    pub bone: Handle<Node>,
    pub physical_bone: Handle<Node>,
    pub children: Vec<Limb>,
}

uuid_provider!(Limb = "6d5bc2f7-8acc-4b64-8e4b-65d4551150bf");

// Rust has a compiler bug `overflow evaluating the requirement` that prevents deriving this impl.
impl Reflect for Limb {
    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        ""
    }

    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        func(&[
            FieldInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "Bone",
                display_name: "Bone",
                description: "",
                type_name: type_name::<Handle<Node>>(),
                value: &self.bone,
                reflect_value: &self.bone,
                read_only: false,
                immutable_collection: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                doc: "",
            },
            FieldInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "PhysicalBone",
                display_name: "Physical Bone",
                description: "",
                type_name: type_name::<Handle<Node>>(),
                value: &self.physical_bone,
                reflect_value: &self.physical_bone,
                read_only: false,
                immutable_collection: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                doc: "",
            },
            FieldInfo {
                owner_type_id: TypeId::of::<Self>(),
                name: "Children",
                display_name: "Children",
                description: "",
                type_name: type_name::<Vec<Limb>>(),
                value: &self.children,
                reflect_value: &self.children,
                read_only: false,
                immutable_collection: false,
                min_value: None,
                max_value: None,
                step: None,
                precision: None,
                doc: "",
            },
        ])
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        func(self)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        func(self)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        func(self)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        func(self)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let this = std::mem::replace(self, value.take()?);
        Ok(Box::new(this))
    }

    fn fields(&self, func: &mut dyn FnMut(&[&dyn Reflect])) {
        func(&[&self.bone, &self.physical_bone, &self.children])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
        func(&mut [&mut self.bone, &mut self.physical_bone, &mut self.children])
    }

    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        func(match name {
            "Bone" => Some(&self.bone),
            "PhysicalBone" => Some(&self.physical_bone),
            "Children" => Some(&self.children),
            _ => None,
        })
    }

    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        func(match name {
            "Bone" => Some(&mut self.bone),
            "PhysicalBone" => Some(&mut self.physical_bone),
            "Children" => Some(&mut self.children),
            _ => None,
        })
    }
}

impl Visit for Limb {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut guard = visitor.enter_region(name)?;

        self.bone.visit("Bone", &mut guard)?;
        self.physical_bone.visit("PhysicalBone", &mut guard)?;
        self.children.visit("Children", &mut guard)?;

        Ok(())
    }
}

impl Limb {
    fn iterate_recursive<F>(&self, func: &mut F)
    where
        F: FnMut(&Self),
    {
        func(self);

        for child in self.children.iter() {
            child.iterate_recursive(func)
        }
    }
}

#[derive(Clone, Reflect, Visit, Debug, Default)]
pub struct Ragdoll {
    base: Base,
    character_rigid_body: InheritableVariable<Handle<Node>>,
    is_active: InheritableVariable<bool>,
    root_limb: InheritableVariable<Limb>,
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

        self.root_limb.iterate_recursive(&mut |limb| {
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
                        .set_pre_rotation(UnitQuaternion::identity())
                        .set_post_rotation(UnitQuaternion::identity())
                        .set_rotation(UnitQuaternion::from_matrix_eps(
                            &transform.basis(),
                            f32::EPSILON,
                            16,
                            Default::default(),
                        ));

                    // Calculate transform of the descendants explicitly, so the next bones in hierarchy will have new transform
                    // that can be used to calculate relative transform.
                    Graph::update_hierarchical_data_recursively(
                        ctx.nodes,
                        ctx.sound_context,
                        ctx.physics,
                        ctx.physics2d,
                        limb.bone,
                    );
                } else {
                    limb_body.set_body_type(RigidBodyType::KinematicPositionBased);
                    limb_body.set_lin_vel(Default::default());
                    limb_body.set_ang_vel(Default::default());

                    let self_transform_inverse =
                        self.global_transform().try_inverse().unwrap_or_default();

                    // Sync transform of the physical body with respective bone.
                    if let Some(bone) = ctx.nodes.try_borrow(limb.bone) {
                        let relative_transform = self_transform_inverse * bone.global_transform();

                        let position = Vector3::new(
                            relative_transform[12],
                            relative_transform[13],
                            relative_transform[14],
                        );
                        let rotation = UnitQuaternion::from_matrix_eps(
                            &relative_transform.basis(),
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
        });

        if let Some(root_limb_body) = ctx.nodes.try_borrow(self.root_limb.bone) {
            let position = root_limb_body.global_position();
            if let Some(character_rigid_body) = ctx
                .nodes
                .try_borrow_mut(*self.character_rigid_body)
                .and_then(|n| n.query_component_mut::<RigidBody>())
            {
                if *self.is_active {
                    character_rigid_body.set_lin_vel(Default::default());
                    character_rigid_body.set_ang_vel(Default::default());
                    character_rigid_body
                        .local_transform_mut()
                        .set_position(position);
                    character_rigid_body.set_body_type(RigidBodyType::KinematicPositionBased);
                } else {
                    character_rigid_body.set_body_type(RigidBodyType::Dynamic);
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

    pub fn root_limb(&self) -> &Limb {
        &self.root_limb
    }

    pub fn set_root_limb(&mut self, root_limb: Limb) {
        self.root_limb.set_value_and_mark_modified(root_limb);
    }
}

pub struct RagdollBuilder {
    base_builder: BaseBuilder,
    character_rigid_body: Handle<Node>,
    is_active: bool,
    root_limb: Limb,
}

impl RagdollBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            character_rigid_body: Default::default(),
            is_active: true,
            root_limb: Default::default(),
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

    pub fn with_root_limb(mut self, root_limb: Limb) -> Self {
        self.root_limb = root_limb;
        self
    }

    pub fn build_ragdoll(self) -> Ragdoll {
        Ragdoll {
            base: self.base_builder.build_base(),
            character_rigid_body: self.character_rigid_body.into(),
            is_active: self.is_active.into(),
            root_limb: self.root_limb.into(),
            prev_enabled: self.is_active,
        }
    }

    /// Creates ragdoll node, but does not add it to a graph.
    pub fn build_node(self) -> Node {
        Node::new(self.build_ragdoll())
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
