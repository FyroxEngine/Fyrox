//! Animation blending state machine is a node that takes multiple animations from an animation player and
//! mixes them in arbitrary way into one animation. See [`AnimationBlendingStateMachine`] docs for more info.

use crate::{
    animation::machine::Machine,
    core::{
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    scene::{
        animation::{AnimationPlayer, AnimationPoseExt},
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, UpdateContext},
        Scene,
    },
};
use std::ops::{Deref, DerefMut};

/// Animation blending state machine (ABSM) is a node that takes multiple animations from an animation player and
/// mixes them in arbitrary way into one animation. Usually, ABSMs are used to animate humanoid characters in games,
/// by blending multiple states with one or more animations. More info about state machines can be found in
/// [`Machine`] docs.
///
/// # Important notes
///
/// The node does **not** contain any animations, instead it just takes animations from an animation
/// player node and mixes them.
///
/// # Example
///
/// You should always prefer using the editor (FyroxEd) to create animation blending state machines, for many cases
/// creating machines by code is quite slow and hard to debug. The editor shows all the states, nodes, transitions and
/// helps you to quickly debug your ABSMs. However, if you need to create a state machine from code (for example, for
/// procedural animations), then the following example is for you.
///
/// ```rust
/// use fyrox::{
///     animation::machine::{Machine, PoseNode, State, Transition},
///     core::pool::Handle,
///     scene::{
///         animation::{absm::AnimationBlendingStateMachineBuilder, AnimationPlayer},
///         base::BaseBuilder,
///         graph::Graph,
///         node::Node,
///     },
/// };
///
/// fn create_walk_idle_state_machine(
///     animation_player_handle: Handle<Node>,
///     graph: &mut Graph,
/// ) -> Handle<Node> {
///     // Find idle and run animations first.
///     let animation_player = graph
///         .try_get_of_type::<AnimationPlayer>(animation_player_handle)
///         .unwrap();
///     let idle_animation = animation_player
///         .animations()
///         .find_by_name_ref("Idle")
///         .unwrap()
///         .0;
///     let run_animation = animation_player
///         .animations()
///         .find_by_name_ref("Run")
///         .unwrap()
///         .0;
///
///     // Create state machine.
///     let mut machine = Machine::new();
///
///     let root_layer = machine.layers_mut().first_mut().unwrap();
///
///     let idle_pose = root_layer.add_node(PoseNode::make_play_animation(idle_animation));
///     let idle_state = root_layer.add_state(State::new("Idle", idle_pose));
///
///     let run_pose = root_layer.add_node(PoseNode::make_play_animation(run_animation));
///     let run_state = root_layer.add_state(State::new("Idle", run_pose));
///
///     root_layer.add_transition(Transition::new(
///         "Idle -> Run",
///         idle_state,
///         run_state,
///         0.3,
///         "Run",
///     ));
///     root_layer.add_transition(Transition::new(
///         "Run -> Idle",
///         idle_state,
///         run_state,
///         0.3,
///         "Idle",
///     ));
///
///     // Make the node.
///     AnimationBlendingStateMachineBuilder::new(BaseBuilder::new())
///         .with_machine(machine)
///         .with_animation_player(animation_player_handle)
///         .build(graph)
/// }
/// ```
#[derive(Visit, Reflect, Clone, Debug, Default)]
pub struct AnimationBlendingStateMachine {
    base: Base,
    machine: InheritableVariable<Machine>,
    animation_player: InheritableVariable<Handle<Node>>,
}

impl AnimationBlendingStateMachine {
    /// Sets new state machine to the node.
    pub fn set_machine(&mut self, machine: Machine) {
        self.machine.set_value_and_mark_modified(machine);
    }

    /// Returns a reference to the state machine used by the node.
    pub fn machine(&self) -> &InheritableVariable<Machine> {
        &self.machine
    }

    /// Returns a mutable reference to the state machine used by the node.
    pub fn machine_mut(&mut self) -> &mut InheritableVariable<Machine> {
        &mut self.machine
    }

    /// Sets new animation player of the node. The animation player is a source of animations for blending, the state
    /// machine node must have the animation player specified, otherwise it won't have any effect.
    pub fn set_animation_player(&mut self, animation_player: Handle<Node>) {
        self.animation_player
            .set_value_and_mark_modified(animation_player);
    }

    /// Returns an animation player used by the node.
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

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, context: &mut UpdateContext) {
        if let Some(animation_player) = context
            .nodes
            .try_borrow_mut(*self.animation_player)
            .and_then(|n| n.query_component_mut::<AnimationPlayer>())
        {
            // Prevent animation player to apply animation to scene nodes. The animation will
            // do than instead.
            animation_player.set_auto_apply(false);

            let pose = self.machine.get_value_mut_silent().evaluate_pose(
                animation_player.animations.get_value_mut_silent(),
                context.dt,
            );

            pose.apply_internal(context.nodes);
        }
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

/// Animation blending state machine builder allows you to create state machines in declarative manner.
pub struct AnimationBlendingStateMachineBuilder {
    base_builder: BaseBuilder,
    machine: Machine,
    animation_player: Handle<Node>,
}

impl AnimationBlendingStateMachineBuilder {
    /// Creates new builder instance.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            machine: Default::default(),
            animation_player: Default::default(),
        }
    }

    /// Sets the desired state machine.
    pub fn with_machine(mut self, machine: Machine) -> Self {
        self.machine = machine;
        self
    }

    /// Sets the animation player as a source of animations.
    pub fn with_animation_player(mut self, animation_player: Handle<Node>) -> Self {
        self.animation_player = animation_player;
        self
    }

    /// Creates new node.
    pub fn build_node(self) -> Node {
        Node::new(AnimationBlendingStateMachine {
            base: self.base_builder.build_base(),
            machine: self.machine.into(),
            animation_player: self.animation_player.into(),
        })
    }

    /// Creates new node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
