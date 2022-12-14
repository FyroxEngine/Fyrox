//! Layer is a separate state graph that usually animates only a part of nodes from animations. See docs of [`MachineLayer`]
//! for more info.

#![warn(missing_docs)]

use crate::{
    animation::{
        machine::{
            event::FixedEventQueue, Event, LayerMask, Parameter, ParameterContainer, PoseNode,
            State, Transition,
        },
        AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    utils::log::{Log, MessageKind},
};

/// Layer is a separate state graph. Layers mainly used to animate different parts of humanoid (but not only) characters. For
/// example there could a layer for upper body and a layer for lower body. Upper body layer could contain animations for aiming,
/// melee attacks while lower body layer could contain animations for standing, running, crouching, etc. This gives you an
/// ability to have running character that could aim or melee attack, or crouching and aiming, and so on with any combination.
/// Both layers use the same set of parameters, so a change in a parameter will affect all layers that use it.
///
/// # Example
///
/// ```rust
/// use fyrox::{
///     animation::machine::{
///         State, Transition, PoseNode, MachineLayer,
///         Parameter, PlayAnimation, PoseWeight, BlendAnimations, BlendPose
///     },
///     core::pool::Handle
/// };
///
/// // Assume that these are correct handles.
/// let idle_animation = Handle::default();
/// let walk_animation = Handle::default();
/// let aim_animation = Handle::default();
///
/// let mut root_layer = MachineLayer::new();
///
/// let aim = root_layer.add_node(PoseNode::PlayAnimation(PlayAnimation::new(aim_animation)));
/// let walk = root_layer.add_node(PoseNode::PlayAnimation(PlayAnimation::new(walk_animation)));
///
/// // Blend two animations together
/// let blend_aim_walk = root_layer.add_node(PoseNode::BlendAnimations(
///     BlendAnimations::new(vec![
///         BlendPose::new(PoseWeight::Constant(0.75), aim),
///         BlendPose::new(PoseWeight::Constant(0.25), walk)
///     ])
/// ));
///
/// let walk_state = root_layer.add_state(State::new("Walk", blend_aim_walk));
///
/// let idle = root_layer.add_node(PoseNode::PlayAnimation(PlayAnimation::new(idle_animation)));
/// let idle_state = root_layer.add_state(State::new("Idle", idle));
///
/// root_layer.add_transition(Transition::new("Walk->Idle", walk_state, idle_state, 1.0, "WalkToIdle"));
/// root_layer.add_transition(Transition::new("Idle->Walk", idle_state, walk_state, 1.0, "IdleToWalk"));
///
/// ```
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq)]
pub struct MachineLayer {
    name: String,

    weight: f32,

    mask: LayerMask,

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

    #[visit(skip)]
    #[reflect(hidden)]
    final_pose: AnimationPose,

    #[visit(skip)]
    #[reflect(hidden)]
    events: FixedEventQueue,

    #[visit(skip)]
    #[reflect(hidden)]
    debug: bool,
}

