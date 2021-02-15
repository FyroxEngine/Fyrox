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
//! use rg3d::{
//!     animation::machine::{
//!         Machine, State, Transition, PoseNode, BlendPose,
//!         Parameter, PlayAnimation, PoseWeight, BlendAnimations
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
//! let aim = machine.add_node(PoseNode::PlayAnimation(PlayAnimation::new(aim_animation)));
//! let walk = machine.add_node(PoseNode::PlayAnimation(PlayAnimation::new(walk_animation)));
//!
//! // Blend two animations together
//! let blend_aim_walk = machine.add_node(PoseNode::BlendAnimations(
//!     BlendAnimations::new(vec![
//!         BlendPose::new(PoseWeight::Constant(0.75), aim),
//!         BlendPose::new(PoseWeight::Constant(0.25), walk)
//!     ])
//! ));
//!
//! let walk_state = machine.add_state(State::new("Walk", blend_aim_walk));
//!
//! let idle = machine.add_node(PoseNode::PlayAnimation(PlayAnimation::new(idle_animation)));
//! let idle_state = machine.add_state(State::new("Idle", idle));
//!
//! machine.add_transition(Transition::new("Walk->Idle", walk_state, idle_state, 1.0, "WalkToIdle"));
//! machine.add_transition(Transition::new("Idle->Walk", idle_state, walk_state, 1.0, "IdleToWalk"));
//!
//! ```
//!
//! You can use multiple machines to animation single model - for example one machine can be for
//! locomotion and other is for combat. This means that locomotion machine will take control over
//! lower body and combat machine will control upper body.

use crate::animation::machine::blend_nodes::IndexedBlendInput;
use crate::{
    animation::{
        machine::blend_nodes::{BlendAnimations, BlendAnimationsByIndex, BlendPose},
        Animation, AnimationContainer, AnimationPose,
    },
    core::{
        pool::{Handle, Pool, PoolIterator},
        visitor::{Visit, VisitResult, Visitor},
    },
    utils::log::{Log, MessageKind},
};
use std::{
    cell::{Ref, RefCell},
    collections::{HashMap, VecDeque},
};

pub mod blend_nodes;

/// Specific machine event.
pub enum Event {
    /// Occurs when enter some state. See module docs for example.
    StateEnter(Handle<State>),

    /// Occurs when leaving some state. See module docs for example.
    StateLeave(Handle<State>),

    /// Occurs when transition is done and new active state was set.
    ActiveStateChanged(Handle<State>),
}

/// Machine node that plays specified animation.
#[derive(Default)]
pub struct PlayAnimation {
    pub animation: Handle<Animation>,
    output_pose: RefCell<AnimationPose>,
}

impl PlayAnimation {
    /// Creates new PlayAnimation node with given animation handle.
    pub fn new(animation: Handle<Animation>) -> Self {
        Self {
            animation,
            output_pose: Default::default(),
        }
    }
}

impl Visit for PlayAnimation {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.animation.visit("Animation", visitor)?;

        visitor.leave_region()
    }
}

/// Machine parameter.  Machine uses various parameters for specific actions. For example
/// Rule parameter is used to check where transition from a state to state is possible.
/// See module docs for example.
#[derive(Copy, Clone)]
pub enum Parameter {
    /// Weight parameter is used to control blend weight in BlendAnimation node.
    Weight(f32),

    /// Rule parameter is used to check where transition from a state to state is possible.
    Rule(bool),

    /// An index of pose.
    Index(u32),
}

impl Default for Parameter {
    fn default() -> Self {
        Self::Weight(0.0)
    }
}

impl Parameter {
    fn from_id(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Weight(0.0)),
            1 => Ok(Self::Rule(false)),
            2 => Ok(Self::Index(0)),
            _ => Err(format!("Invalid parameter id {}", id)),
        }
    }

    fn id(self) -> i32 {
        match self {
            Self::Weight(_) => 0,
            Self::Rule(_) => 1,
            Self::Index(_) => 2,
        }
    }
}

