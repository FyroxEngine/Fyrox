//! Animation blending state machine.
//!
//! Machine is used to blend multiple animation as well as perform automatic "smooth transition
//! between states. Let have a quick look at simple machine graph:
//!
//! ```text
//!                                                  +-------------+
//!                                                  |  Idle Anim  |
//!                                                  +------+------+
//!                                                         |
//!           Walk Weight                                   |
//! +-----------+      +-------+           Walk->Idle Rule  |
//! | Walk Anim +------+       |                            |
//! +-----------+      |       |      +-------+         +---+---+
//!                    | Blend |      |       +-------->+       |
//!                    |       +------+ Walk  |         |  Idle |
//! +-----------+      |       |      |       +<--------+       |
//! | Aim Anim  +------+       |      +--+----+         +---+---+
//! +-----------+      +-------+         |                  ^
//!           Aim Weight                 | Idle->Walk Rule  |
//!                                      |                  |
//!                       Walk->Run Rule |    +---------+   | Run->Idle Rule
//!                                      |    |         |   |
//!                                      +--->+   Run   +---+
//!                                           |         |
//!                                           +----+----+
//!                                                |
//!                                                |
//!                                         +------+------+
//!                                         |  Run Anim   |
//!                                         +-------------+
//! ```
//!
//! Here we have Walk, Idle, Run states which uses different sources of poses:
//! - Walk - is most complicated here - it uses result of blending between
//!   Aim and Walk animations with different weights. This is useful if your
//!   character can only walk or can walk *and* aim at the same time. Desired pose
//!   determined by Walk Weight and Aim Weight parameters combination.
//! - Run and idle both directly uses animation as pose source.
//!
//! There are four transitions between three states each with its own rule. Rule
//! is just Rule parameter which can have boolean value that indicates that transition
//! should be activated.
//!
//! Example:
//!
//! ```no_run
//! use fyrox::{
//!     animation::machine::{
//!         Machine, State, Transition, PoseNode,
//!         Parameter, PlayAnimation, PoseWeight, BlendAnimations, BlendPose
//!     },
//!     core::pool::Handle
//! };
//!
//! // Assume that these are correct handles.
//! let idle_animation = Handle::default();
//! let walk_animation = Handle::default();
//! let aim_animation = Handle::default();
//!
//! let mut machine = Machine::new();
//!
//! let root_layer = &mut machine.layers_mut()[0];
//!
//! let aim = root_layer.add_node(PoseNode::PlayAnimation(PlayAnimation::new(aim_animation)));
//! let walk = root_layer.add_node(PoseNode::PlayAnimation(PlayAnimation::new(walk_animation)));
//!
//! // Blend two animations together
//! let blend_aim_walk = root_layer.add_node(PoseNode::BlendAnimations(
//!     BlendAnimations::new(vec![
//!         BlendPose::new(PoseWeight::Constant(0.75), aim),
//!         BlendPose::new(PoseWeight::Constant(0.25), walk)
//!     ])
//! ));
//!
//! let walk_state = root_layer.add_state(State::new("Walk", blend_aim_walk));
//!
//! let idle = root_layer.add_node(PoseNode::PlayAnimation(PlayAnimation::new(idle_animation)));
//! let idle_state = root_layer.add_state(State::new("Idle", idle));
//!
//! root_layer.add_transition(Transition::new("Walk->Idle", walk_state, idle_state, 1.0, "WalkToIdle"));
//! root_layer.add_transition(Transition::new("Idle->Walk", idle_state, walk_state, 1.0, "IdleToWalk"));
//!
//! ```
//!
//! You can use multiple machines to animation single model - for example one machine can be for
//! locomotion and other is for combat. This means that locomotion machine will take control over
//! lower body and combat machine will control upper body.