impl MachineLayer {
    /// Creates a new machine layer. See examples in [`MachineLayer`] docs.
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
            events: FixedEventQueue::new(2048),
            debug: false,
            mask: Default::default(),
        }
    }

    /// Sets new name for the layer. The name can then be used to find a layer in a parent state machine.
    #[inline]
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    /// Returns a current name of the layer.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a new node to the layer and returns its handle.
    #[inline]
    pub fn add_node(&mut self, node: PoseNode) -> Handle<PoseNode> {
        self.nodes.spawn(node)
    }

    /// Sets new entry state of the layer. Entry state will always be active on the first frame and will remain active
    /// until some transition won't change it.
    #[inline]
    pub fn set_entry_state(&mut self, entry_state: Handle<State>) {
        self.active_state = entry_state;
        self.entry_state = entry_state;
    }

    /// Returns a handle of current entry state.
    #[inline]
    pub fn entry_state(&self) -> Handle<State> {
        self.entry_state
    }

    /// Turns on/off the debug mode. Debug mode forces to log all events happening in the layer. For example when a
    /// state changes, there will be a respective message in the log.
    #[inline]
    pub fn debug(&mut self, state: bool) {
        self.debug = state;
    }

    /// Adds a new state to the layer and returns its handle.
    #[inline]
    pub fn add_state(&mut self, state: State) -> Handle<State> {
        let state = self.states.spawn(state);
        if self.active_state.is_none() {
            self.active_state = state;
        }
        state
    }

    /// Adds a new transition to the layer and returns its handle.
    #[inline]
    pub fn add_transition(&mut self, transition: Transition) -> Handle<Transition> {
        self.transitions.spawn(transition)
    }

    /// Borrows a state using its handle, panics if the handle is invalid.
    #[inline]
    pub fn get_state(&self, state: Handle<State>) -> &State {
        &self.states[state]
    }

    /// Borrows a transition using its handle, panics if the handle is invalid.
    #[inline]
    pub fn get_transition(&self, transition: Handle<Transition>) -> &Transition {
        &self.transitions[transition]
    }

    /// Tries to extract a next event from the inner event queue. You should use this method if you need to react
    /// to layer events somehow. For example you might want to do some action when `jump` state had become active.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fyrox::animation::machine::{Event, MachineLayer};
    ///
    /// let mut layer = MachineLayer::new();
    ///
    /// while let Some(event) = layer.pop_event() {
    ///     match event {
    ///         Event::StateEnter(state_handle) => {
    ///             // Occurs when a state is just entered.
    ///         }
    ///         Event::StateLeave(state_handle) => {
    ///             // Occurs when a state is just left.
    ///         }
    ///         Event::ActiveStateChanged(state_handle) => {
    ///             // Occurs when active state has changed.
    ///         }
    ///         Event::ActiveTransitionChanged(transition_handle) => {
    ///             // Occurs when active transition has changed.
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn pop_event(&mut self) -> Option<Event> {
        self.events.pop()
    }

    /// Resets layer state; deactivates all active transitions and sets active state to entry state.
    #[inline]
    pub fn reset(&mut self) {
        for transition in self.transitions.iter_mut() {
            transition.reset();
        }

        self.active_state = self.entry_state;
    }

    /// Tries to borrow a node by its handle, panics if the handle is invalid.
    #[inline]
    pub fn node(&self, handle: Handle<PoseNode>) -> &PoseNode {
        &self.nodes[handle]
    }

    /// Tries to borrow a node by its handle, panics if the handle is invalid.
    #[inline]
    pub fn node_mut(&mut self, handle: Handle<PoseNode>) -> &mut PoseNode {
        &mut self.nodes[handle]
    }

    /// Returns a reference to inner node container.
    #[inline]
    pub fn nodes(&self) -> &Pool<PoseNode> {
        &self.nodes
    }

    /// Returns a reference to inner node container.
    #[inline]
    pub fn nodes_mut(&mut self) -> &mut Pool<PoseNode> {
        &mut self.nodes
    }

    /// Returns a handle of active state. It could be used if you need to perform some action only if some
    /// state is active. For example jumping could be done only from `idle` and `run` state, and not from
    /// `crouch` and other states.
    #[inline]
    pub fn active_state(&self) -> Handle<State> {
        self.active_state
    }

    /// Returns a handle of active transition. It is not empty only while a transition is active (doing blending
    /// between states).
    #[inline]
    pub fn active_transition(&self) -> Handle<Transition> {
        self.active_transition
    }

    /// Tries to borrow a transition using its handle, panics if the handle is invalid.
    #[inline]
    pub fn transition(&self, handle: Handle<Transition>) -> &Transition {
        &self.transitions[handle]
    }

    /// Tries to borrow a transition using its handle, panics if the handle is invalid.
    #[inline]
    pub fn transition_mut(&mut self, handle: Handle<Transition>) -> &mut Transition {
        &mut self.transitions[handle]
    }

    /// Returns a reference to inner transitions container.
    #[inline]
    pub fn transitions(&self) -> &Pool<Transition> {
        &self.transitions
    }

    /// Returns a reference to inner transitions container.
    #[inline]
    pub fn transitions_mut(&mut self) -> &mut Pool<Transition> {
        &mut self.transitions
    }

    /// Tries to borrow a state using its handle, panics if the handle is invalid.
    #[inline]
    pub fn state(&self, handle: Handle<State>) -> &State {
        &self.states[handle]
    }

    /// Tries to borrow a state using its handle, panics if the handle is invalid.
    #[inline]
    pub fn state_mut(&mut self, handle: Handle<State>) -> &mut State {
        &mut self.states[handle]
    }

    /// Returns a reference to inner states container.
    #[inline]
    pub fn states(&self) -> &Pool<State> {
        &self.states
    }

    /// Returns a reference to inner states container.
    #[inline]
    pub fn states_mut(&mut self) -> &mut Pool<State> {
        &mut self.states
    }

    /// Sets layer weight. The weight will be used by parent state machine to blend into final pose. By default
    /// the weight is 1.0.
    #[inline]
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }

    /// Returns the layer weight.
    #[inline]
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Sets new layer mask. See docs of [`LayerMask`] for more info about layer masks.
    #[inline]
    pub fn set_mask(&mut self, mask: LayerMask) -> LayerMask {
        std::mem::replace(&mut self.mask, mask)
    }

    /// Returns a reference to current layer mask.
    #[inline]
    pub fn mask(&self) -> &LayerMask {
        &self.mask
    }

    #[inline]
    pub(super) fn evaluate_pose(
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
                                        self.states[self.active_state].name
                                    ),
                                );
                            }

                            self.events.push(Event::StateEnter(transition.source()));
                            if self.debug {
                                Log::writeln(
                                    MessageKind::Information,
                                    format!(
                                        "Entering state: {}",
                                        self.states[transition.source()].name
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
                                self.states[self.active_state].name
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