impl Visit for Parameter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }

        match self {
            Self::Weight(weight) => weight.visit("Value", visitor)?,
            Self::Rule(rule) => rule.visit("Value", visitor)?,
            Self::Index(index) => index.visit("Value", visitor)?,
        }

        visitor.leave_region()
    }
}

/// Specific animation pose weight.
pub enum PoseWeight {
    /// Fixed scalar value. Should not be negative (can't even realize what will happen
    /// with negative weight here)
    Constant(f32),

    /// Reference to Weight parameter with given name.
    Parameter(String),
}

impl Default for PoseWeight {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl PoseWeight {
    fn from_id(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Parameter(Default::default())),
            1 => Ok(Self::Constant(0.0)),
            _ => Err(format!("Invalid pose weight id {}", id)),
        }
    }

    fn id(&self) -> i32 {
        match self {
            Self::Parameter(_) => 0,
            Self::Constant(_) => 1,
        }
    }
}

impl Visit for PoseWeight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }

        match self {
            PoseWeight::Constant(constant) => constant.visit("Value", visitor)?,
            PoseWeight::Parameter(param_id) => param_id.visit("ParamId", visitor)?,
        }

        visitor.leave_region()
    }
}

/// Specialized node that provides animation pose. See documentation for each variant.
pub enum PoseNode {
    /// See docs for `PlayAnimation`.
    PlayAnimation(PlayAnimation),

    /// See docs for `BlendAnimations`.
    BlendAnimations(BlendAnimations),

    /// See docs for `BlendAnimationsByIndex`.
    BlendAnimationsByIndex(BlendAnimationsByIndex),
}

impl Default for PoseNode {
    fn default() -> Self {
        Self::PlayAnimation(Default::default())
    }
}

impl PoseNode {
    /// Creates new node that plays animation.
    pub fn make_play_animation(animation: Handle<Animation>) -> Self {
        Self::PlayAnimation(PlayAnimation::new(animation))
    }

    /// Creates new node that blends multiple poses.
    pub fn make_blend_animations(poses: Vec<BlendPose>) -> Self {
        Self::BlendAnimations(BlendAnimations::new(poses))
    }

    /// Creates new node that blends multiple poses.
    pub fn make_blend_animations_by_index(
        index_parameter: String,
        inputs: Vec<IndexedBlendInput>,
    ) -> Self {
        Self::BlendAnimationsByIndex(BlendAnimationsByIndex::new(index_parameter, inputs))
    }

    fn from_id(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::PlayAnimation(Default::default())),
            1 => Ok(Self::BlendAnimations(Default::default())),
            2 => Ok(Self::BlendAnimationsByIndex(Default::default())),
            _ => Err(format!("Invalid pose node id {}", id)),
        }
    }

    fn id(&self) -> i32 {
        match self {
            Self::PlayAnimation(_) => 0,
            Self::BlendAnimations(_) => 1,
            Self::BlendAnimationsByIndex(_) => 2,
        }
    }
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            PoseNode::PlayAnimation(v) => v.$func($($args),*),
            PoseNode::BlendAnimations(v) => v.$func($($args),*),
            PoseNode::BlendAnimationsByIndex(v) => v.$func($($args),*),
        }
    };
}

impl Visit for PoseNode {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind_id = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = PoseNode::from_id(kind_id)?;
        }

        static_dispatch!(self, visit, name, visitor)
    }
}

/// State is a
#[derive(Default)]
pub struct State {
    name: String,
    root: Handle<PoseNode>,
    pose: AnimationPose,
}

type ParameterContainer = HashMap<String, Parameter>;

trait EvaluatePose {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose>;
}

impl EvaluatePose for PlayAnimation {
    fn eval_pose(
        &self,
        _nodes: &Pool<PoseNode>,
        _params: &ParameterContainer,
        animations: &AnimationContainer,
        _dt: f32,
    ) -> Ref<AnimationPose> {
        animations
            .get(self.animation)
            .get_pose()
            .clone_into(&mut self.output_pose.borrow_mut());
        self.output_pose.borrow()
    }
}

impl EvaluatePose for PoseNode {
    fn eval_pose(
        &self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) -> Ref<AnimationPose> {
        static_dispatch!(self, eval_pose, nodes, params, animations, dt)
    }
}

