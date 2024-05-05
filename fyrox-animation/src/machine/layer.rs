//! Layer is a separate state graph that usually animates only a part of nodes from animations. See docs of [`MachineLayer`]
//! for more info.

use crate::{
    core::{
        log::{Log, MessageKind},
        pool::{Handle, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
    },
    machine::{
        event::FixedEventQueue, node::AnimationEventCollectionStrategy, AnimationPoseSource, Event,
        LayerMask, ParameterContainer, PoseNode, State, Transition,
    },
    Animation, AnimationContainer, AnimationEvent, AnimationPose, EntityId,
};
use fyrox_core::{find_by_name_mut, find_by_name_ref, NameProvider};

/// Layer is a separate state graph. Layers mainly used to animate different parts of humanoid (but not only) characters. For
/// example there could a layer for upper body and a layer for lower body. Upper body layer could contain animations for aiming,
/// melee attacks while lower body layer could contain animations for standing, running, crouching, etc. This gives you an
/// ability to have running character that could aim or melee attack, or crouching and aiming, and so on with any combination.
/// Both layers use the same set of parameters, so a change in a parameter will affect all layers that use it.
///
/// # Example
///
/// ```rust
/// use fyrox_animation::{
///     machine::{
///         State, Transition, PoseNode, MachineLayer,
///         Parameter, PlayAnimation, PoseWeight, BlendAnimations, BlendPose
///     },
///     core::pool::Handle
/// };
/// use fyrox_core::pool::ErasedHandle;
///
/// // Assume that these are correct handles.
/// let idle_animation = Handle::default();
/// let walk_animation = Handle::default();
/// let aim_animation = Handle::default();
///
/// let mut root_layer = MachineLayer::<ErasedHandle>::new();
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
pub struct MachineLayer<T: EntityId> {
    name: String,

    weight: f32,

    mask: LayerMask<T>,

    #[reflect(hidden)]
    nodes: Pool<PoseNode<T>>,

    #[reflect(hidden)]
    transitions: Pool<Transition<T>>,

    #[reflect(hidden)]
    states: Pool<State<T>>,

    #[reflect(hidden)]
    active_state: Handle<State<T>>,

    #[reflect(hidden)]
    entry_state: Handle<State<T>>,

    #[reflect(hidden)]
    active_transition: Handle<Transition<T>>,

    #[visit(skip)]
    #[reflect(hidden)]
    final_pose: AnimationPose<T>,

    #[visit(skip)]
    #[reflect(hidden)]
    events: FixedEventQueue<T>,

    #[visit(skip)]
    #[reflect(hidden)]
    debug: bool,
}

impl<T: EntityId> NameProvider for MachineLayer<T> {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A source of animation events coming from a layer.
#[derive(Default)]
pub enum AnimationEventsSource<T: EntityId> {
    /// Layer is malformed and no events were gathered.
    #[default]
    Unknown,
    /// Animation events were gathered from a state.
    State {
        /// A handle of a state, from which the events were collected.
        handle: Handle<State<T>>,
        /// A name of a state, from which the events were collected.
        name: String,
    },
    /// Animation events were gathered from both states of a transition.
    Transition {
        /// A handle of an active transition.
        handle: Handle<Transition<T>>,
        /// A handle of a source state of an active transition.
        source_state_handle: Handle<State<T>>,
        /// A handle of a destination state of an active transition.
        dest_state_handle: Handle<State<T>>,
        /// A name of a source state of an active transition.
        source_state_name: String,
        /// A name of a destination state of an active transition.
        dest_state_name: String,
    },
}

/// A collection of events gathered from an active state (or a transition between states). See docs of [`MachineLayer::collect_active_animations_events`]
/// for more info and usage examples.
#[derive(Default)]
pub struct LayerAnimationEventsCollection<T: EntityId> {
    /// A source of events.
    pub source: AnimationEventsSource<T>,
    /// Actual animation events, defined as a tuple `(animation handle, event)`.
    pub events: Vec<(Handle<Animation<T>>, AnimationEvent)>,
}

impl<T: EntityId> MachineLayer<T> {
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
        name.as_ref().clone_into(&mut self.name);
    }

    /// Returns a current name of the layer.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Adds a new node to the layer and returns its handle.
    #[inline]
    pub fn add_node(&mut self, node: PoseNode<T>) -> Handle<PoseNode<T>> {
        self.nodes.spawn(node)
    }

    /// Sets new entry state of the layer. Entry state will always be active on the first frame and will remain active
    /// until some transition won't change it.
    #[inline]
    pub fn set_entry_state(&mut self, entry_state: Handle<State<T>>) {
        self.active_state = entry_state;
        self.entry_state = entry_state;
    }

    /// Returns a handle of current entry state.
    #[inline]
    pub fn entry_state(&self) -> Handle<State<T>> {
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
    pub fn add_state(&mut self, state: State<T>) -> Handle<State<T>> {
        let state = self.states.spawn(state);
        if self.active_state.is_none() {
            self.active_state = state;
        }
        state
    }

    /// Adds a new transition to the layer and returns its handle.
    #[inline]
    pub fn add_transition(&mut self, transition: Transition<T>) -> Handle<Transition<T>> {
        self.transitions.spawn(transition)
    }

    /// Borrows a state using its handle, panics if the handle is invalid.
    #[inline]
    pub fn get_state(&self, state: Handle<State<T>>) -> &State<T> {
        &self.states[state]
    }

    /// Borrows a transition using its handle, panics if the handle is invalid.
    #[inline]
    pub fn get_transition(&self, transition: Handle<Transition<T>>) -> &Transition<T> {
        &self.transitions[transition]
    }

    /// Tries to extract a next event from the inner event queue. You should use this method if you need to react
    /// to layer events somehow. For example you might want to do some action when `jump` state had become active.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fyrox_animation::machine::{Event, MachineLayer};
    /// use fyrox_core::pool::ErasedHandle;
    ///
    /// let mut layer = MachineLayer::<ErasedHandle>::new();
    ///
    /// while let Some(event) = layer.pop_event() {
    ///     match event {
    ///         Event::StateEnter(state_handle) => {
    ///             // Occurs when a state is just entered.
    ///         }
    ///         Event::StateLeave(state_handle) => {
    ///             // Occurs when a state is just left.
    ///         }
    ///         Event::ActiveStateChanged { prev, new } => {
    ///             // Occurs when active state has changed.
    ///         }
    ///         Event::ActiveTransitionChanged(transition_handle) => {
    ///             // Occurs when active transition has changed.
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn pop_event(&mut self) -> Option<Event<T>> {
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

    /// Fetches animation events from an active state (or a transition). It could be used to fetch animation events from a layer
    /// and receive events only from active state (or transition) without a need to manually fetching the events from a dozens
    /// of animations. Additionally, it provides a way of weight filtering of events - you can pick one of
    /// [`AnimationEventCollectionStrategy`]. For example, [`AnimationEventCollectionStrategy::MaxWeight`] could be used to fetch
    /// events from the animations with the largest weight (when blending them together).
    ///
    /// ## Important notes
    ///
    /// This method does **not** remove the events from events queue of respective animations, so you need to clear the queue
    /// manually each frame.
    pub fn collect_active_animations_events(
        &self,
        params: &ParameterContainer,
        animations: &AnimationContainer<T>,
        strategy: AnimationEventCollectionStrategy,
    ) -> LayerAnimationEventsCollection<T> {
        if let Some(state) = self.states.try_borrow(self.active_state) {
            return LayerAnimationEventsCollection {
                source: AnimationEventsSource::State {
                    handle: self.active_state,
                    name: state.name.clone(),
                },
                events: self.nodes[state.root].collect_animation_events(
                    &self.nodes,
                    params,
                    animations,
                    strategy,
                ),
            };
        } else if let Some(transition) = self.transitions.try_borrow(self.active_transition) {
            if let (Some(source_state), Some(dest_state)) = (
                self.states.try_borrow(transition.source()),
                self.states.try_borrow(transition.dest()),
            ) {
                let mut events = Vec::new();
                match strategy {
                    AnimationEventCollectionStrategy::All => {
                        for state in [source_state, dest_state] {
                            if let Some(root) = self.nodes.try_borrow(state.root) {
                                events.extend(root.collect_animation_events(
                                    &self.nodes,
                                    params,
                                    animations,
                                    strategy,
                                ));
                            }
                        }
                    }
                    AnimationEventCollectionStrategy::MaxWeight => {
                        let input = if transition.blend_factor() < 0.5 {
                            source_state
                        } else {
                            dest_state
                        };

                        if let Some(pose_source) = self.nodes.try_borrow(input.root) {
                            events = pose_source.collect_animation_events(
                                &self.nodes,
                                params,
                                animations,
                                strategy,
                            );
                        }
                    }
                    AnimationEventCollectionStrategy::MinWeight => {
                        let input = if transition.blend_factor() < 0.5 {
                            dest_state
                        } else {
                            source_state
                        };

                        if let Some(pose_source) = self.nodes.try_borrow(input.root) {
                            events = pose_source.collect_animation_events(
                                &self.nodes,
                                params,
                                animations,
                                strategy,
                            );
                        }
                    }
                }

                return LayerAnimationEventsCollection {
                    source: AnimationEventsSource::Transition {
                        handle: self.active_transition,
                        source_state_handle: transition.source,
                        dest_state_handle: transition.dest,
                        source_state_name: self
                            .states
                            .try_borrow(transition.source)
                            .map(|s| s.name.clone())
                            .unwrap_or_default(),
                        dest_state_name: self
                            .states
                            .try_borrow(transition.dest)
                            .map(|s| s.name.clone())
                            .unwrap_or_default(),
                    },
                    events,
                };
            }
        }
        Default::default()
    }

    /// Tries to borrow a node by its handle, panics if the handle is invalid.
    #[inline]
    pub fn node(&self, handle: Handle<PoseNode<T>>) -> &PoseNode<T> {
        &self.nodes[handle]
    }

    /// Tries to borrow a node by its handle, panics if the handle is invalid.
    #[inline]
    pub fn node_mut(&mut self, handle: Handle<PoseNode<T>>) -> &mut PoseNode<T> {
        &mut self.nodes[handle]
    }

    /// Returns a reference to inner node container.
    #[inline]
    pub fn nodes(&self) -> &Pool<PoseNode<T>> {
        &self.nodes
    }

    /// Returns a reference to inner node container.
    #[inline]
    pub fn nodes_mut(&mut self) -> &mut Pool<PoseNode<T>> {
        &mut self.nodes
    }

    /// Returns a handle of active state. It could be used if you need to perform some action only if some
    /// state is active. For example jumping could be done only from `idle` and `run` state, and not from
    /// `crouch` and other states.
    #[inline]
    pub fn active_state(&self) -> Handle<State<T>> {
        self.active_state
    }

    /// Returns a handle of active transition. It is not empty only while a transition is active (doing blending
    /// between states).
    #[inline]
    pub fn active_transition(&self) -> Handle<Transition<T>> {
        self.active_transition
    }

    /// Tries to borrow a transition using its handle, panics if the handle is invalid.
    #[inline]
    pub fn transition(&self, handle: Handle<Transition<T>>) -> &Transition<T> {
        &self.transitions[handle]
    }

    /// Tries to borrow a transition using its handle, panics if the handle is invalid.
    #[inline]
    pub fn transition_mut(&mut self, handle: Handle<Transition<T>>) -> &mut Transition<T> {
        &mut self.transitions[handle]
    }

    /// Returns a reference to inner transitions container.
    #[inline]
    pub fn transitions(&self) -> &Pool<Transition<T>> {
        &self.transitions
    }

    /// Returns a reference to inner transitions container.
    #[inline]
    pub fn transitions_mut(&mut self) -> &mut Pool<Transition<T>> {
        &mut self.transitions
    }

    /// Tries to find a transition by its name.
    #[inline]
    pub fn find_transition_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(Handle<Transition<T>>, &Transition<T>)> {
        find_by_name_ref(self.transitions.pair_iter(), name)
    }

    /// Tries to find a transition by its name.
    #[inline]
    pub fn find_transition_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(Handle<Transition<T>>, &mut Transition<T>)> {
        find_by_name_mut(self.transitions.pair_iter_mut(), name)
    }

    /// Tries to borrow a state using its handle, panics if the handle is invalid.
    #[inline]
    pub fn state(&self, handle: Handle<State<T>>) -> &State<T> {
        &self.states[handle]
    }

    /// Tries to borrow a state using its handle, panics if the handle is invalid.
    #[inline]
    pub fn state_mut(&mut self, handle: Handle<State<T>>) -> &mut State<T> {
        &mut self.states[handle]
    }

    /// Tries to find a state by its name.
    #[inline]
    pub fn find_state_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(Handle<State<T>>, &State<T>)> {
        find_by_name_ref(self.states.pair_iter(), name)
    }

    /// Tries to find a state by its name.
    #[inline]
    pub fn find_state_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(Handle<State<T>>, &mut State<T>)> {
        find_by_name_mut(self.states.pair_iter_mut(), name)
    }

    /// Returns a reference to inner states container.
    #[inline]
    pub fn states(&self) -> &Pool<State<T>> {
        &self.states
    }

    /// Returns a reference to inner states container.
    #[inline]
    pub fn states_mut(&mut self) -> &mut Pool<State<T>> {
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
    pub fn set_mask(&mut self, mask: LayerMask<T>) -> LayerMask<T> {
        std::mem::replace(&mut self.mask, mask)
    }

    /// Returns a reference to current layer mask.
    #[inline]
    pub fn mask(&self) -> &LayerMask<T> {
        &self.mask
    }

    /// Returns final pose of the layer.
    #[inline]
    pub fn pose(&self) -> &AnimationPose<T> {
        &self.final_pose
    }

    /// Returns an iterator over all animations of a given state. It fetches the animations from [`PoseNode::PlayAnimation`]
    /// nodes and returns them. This method could be useful to extract all animations used by a particular state. For example,
    /// to listen for animation events and react to them.
    pub fn animations_of_state(
        &self,
        state: Handle<State<T>>,
    ) -> impl Iterator<Item = Handle<Animation<T>>> + '_ {
        self.nodes.iter().filter_map(move |node| {
            if node.parent_state == state {
                if let PoseNode::PlayAnimation(play_animation) = node {
                    Some(play_animation.animation)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Returns `true` if all animations of the given state has ended, `false` - otherwise.
    pub fn is_all_animations_of_state_ended(
        &self,
        state: Handle<State<T>>,
        animations: &AnimationContainer<T>,
    ) -> bool {
        self.animations_of_state(state)
            .filter_map(|a| animations.try_get(a))
            .all(|a| a.has_ended())
    }

    #[inline]
    pub(super) fn evaluate_pose(
        &mut self,
        animations: &mut AnimationContainer<T>,
        parameters: &ParameterContainer,
        dt: f32,
    ) -> &AnimationPose<T> {
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

                    if transition.condition.calculate_value(parameters, animations) {
                        if let Some(active_state) = self.states.try_borrow(self.active_state) {
                            for action in active_state.on_leave_actions.iter() {
                                action.apply(animations);
                            }
                        }

                        self.events.push(Event::StateLeave(self.active_state));
                        if self.debug {
                            Log::writeln(
                                MessageKind::Information,
                                format!("Leaving state: {}", self.states[self.active_state].name),
                            );
                        }

                        if let Some(source) = self.states.try_borrow(transition.dest()) {
                            for action in source.on_enter_actions.iter() {
                                action.apply(animations);
                            }
                        }

                        self.events.push(Event::StateEnter(transition.dest()));
                        if self.debug {
                            Log::writeln(
                                MessageKind::Information,
                                format!("Entering state: {}", self.states[transition.dest()].name),
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
                    self.events.push(Event::ActiveStateChanged {
                        prev: transition.source(),
                        new: transition.dest(),
                    });

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
            .poses_mut()
            .retain(|h, _| self.mask.should_animate(*h));

        &self.final_pose
    }
}
