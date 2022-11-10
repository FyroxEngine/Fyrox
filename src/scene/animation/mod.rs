#![allow(missing_docs)] // TODO

use crate::{
    animation::Animation,
    core::{
        math::aabb::AxisAlignedBoundingBox,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        base::Base,
        node::{NodeTrait, TypeUuidProvider, UpdateContext},
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit, Reflect, Clone, Debug)]
pub struct AnimationPlayer {
    base: Base,
    animations: Vec<Animation>,
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
        self.base.restore_resources(resource_manager)
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, context: &mut UpdateContext) -> bool {
        for animation in self.animations.iter_mut().filter(|anim| anim.is_enabled()) {
            animation.tick(context.dt);
        }

        self.base.update_lifetime(context.dt)
    }
}