impl State {
    /// Creates new instance of state with a given pose.
    pub fn new(name: &str, root: Handle<PoseNode>) -> Self {
        Self {
            name: name.to_owned(),
            root,
            pose: Default::default(),
        }
    }

    fn update(
        &mut self,
        nodes: &Pool<PoseNode>,
        params: &ParameterContainer,
        animations: &AnimationContainer,
        dt: f32,
    ) {
        self.pose.reset();
        nodes
            .borrow(self.root)
            .eval_pose(nodes, params, animations, dt)
            .clone_into(&mut self.pose);
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Visit for State {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.root.visit("Root", visitor)?;

        visitor.leave_region()
    }
}

/// Transition is a connection between two states with a rule that defines possibility
/// of actual transition with blending.
#[derive(Default)]
pub struct Transition {
    name: String,
    /// Total amount of time to transition from `src` to `dst` state.
    transition_time: f32,
    elapsed_time: f32,
    source: Handle<State>,
    dest: Handle<State>,
    /// Identifier of Rule parameter which defines is transition should be activated or not.
    rule: String,
    /// 0 - evaluates `src` pose, 1 - `dest`, 0..1 - blends `src` and `dest`
    blend_factor: f32,
}

impl Visit for Transition {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.transition_time.visit("TransitionTime", visitor)?;
        self.elapsed_time.visit("ElapsedTime", visitor)?;
        self.source.visit("Source", visitor)?;
        self.dest.visit("Dest", visitor)?;
        self.rule.visit("Rule", visitor)?;
        self.blend_factor.visit("BlendFactor", visitor)?;

        visitor.leave_region()
    }
}

impl Transition {
    pub fn new(
        name: &str,
        src: Handle<State>,
        dest: Handle<State>,
        time: f32,
        rule: &str,
    ) -> Transition {
        Self {
            name: name.to_owned(),
            transition_time: time,
            elapsed_time: 0.0,
            source: src,
            dest,
            rule: rule.to_owned(),
            blend_factor: 0.0,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn transition_time(&self) -> f32 {
        self.transition_time
    }

    pub fn source(&self) -> Handle<State> {
        self.source
    }

    pub fn dest(&self) -> Handle<State> {
        self.dest
    }

    pub fn rule(&self) -> &str {
        self.rule.as_str()
    }

    fn reset(&mut self) {
        self.elapsed_time = 0.0;
        self.blend_factor = 0.0;
    }

    fn update(&mut self, dt: f32) {
        self.elapsed_time += dt;
        if self.elapsed_time > self.transition_time {
            self.elapsed_time = self.transition_time;
        }
        self.blend_factor = self.elapsed_time / self.transition_time;
    }

    pub fn is_done(&self) -> bool {
        (self.transition_time - self.elapsed_time).abs() <= std::f32::EPSILON
    }
}

#[derive(Default)]
pub struct Machine {
    nodes: Pool<PoseNode>,
    states: Pool<State>,
    transitions: Pool<Transition>,
    final_pose: AnimationPose,
    active_state: Handle<State>,
    entry_state: Handle<State>,
    active_transition: Handle<Transition>,
    parameters: ParameterContainer,
    events: LimitedEventQueue,
    debug: bool,
}

struct LimitedEventQueue {
    queue: VecDeque<Event>,
    limit: u32,
}

impl Default for LimitedEventQueue {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            limit: std::u32::MAX,
        }
    }
}

impl LimitedEventQueue {
    fn new(limit: u32) -> Self {
        Self {
            queue: VecDeque::with_capacity(limit as usize),
            limit,
        }
    }

    fn push(&mut self, event: Event) {
        if self.queue.len() < (self.limit as usize) {
            self.queue.push_back(event);
        }
    }

