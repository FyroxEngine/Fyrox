#![allow(missing_docs)] // TODO

use crate::{
    animation::AnimationContainer,
    core::{
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, TypeUuidProvider, UpdateContext},
    },
};
use std::ops::{Deref, DerefMut};

pub mod absm;

#[derive(Visit, Reflect, Clone, Debug)]
pub struct AnimationPlayer {
    base: Base,
    animations: InheritableVariable<AnimationContainer>,
    auto_apply: bool,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            base: Default::default(),
            animations: Default::default(),
            auto_apply: true,
        }
    }
}

impl AnimationPlayer {
    pub fn set_auto_apply(&mut self, auto_apply: bool) {
        self.auto_apply = auto_apply;
    }

    pub fn is_auto_apply(&self) -> bool {
        self.auto_apply
    }

    pub fn animations(&self) -> &InheritableVariable<AnimationContainer> {
        &self.animations
    }

    pub fn animations_mut(&mut self) -> &mut InheritableVariable<AnimationContainer> {
        &mut self.animations
    }

    pub fn set_animations(&mut self, animations: AnimationContainer) {
        self.animations.set_value_and_mark_modified(animations);
    }
}

impl TypeUuidProvider for AnimationPlayer {
    fn type_uuid() -> Uuid {
        uuid!("44d1c94e-354f-4f9a-b918-9d31c28aa16a")
    }
}

impl Deref for AnimationPlayer {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for AnimationPlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for AnimationPlayer {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, context: &mut UpdateContext) -> bool {
        self.animations.get_value_mut_silent().update_animations(
            context.nodes,
            self.auto_apply,
            context.dt,
        );
        self.base.update_lifetime(context.dt)
    }
}

pub struct AnimationPlayerBuilder {
    base_builder: BaseBuilder,
    animations: AnimationContainer,
    auto_apply: bool,
}

impl AnimationPlayerBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            animations: AnimationContainer::new(),
            auto_apply: true,
        }
    }

    pub fn with_animations(mut self, animations: AnimationContainer) -> Self {
        self.animations = animations;
        self
    }

    pub fn with_auto_apply(mut self, auto_apply: bool) -> Self {
        self.auto_apply = auto_apply;
        self
    }

    pub fn build_node(self) -> Node {
        Node::new(AnimationPlayer {
            base: self.base_builder.build_base(),
            animations: self.animations.into(),
            auto_apply: self.auto_apply,
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