use crate::{
    animation::{machine::event::LimitedEventQueue, AnimationContainer, AnimationPose},
    core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::node::Node,
    utils::log::{Log, MessageKind},
};
pub use event::Event;
use fxhash::FxHashSet;
pub use node::{
    blend::{BlendAnimations, BlendAnimationsByIndex, BlendPose, IndexedBlendInput},
    play::PlayAnimation,
    EvaluatePose, PoseNode,
};
pub use parameter::{Parameter, ParameterContainer, PoseWeight};
pub use state::State;
pub use transition::Transition;

pub mod event;
pub mod node;
pub mod parameter;
pub mod state;
pub mod transition;

#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq)]
pub struct Machine {
    #[reflect(hidden)]
    parameters: ParameterContainer,

    #[reflect(hidden)]
    layers: Vec<MachineLayer>,

    #[visit(skip)]
    #[reflect(hidden)]
    final_pose: AnimationPose,
}

impl Machine {
    /// Creates a new animation blending state machine with a single animation layer.
    #[inline]
    pub fn new() -> Self {
        Self {
            parameters: Default::default(),
            layers: vec![MachineLayer::new()],
            final_pose: Default::default(),
        }
    }

    #[inline]
    pub fn set_parameter(&mut self, id: &str, new_value: Parameter) -> &mut Self {
        match self.parameters.get_mut(id) {
            Some(parameter) => {
                *parameter = new_value;
            }
            None => {
                self.parameters.add(id, new_value);
            }
        }

        self
    }

    #[inline]
    pub fn parameters(&self) -> &ParameterContainer {
        &self.parameters
    }

    #[inline]
    pub fn parameters_mut(&mut self) -> &mut ParameterContainer {
        &mut self.parameters
    }

    #[inline]
    pub fn add_layer(&mut self, layer: MachineLayer) {
        self.layers.push(layer)
    }

    #[inline]
    pub fn remove_layer(&mut self, index: usize) -> MachineLayer {
        self.layers.remove(index)
    }

    #[inline]
    pub fn insert_layer(&mut self, index: usize, layer: MachineLayer) {
        self.layers.insert(index, layer)
    }

    #[inline]
    pub fn pop_layer(&mut self) -> Option<MachineLayer> {
        self.layers.pop()
    }

    #[inline]
    pub fn layers(&self) -> &[MachineLayer] {
        &self.layers
    }

    #[inline]
    pub fn layers_mut(&mut self) -> &mut [MachineLayer] {
        &mut self.layers
    }

    #[inline]
    pub fn evaluate_pose(&mut self, animations: &AnimationContainer, dt: f32) -> &AnimationPose {
        self.final_pose.reset();

        for layer in self.layers.iter_mut() {
            let weight = layer.weight;
            let pose = layer.evaluate_pose(animations, &self.parameters, dt);

            self.final_pose.blend_with(pose, weight);
        }

        &self.final_pose
    }
}

#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq, Eq)]
pub struct LayerMask {
    #[reflect(hidden)]
    excluded_bones: FxHashSet<Handle<Node>>,
}

impl From<FxHashSet<Handle<Node>>> for LayerMask {
    fn from(map: FxHashSet<Handle<Node>>) -> Self {
        Self {
            excluded_bones: map,
        }
    }
}

impl LayerMask {
    #[inline]
    pub fn exclude_from_animation(&mut self, node: Handle<Node>) {
        self.excluded_bones.insert(node);
    }

    #[inline]
    pub fn should_animate(&self, node: Handle<Node>) -> bool {
        !self.excluded_bones.contains(&node)
    }

    #[inline]
    pub fn inner(&self) -> &FxHashSet<Handle<Node>> {
        &self.excluded_bones
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut FxHashSet<Handle<Node>> {
        &mut self.excluded_bones
    }

    #[inline]
    pub fn into_inner(self) -> FxHashSet<Handle<Node>> {
        self.excluded_bones
    }
}

#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq)]
pub struct MachineLayer {
    name: String,

    #[reflect(hidden)]
    nodes: Pool<PoseNode>,

