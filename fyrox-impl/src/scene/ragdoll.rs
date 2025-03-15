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

//! Ragdoll is a set of rigid bodies linked with various joints, which can control a set of bones
//! of a mesh. Ragdolls are used mostly for body physics. See [`Ragdoll`] docs for more info and
//! usage examples.

use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        uuid_provider,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::BaseSceneGraph,
    scene::{
        base::{Base, BaseBuilder},
        collider::Collider,
        graph::Graph,
        node::{Node, NodeTrait, UpdateContext},
        rigidbody::{RigidBody, RigidBodyType},
    },
};
use fyrox_graph::constructor::ConstructorProvider;
use fyrox_graph::SceneGraphNode;
use std::{
    any::{type_name, Any, TypeId},
    ops::{Deref, DerefMut},
};

/// A part of ragdoll, that has a physical rigid body, a bone and zero or more children limbs.
/// Multiple limbs together forms a ragdoll.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Limb {
    /// A handle of a scene node, that is used as a bone in some other scene node (mesh).
    pub bone: Handle<Node>,
    /// A handle to a rigid body scene node.
    pub physical_bone: Handle<Node>,
    /// A set of children limbs.
    pub children: Vec<Limb>,
}

uuid_provider!(Limb = "6d5bc2f7-8acc-4b64-8e4b-65d4551150bf");

// Rust has a compiler bug `overflow evaluating the requirement` that prevents deriving this impl.
impl Reflect for Limb {
    fn source_path() -> &'static str {
        file!()
    }

    fn derived_types() -> &'static [TypeId] {
        &[]
    }

    fn query_derived_types(&self) -> &'static [TypeId] {
        Self::derived_types()
    }

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn doc(&self) -> &'static str {
        ""
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        func(&[
            {
                static METADATA: FieldMetadata = FieldMetadata {
                    name: "Bone",
                    display_name: "Bone",
                    description: "",
                    tag: "",
                    read_only: false,
                    immutable_collection: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    doc: "",
                };
                FieldRef {
                    metadata: &METADATA,
                    value: &self.bone,
                }
            },
            {
                static METADATA: FieldMetadata = FieldMetadata {
                    name: "PhysicalBone",
                    display_name: "Physical Bone",
                    description: "",
                    tag: "",
                    read_only: false,
                    immutable_collection: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    doc: "",
                };
                FieldRef {
                    metadata: &METADATA,
                    value: &self.physical_bone,
                }
            },
            {
                static METADATA: FieldMetadata = FieldMetadata {
                    name: "Children",
                    display_name: "Children",
                    description: "",
                    tag: "",
                    read_only: false,
                    immutable_collection: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    doc: "",
                };
                FieldRef {
                    metadata: &METADATA,
                    value: &self.children,
                }
            },
        ])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        func(&mut [
            {
                static METADATA: FieldMetadata = FieldMetadata {
                    name: "Bone",
                    display_name: "Bone",
                    description: "",
                    tag: "",
                    read_only: false,
                    immutable_collection: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    doc: "",
                };
                FieldMut {
                    metadata: &METADATA,
                    value: &mut self.bone,
                }
            },
            {
                static METADATA: FieldMetadata = FieldMetadata {
                    name: "PhysicalBone",
                    display_name: "Physical Bone",
                    description: "",
                    tag: "",
                    read_only: false,
                    immutable_collection: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    doc: "",
                };
                FieldMut {
                    metadata: &METADATA,
                    value: &mut self.physical_bone,
                }
            },
            {
                static METADATA: FieldMetadata = FieldMetadata {
                    name: "Children",
                    display_name: "Children",
                    description: "",
                    tag: "",
                    read_only: false,
                    immutable_collection: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    doc: "",
                };
                FieldMut {
                    metadata: &METADATA,
                    value: &mut self.children,
                }
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
    /// Iterates recursively across the entire tree of descendant limbs and does the specified action
    /// with every limb along the way.
    pub fn iterate_recursive<F>(&self, func: &mut F)
    where
        F: FnMut(&Self),
    {
        func(self);

        for child in self.children.iter() {
            child.iterate_recursive(func)
        }
    }
}

/// Ragdoll is a set of rigid bodies linked with various joints, which can control a set of bones
/// of a mesh. Ragdolls are used mostly for body physics.
///
/// ## How to create
///
/// Usually, bodies have quite complex hierarchy of bones and total count of the bones could be 30+.
/// Manual creation of such ragdoll is very tedious and counterproductive. That's why the best way
/// to create a ragdoll is to use the editor, and the ragdoll wizard in particular. However, if
/// you're brave enough you can read this code <https://github.com/FyroxEngine/Fyrox/blob/master/editor/src/utils/ragdoll.rs> -
/// it creates a ragdoll using a humanoid skeleton.  
#[derive(Clone, Reflect, Visit, Debug, Default, ComponentProvider)]
#[reflect(derived_type = "Node")]
#[visit(optional)]
pub struct Ragdoll {
    base: Base,
    /// A handle to a main rigid body of the character to which this ragdoll belongs to. If set, the
    /// ragdoll will take control over the collider and will move it together with the root limb.
    pub character_rigid_body: InheritableVariable<Handle<Node>>,
    /// A flag, that defines whether the ragdoll is active or not. Active ragdoll enables limb rigid
    /// bodies and takes control over `character_rigid_body` (if set).
    pub is_active: InheritableVariable<bool>,
    /// Root limb of the ragdoll. Usually it is hips of the body and rest of the limbs are forming
    /// the rest of the hierarchy.
    pub root_limb: InheritableVariable<Limb>,
    /// A flag, that defines whether the ragdoll will deactivate colliders when it is not active or not.
    /// This option could be useful if you want to disable physics of limbs while the ragdoll is active.
    pub deactivate_colliders: InheritableVariable<bool>,
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

impl ConstructorProvider<Node, Graph> for Ragdoll {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Ragdoll", |_| {
                RagdollBuilder::new(BaseBuilder::new().with_name("Ragdoll"))
                    .build_node()
                    .into()
            })
            .with_group("Physics")
    }
}