    fn pop(&mut self) -> Option<Event> {
        self.queue.pop_front()
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
            states: Default::default(),
            transitions: Default::default(),
            final_pose: Default::default(),
            active_state: Default::default(),
            entry_state: Default::default(),
            active_transition: Default::default(),
            parameters: Default::default(),
            events: LimitedEventQueue::new(2048),
            debug: false,
        }
    }

    pub fn add_node(&mut self, node: PoseNode) -> Handle<PoseNode> {
        self.nodes.spawn(node)
    }

    pub fn set_parameter(&mut self, id: &str, new_value: Parameter) -> &mut Self {
        match self.parameters.get_mut(id) {
            Some(parameter) => {
                *parameter = new_value;
            }
            None => {
                self.parameters.insert(id.to_owned(), new_value);
            }
        }

        self
    }

    pub fn set_entry_state(&mut self, entry_state: Handle<State>) {
        self.active_state = entry_state;
        self.entry_state = entry_state;
    }

    pub fn debug(&mut self, state: bool) {
        self.debug = state;
    }

    pub fn add_state(&mut self, state: State) -> Handle<State> {
        let state = self.states.spawn(state);
        if self.active_state.is_none() {
            self.active_state = state;
        }
        state
    }

    pub fn add_transition(&mut self, transition: Transition) -> Handle<Transition> {
        self.transitions.spawn(transition)
    }

    pub fn get_state(&self, state: Handle<State>) -> &State {
        &self.states[state]
    }

    pub fn get_transition(&self, transition: Handle<Transition>) -> &Transition {
        &self.transitions[transition]
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        self.events.pop()
    }

    pub fn reset(&mut self) {
        for transition in self.transitions.iter_mut() {
            transition.reset();
        }

        self.active_state = self.entry_state;
    }

    pub fn nodes(&self) -> PoolIterator<PoseNode> {
        self.nodes.iter()
    }

    pub fn active_state(&self) -> Handle<State> {
        self.active_state
    }

    pub fn active_transition(&self) -> Handle<Transition> {
        self.active_transition
    }

    pub fn transitions(&self) -> &Pool<Transition> {
        &self.transitions
    }

    pub fn evaluate_pose(&mut self, animations: &AnimationContainer, dt: f32) -> &AnimationPose {
        self.final_pose.reset();

        if self.active_state.is_some() || self.active_transition.is_some() {
            // Gather actual poses for each state.
            for state in self.states.iter_mut() {
                state.update(&self.nodes, &self.parameters, animations, dt);
            }

            if self.active_transition.is_none() {
                // Find transition.
                for (handle, transition) in self.transitions.pair_iter_mut() {
                    if transition.dest == self.active_state
                        || transition.source != self.active_state
                    {
                        continue;
                    }
                    if let Some(Parameter::Rule(active)) = self.parameters.get(&transition.rule) {
                        if *active {
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

                            self.events.push(Event::StateEnter(transition.source));
                            if self.debug {
                                Log::writeln(
                                    MessageKind::Information,
                                    format!(
                                        "Entering state: {}",
                                        self.states[transition.source].name
                                    ),
                                );
                            }

                            self.active_state = Handle::NONE;
                            self.active_transition = handle;

                            break;
                        }
                    }
                }
            }

            // Double check for active transition because we can have empty machine.
            if self.active_transition.is_some() {
                let transition = &mut self.transitions[self.active_transition];

                // Blend between source and dest states.
                self.final_pose.blend_with(
                    &self.states[transition.source].pose,
                    1.0 - transition.blend_factor,
                );
                self.final_pose
                    .blend_with(&self.states[transition.dest].pose, transition.blend_factor);

                transition.update(dt);

                if transition.is_done() {
                    transition.reset();
                    self.active_transition = Handle::NONE;
                    self.active_state = transition.dest;
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
                self.states[self.active_state]
                    .pose
                    .clone_into(&mut self.final_pose);
            }
        }

        &self.final_pose
    }
}

impl Visit for Machine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.parameters.visit("Parameters", visitor)?;
        self.nodes.visit("Nodes", visitor)?;
        self.transitions.visit("Transitions", visitor)?;
        self.states.visit("States", visitor)?;
        self.active_state.visit("ActiveState", visitor)?;
        self.entry_state.visit("EntryState", visitor)?;
        self.active_transition.visit("ActiveTransition", visitor)?;

        visitor.leave_region()
    }
}
