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

//! Animation blending state machine is a node that takes multiple animations from an animation player and
//! mixes them in arbitrary way into one animation. See [`AnimationBlendingStateMachine`] docs for more info.

use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    scene::{
        animation::prelude::*,
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, UpdateContext},
        Scene,
    },
};
use fyrox_graph::constructor::ConstructorProvider;
use fyrox_graph::{BaseSceneGraph, SceneGraph, SceneGraphNode};
use std::ops::{Deref, DerefMut};

/// Scene specific root motion settings.
pub type RootMotionSettings = crate::generic_animation::RootMotionSettings<Handle<Node>>;
/// Scene specific animation pose node.
pub type PoseNode = crate::generic_animation::machine::PoseNode<Handle<Node>>;
/// Scene specific animation pose node.
pub type PlayAnimation = crate::generic_animation::machine::node::play::PlayAnimation<Handle<Node>>;
/// Scene specific animation blending state machine BlendAnimations node.
pub type BlendAnimations =
    crate::generic_animation::machine::node::blend::BlendAnimations<Handle<Node>>;
/// Scene specific animation blending state machine BlendAnimationsByIndex node.
pub type BlendAnimationsByIndex =
    crate::generic_animation::machine::node::blend::BlendAnimationsByIndex<Handle<Node>>;
/// Scene specific animation blending state machine BlendPose node.
pub type BlendPose = crate::generic_animation::machine::node::blend::BlendPose<Handle<Node>>;
/// Scene specific animation blending state machine IndexedBlendInput node.
pub type IndexedBlendInput =
    crate::generic_animation::machine::node::blend::IndexedBlendInput<Handle<Node>>;
/// Scene specific animation blending state machine BlendSpace node.
pub type BlendSpace = crate::generic_animation::machine::node::blendspace::BlendSpace<Handle<Node>>;
/// Scene specific animation blending state machine blend space point.
pub type BlendSpacePoint =
    crate::generic_animation::machine::node::blendspace::BlendSpacePoint<Handle<Node>>;
/// Scene specific animation blending state machine layer mask.
pub type LayerMask = crate::generic_animation::machine::mask::LayerMask<Handle<Node>>;
/// Scene specific animation blending state machine layer mask.
pub type Event = crate::generic_animation::machine::event::Event<Handle<Node>>;
/// Scene specific animation blending state machine.
pub type Machine = crate::generic_animation::machine::Machine<Handle<Node>>;
/// Scene specific animation blending state machine layer.
pub type MachineLayer = crate::generic_animation::machine::MachineLayer<Handle<Node>>;
/// Scene specific animation blending state machine transition.
pub type Transition = crate::generic_animation::machine::transition::Transition<Handle<Node>>;
/// Scene specific animation blending state machine state.
pub type State = crate::generic_animation::machine::state::State<Handle<Node>>;
/// Scene specific animation blending state machine base pose node.
pub type BasePoseNode = crate::generic_animation::machine::node::BasePoseNode<Handle<Node>>;
/// Scene specific animation blending state machine state action.
pub type StateAction = crate::generic_animation::machine::state::StateAction<Handle<Node>>;
/// Scene specific animation blending state machine state action wrapper.
pub type StateActionWrapper =
    crate::generic_animation::machine::state::StateActionWrapper<Handle<Node>>;
/// Scene specific animation blending state machine logic node.
pub type LogicNode = crate::generic_animation::machine::transition::LogicNode<Handle<Node>>;
/// Scene specific animation blending state machine And logic node.
pub type AndNode = crate::generic_animation::machine::transition::AndNode<Handle<Node>>;
/// Scene specific animation blending state machine Xor logic nde.
pub type XorNode = crate::generic_animation::machine::transition::XorNode<Handle<Node>>;
/// Scene specific animation blending state machine Or logic node.
pub type OrNode = crate::generic_animation::machine::transition::OrNode<Handle<Node>>;
/// Scene specific animation blending state machine Not logic node.
pub type NotNode = crate::generic_animation::machine::transition::NotNode<Handle<Node>>;
/// Scene specific animation blending state machine layer animation events collection.
pub type LayerAnimationEventsCollection =
    crate::generic_animation::machine::layer::LayerAnimationEventsCollection<Handle<Node>>;
/// Scene specific animation blending state machine animation events source.
pub type AnimationEventsSource =
    crate::generic_animation::machine::layer::AnimationEventsSource<Handle<Node>>;

/// Standard prelude for animation blending state machine, that contains all most commonly used types and traits.
pub mod prelude {
    pub use super::{
        AndNode, AnimationBlendingStateMachine, AnimationBlendingStateMachineBuilder,
        AnimationEventsSource, BasePoseNode, BlendAnimations, BlendAnimationsByIndex, BlendPose,
        BlendSpace, BlendSpacePoint, Event, IndexedBlendInput, LayerAnimationEventsCollection,
        LayerMask, LogicNode, Machine, MachineLayer, NotNode, OrNode, PlayAnimation, PoseNode,
        RootMotionSettings, State, StateAction, StateActionWrapper, Transition, XorNode,
    };
    pub use crate::generic_animation::machine::{
        node::AnimationEventCollectionStrategy,
        parameter::{Parameter, ParameterContainer, ParameterDefinition, PoseWeight},
    };
}

/// Extension trait for [`LayerMask`].
pub trait LayerMaskExt {
    /// Creates a layer mask for every descendant node starting from specified `root` (included). It could
    /// be useful if you have an entire node hierarchy (for example, lower part of a body) that needs to
    /// be filtered out.
    fn from_hierarchy(graph: &Graph, root: Handle<Node>) -> Self;
}

impl LayerMaskExt for LayerMask {
    fn from_hierarchy(graph: &Graph, root: Handle<Node>) -> Self {
        Self::from(
            graph
                .traverse_iter(root)
                .map(|(handle, _)| handle)
                .collect::<Vec<_>>(),
        )
    }
}

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
/// # use fyrox_impl::{
/// #     core::pool::Handle,
/// #     scene::{
/// #         animation::{absm::prelude::*, prelude::*},
/// #         base::BaseBuilder,
/// #         graph::Graph,
/// #         node::Node,
/// #     },
/// # };
/// # use fyrox_graph::SceneGraph;
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
#[derive(Visit, Reflect, Clone, Debug, Default, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct AnimationBlendingStateMachine {
    base: Base,
    #[component(include)]
    machine: InheritableVariable<Machine>,
    #[component(include)]
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

impl ConstructorProvider<Node, Graph> for AnimationBlendingStateMachine {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Animation Blending State Machine", |_| {
                let mut machine = Machine::default();

                let mut layer = MachineLayer::new();
                layer.set_name("Base Layer");

                machine.add_layer(layer);

                AnimationBlendingStateMachineBuilder::new(
                    BaseBuilder::new().with_name("Animation Blending State Machine"),
                )
                .with_machine(machine)
                .build_node()
                .into()
            })
            .with_group("Animation")
    }
}

impl NodeTrait for AnimationBlendingStateMachine {
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
            .and_then(|n| n.component_mut::<AnimationPlayer>())
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
            .and_then(|n| n.component_ref::<AnimationPlayer>())
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