impl NodeTrait for Ragdoll {
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
                .and_then(|n| n.component_mut::<RigidBody>())
            {
                new_lin_vel = Some(character_rigid_body.lin_vel());
                new_ang_vel = Some(character_rigid_body.ang_vel());
            }
        }
        self.prev_enabled = *self.is_active;

        self.root_limb.iterate_recursive(&mut |limb| {
            let mbc = ctx.nodes.begin_multi_borrow();

            let mut need_update_transform = false;

            if let Ok(mut limb_body) =
                mbc.try_get_component_of_type_mut::<RigidBody>(limb.physical_bone)
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

                    if *self.deactivate_colliders {
                        for child in limb_body.children() {
                            if let Ok(mut collider) =
                                mbc.try_get_component_of_type_mut::<Collider>(*child)
                            {
                                collider.set_is_sensor(false);
                            }
                        }
                    }

                    let body_transform = limb_body.global_transform();

                    // Sync transform of the bone with respective body.
                    let bone_parent = mbc.try_get(limb.bone).unwrap().parent();
                    let transform: Matrix4<f32> = mbc
                        .try_get(bone_parent)
                        .unwrap()
                        .global_transform()
                        .try_inverse()
                        .unwrap_or_else(Matrix4::identity)
                        * body_transform;

                    mbc.try_get_mut(limb.bone)
                        .unwrap()
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

                    need_update_transform = true;
                } else {
                    limb_body.set_body_type(RigidBodyType::KinematicPositionBased);
                    limb_body.set_lin_vel(Default::default());
                    limb_body.set_ang_vel(Default::default());

                    if *self.deactivate_colliders {
                        for child in limb_body.children() {
                            if let Ok(mut collider) =
                                mbc.try_get_component_of_type_mut::<Collider>(*child)
                            {
                                collider.set_is_sensor(true);
                            }
                        }
                    }

                    let self_transform_inverse =
                        self.global_transform().try_inverse().unwrap_or_default();

                    // Sync transform of the physical body with respective bone.
                    if let Ok(bone) = mbc.try_get(limb.bone) {
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
                        limb_body
                            .local_transform_mut()
                            .set_position(position)
                            .set_rotation(rotation);
                    }
                }
            };

            drop(mbc);

            if need_update_transform {
                // Calculate transform of the descendants explicitly, so the next bones in hierarchy will have new transform
                // that can be used to calculate relative transform.
                Graph::update_hierarchical_data_recursively(
                    ctx.nodes,
                    ctx.sound_context,
                    ctx.physics,
                    ctx.physics2d,
                    limb.bone,
                );
            }
        });

        if let Some(root_limb_body) = ctx.nodes.try_borrow(self.root_limb.bone) {
            let position = root_limb_body.global_position();
            if let Some(character_rigid_body) = ctx
                .nodes
                .try_borrow_mut(*self.character_rigid_body)
                .and_then(|n| n.component_mut::<RigidBody>())
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

/// Ragdoll builder creates [`Ragdoll`] scene nodes.
pub struct RagdollBuilder {
    base_builder: BaseBuilder,
    character_rigid_body: Handle<Node>,
    is_active: bool,
    deactivate_colliders: bool,
    root_limb: Limb,
}

impl RagdollBuilder {
    /// Creates a new ragdoll builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            character_rigid_body: Default::default(),
            is_active: true,
            deactivate_colliders: false,
            root_limb: Default::default(),
        }
    }

    /// Sets the desired character rigid body.
    pub fn with_character_rigid_body(mut self, handle: Handle<Node>) -> Self {
        self.character_rigid_body = handle;
        self
    }

    /// Sets whether the ragdoll is active or not.
    pub fn with_active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Sets the desired root limb.
    pub fn with_root_limb(mut self, root_limb: Limb) -> Self {
        self.root_limb = root_limb;
        self
    }

    /// Sets whether the ragdoll should deactivate colliders of its limbs when it is not active or not.
    pub fn with_deactivate_colliders(mut self, value: bool) -> Self {
        self.deactivate_colliders = value;
        self
    }

    /// Builds the ragdoll.
    pub fn build_ragdoll(self) -> Ragdoll {
        Ragdoll {
            base: self.base_builder.build_base(),
            character_rigid_body: self.character_rigid_body.into(),
            is_active: self.is_active.into(),
            root_limb: self.root_limb.into(),
            deactivate_colliders: self.deactivate_colliders.into(),
            prev_enabled: self.is_active,
        }
    }

    /// Creates ragdoll node, but does not add it to a graph.
    pub fn build_node(self) -> Node {
        Node::new(self.build_ragdoll())
    }

    /// Creates the ragdoll node and adds it to the given graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