    #[reflect(hidden)]
    transitions: Pool<Transition>,

    #[reflect(hidden)]
    states: Pool<State>,

    #[reflect(hidden)]
    active_state: Handle<State>,

    #[reflect(hidden)]
    entry_state: Handle<State>,

    #[reflect(hidden)]
    active_transition: Handle<Transition>,

    #[reflect(hidden)]
    weight: f32,

    #[reflect(hidden)]
    mask: LayerMask,

    #[visit(skip)]
    #[reflect(hidden)]
    final_pose: AnimationPose,

    #[visit(skip)]
    #[reflect(hidden)]
    events: LimitedEventQueue,

    #[visit(skip)]
    #[reflect(hidden)]
    debug: bool,
}

impl MachineLayer {
    #[inline]
    pub fn new() -> Self {
        Self {
            name: Default::default(),
            nodes: Default::default(),
            states: Default::default(),
            transitions: Default::default(),
            final_pose: Default::default(),
            active_state: Default::default(),
            entry_state: Default::default(),
            active_transition: Default::default(),
            weight: 1.0,
            events: LimitedEventQueue::new(2048),
            debug: false,
            mask: Default::default(),
        }
    }

    #[inline]
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn add_node(&mut self, node: PoseNode) -> Handle<PoseNode> {
        self.nodes.spawn(node)
    }

    #[inline]
    pub fn set_entry_state(&mut self, entry_state: Handle<State>) {
        self.active_state = entry_state;
        self.entry_state = entry_state;
    }

    #[inline]
    pub fn entry_state(&self) -> Handle<State> {
        self.entry_state
    }

    #[inline]
    pub fn debug(&mut self, state: bool) {
        self.debug = state;
    }

    #[inline]
    pub fn add_state(&mut self, state: State) -> Handle<State> {
        let state = self.states.spawn(state);
        if self.active_state.is_none() {
            self.active_state = state;
        }
        state
    }

    #[inline]
    pub fn add_transition(&mut self, transition: Transition) -> Handle<Transition> {
        self.transitions.spawn(transition)
    }

    #[inline]
    pub fn get_state(&self, state: Handle<State>) -> &State {
        &self.states[state]
    }

    #[inline]
    pub fn get_transition(&self, transition: Handle<Transition>) -> &Transition {
        &self.transitions[transition]
    }

    #[inline]
    pub fn pop_event(&mut self) -> Option<Event> {
        self.events.pop()
    }

    #[inline]
    pub fn reset(&mut self) {
        for transition in self.transitions.iter_mut() {
            transition.reset();
        }

        self.active_state = self.entry_state;
    }

    #[inline]
    pub fn node(&self, handle: Handle<PoseNode>) -> &PoseNode {
        &self.nodes[handle]
    }

    #[inline]
    pub fn node_mut(&mut self, handle: Handle<PoseNode>) -> &mut PoseNode {
        &mut self.nodes[handle]
    }

    #[inline]
    pub fn nodes(&self) -> &Pool<PoseNode> {
        &self.nodes
    }

    #[inline]
    pub fn nodes_mut(&mut self) -> &mut Pool<PoseNode> {
        &mut self.nodes
    }

    #[inline]
    pub fn active_state(&self) -> Handle<State> {
        self.active_state
    }

    #[inline]
    pub fn active_transition(&self) -> Handle<Transition> {
        self.active_transition
    }

    #[inline]
    pub fn transition(&self, handle: Handle<Transition>) -> &Transition {
        &self.transitions[handle]
    }

    #[inline]
    pub fn transition_mut(&mut self, handle: Handle<Transition>) -> &mut Transition {
        &mut self.transitions[handle]
    }

    #[inline]
    pub fn transitions(&self) -> &Pool<Transition> {
        &self.transitions
    }

    #[inline]
    pub fn transitions_mut(&mut self) -> &mut Pool<Transition> {
        &mut self.transitions
    }

