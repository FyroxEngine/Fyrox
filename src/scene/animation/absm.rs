#![allow(missing_docs)] // TODO

use crate::{
    animation::machine::Machine,
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
        animation::AnimationPlayer,
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, TypeUuidProvider, UpdateContext},
        Scene,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit, Reflect, Clone, Debug, Default)]
pub struct AnimationBlendingStateMachine {
    base: Base,
    machine: InheritableVariable<Machine>,
    animation_player: InheritableVariable<Handle<Node>>,
}

impl AnimationBlendingStateMachine {
    pub fn set_machine(&mut self, machine: Machine) {
        self.machine.set_value_and_mark_modified(machine);
    }

    pub fn machine(&self) -> &InheritableVariable<Machine> {
        &self.machine
    }

    pub fn machine_mut(&mut self) -> &mut InheritableVariable<Machine> {
        &mut self.machine
    }

    pub fn set_animation_player(&mut self, animation_player: Handle<Node>) {
        self.animation_player
            .set_value_and_mark_modified(animation_player);
    }

    pub fn animation_player(&self) -> Handle<Node> {
        *self.animation_player
    }
}

impl TypeUuidProvider for AnimationBlendingStateMachine {
    fn type_uuid() -> Uuid {
        uuid!("4b08c753-2a10-41e3-8fb2-4fd0517e86bc")
    }
}

impl Deref for AnimationBlendingStateMachine {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for AnimationBlendingStateMachine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for AnimationBlendingStateMachine {
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
        if let Some(animation_player) = context
            .nodes
            .try_borrow_mut(*self.animation_player)
            .and_then(|n| n.query_component_mut::<AnimationPlayer>())
        {
            // Prevent animation player to apply animation to scene nodes. The animation will
            // do than instead.
            animation_player.set_auto_apply(false);

            let pose = self
                .machine
                .get_value_mut_silent()
                .evaluate_pose(&animation_player.animations, context.dt);

            pose.apply_internal(context.nodes);
        }
        self.base.update_lifetime(context.dt)
    }

    fn validate(&self, scene: &Scene) -> Result<(), String> {
        if scene
            .graph
            .try_get(*self.animation_player)
            .and_then(|n| n.query_component_ref::<AnimationPlayer>())
            .is_none()
        {
            Err(
                "Animation player is not set or invalid! Animation blending state \
            machine won't operate! Set the animation player handle in the Inspector."
                    .to_string(),
            )
        } else {
            Ok(())
        }
    }
}

pub struct AnimationBlendingStateMachineBuilder {
    base_builder: BaseBuilder,
    machine: Machine,
    animation_player: Handle<Node>,
}

impl AnimationBlendingStateMachineBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            machine: Default::default(),
            animation_player: Default::default(),
        }
    }

    pub fn with_machine(mut self, machine: Machine) -> Self {
        self.machine = machine;
        self
    }

    pub fn with_animation_player(mut self, animation_player: Handle<Node>) -> Self {
        self.animation_player = animation_player;
        self
    }

    pub fn build_node(self) -> Node {
        Node::new(AnimationBlendingStateMachine {
            base: self.base_builder.build_base(),
            machine: self.machine.into(),
            animation_player: self.animation_player.into(),
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