    #[inline]
    pub fn state(&self, handle: Handle<State>) -> &State {
        &self.states[handle]
    }

    #[inline]
    pub fn state_mut(&mut self, handle: Handle<State>) -> &mut State {
        &mut self.states[handle]
    }

    #[inline]
    pub fn states(&self) -> &Pool<State> {
        &self.states
    }

    #[inline]
    pub fn states_mut(&mut self) -> &mut Pool<State> {
        &mut self.states
    }

    #[inline]
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }

    #[inline]
    pub fn weight(&self) -> f32 {
        self.weight
    }

    #[inline]
    pub fn set_mask(&mut self, mask: LayerMask) -> LayerMask {
        std::mem::replace(&mut self.mask, mask)
    }

    #[inline]
    pub fn mask(&self) -> &LayerMask {
        &self.mask
    }

    #[inline]
    fn evaluate_pose(
        &mut self,
        animations: &AnimationContainer,
        parameters: &ParameterContainer,
        dt: f32,
    ) -> &AnimationPose {
        self.final_pose.reset();

        if self.active_state.is_some() || self.active_transition.is_some() {
            // Gather actual poses for each state.
            for state in self.states.iter_mut() {
                state.update(&self.nodes, parameters, animations, dt);
            }

            if self.active_transition.is_none() {
                // Find transition.
                for (handle, transition) in self.transitions.pair_iter_mut() {
                    if transition.dest() == self.active_state
                        || transition.source() != self.active_state
                    {
                        continue;
                    }
                    if let Some(Parameter::Rule(mut active)) =
                        parameters.get(transition.rule()).cloned()
                    {
                        if transition.invert_rule {
                            active = !active;
                        }

                        if active {
                            self.events.push(Event::StateLeave(self.active_state));
                            if self.debug {
                                Log::writeln(
                                    MessageKind::Information,
                                    format!(
                                        "Leaving state: {}",
                                        self.states[self.active_state].name()
                                    ),
                                );
                            }

                            self.events.push(Event::StateEnter(transition.source()));
                            if self.debug {
                                Log::writeln(
                                    MessageKind::Information,
                                    format!(
                                        "Entering state: {}",
                                        self.states[transition.source()].name()
                                    ),
                                );
                            }

                            self.active_state = Handle::NONE;

                            self.active_transition = handle;
                            self.events
                                .push(Event::ActiveTransitionChanged(self.active_transition));

                            break;
                        }
                    }
                }
            }

            // Double check for active transition because we can have empty machine.
            if self.active_transition.is_some() {
                let transition = &mut self.transitions[self.active_transition];

                // Blend between source and dest states.
                if let Some(source_pose) = self.states[transition.source()].pose(&self.nodes) {
                    self.final_pose
                        .blend_with(&source_pose, 1.0 - transition.blend_factor());
                }
                if let Some(dest_pose) = self.states[transition.dest()].pose(&self.nodes) {
                    self.final_pose
                        .blend_with(&dest_pose, transition.blend_factor());
                }

                transition.update(dt);

                if transition.is_done() {
                    transition.reset();

                    self.active_transition = Handle::NONE;
                    self.events
                        .push(Event::ActiveTransitionChanged(self.active_transition));

                    self.active_state = transition.dest();
                    self.events
                        .push(Event::ActiveStateChanged(self.active_state));

                    if self.debug {
                        Log::writeln(
                            MessageKind::Information,
                            format!(
                                "Active state changed: {}",
                                self.states[self.active_state].name()
                            ),
                        );
                    }
                }
            } else {
                // We must have active state all the time when we do not have any active transition.
                // Just get pose from active state.
                if let Some(active_state_pose) = self.states[self.active_state].pose(&self.nodes) {
                    active_state_pose.clone_into(&mut self.final_pose);
                }
            }
        }

        self.final_pose
            .local_poses
            .retain(|h, _| self.mask.should_animate(*h));

        &self.final_pose
    }
}
