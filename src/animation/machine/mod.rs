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

use fxhash::FxHashMap;
use std::sync::Arc;

use crate::{
    animation::{
        machine::{
            event::LimitedEventQueue, node::PoseNodeDefinition,
            parameter::ParameterContainerDefinition, state::StateDefinition,
            transition::TransitionDefinition,
        },
        AnimationContainer, AnimationPose,
    },
    core::{
        io::FileLoadError,
        pool::{Handle, Pool},
        visitor::VisitError,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::model::ModelLoadError,
    scene::{node::Node, Scene},
    utils::log::{Log, MessageKind},
};
pub use event::Event;
pub use node::{
    blend::{BlendAnimations, BlendAnimationsByIndex, BlendPose, IndexedBlendInput},
    play::PlayAnimation,
    EvaluatePose, PoseNode,
};
pub use parameter::{Parameter, ParameterContainer, PoseWeight};
pub use state::State;
pub use transition::Transition;

pub mod container;
pub mod event;
pub mod node;
pub mod parameter;
pub mod state;
pub mod transition;

#[derive(Default, Debug, Visit, Clone)]
pub struct Machine {
    parameters: ParameterContainer,
    nodes: Pool<PoseNode>,
    transitions: Pool<Transition>,
    states: Pool<State>,
    active_state: Handle<State>,
    entry_state: Handle<State>,
    active_transition: Handle<Transition>,

    #[visit(skip)]
    final_pose: AnimationPose,
    #[visit(skip)]
    events: LimitedEventQueue,
    #[visit(skip)]
    debug: bool,
}

#[derive(Default, Debug, Visit, Clone)]
pub struct MachineDefinition {
    pub parameters: ParameterContainerDefinition,
    pub nodes: Pool<PoseNodeDefinition>,
    pub transitions: Pool<TransitionDefinition>,
    pub states: Pool<StateDefinition>,
    pub entry_state: Handle<StateDefinition>,
}

/// An error that may occur during ABSM resource loading.
#[derive(Debug, thiserror::Error)]
pub enum MachineInstantiationError {
    /// An i/o error has occurred.
    #[error("A file load error has occurred {0:?}")]
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    #[error("An error that may occur due to version incompatibilities. {0:?}")]
    Visit(VisitError),

    /// An error that may occur during instantiation of the ABSM. It means that an external
    /// animation resource wasn't able to load correctly.
    #[error("An error that may occur during instantiation of the ABSM. {0:?}")]
    AnimationLoadError(Option<Arc<ModelLoadError>>),

    /// An animation is not valid.
    #[error("An animation is not valid.")]
    InvalidAnimation,
}

impl From<FileLoadError> for MachineInstantiationError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for MachineInstantiationError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

impl From<Option<Arc<ModelLoadError>>> for MachineInstantiationError {
    fn from(e: Option<Arc<ModelLoadError>>) -> Self {
        Self::AnimationLoadError(e)
    }
}

impl MachineDefinition {
    /// Instantiates animation blending state machine to the specified scene for a given root node.
    ///
    /// # Steps
    ///
    /// Instantiation involves multiple steps, the most important are:
    ///
    /// - Animation retargeting - it tries to retarget animation stored in PlayAnimation nodes to
    ///   a node hierarchy that starts from `root` node. The step may fail if the animation is not
    ///   suitable for the hierarchy.
    /// - ABSM instantiation - it uses ABSM definition to create a new instance of the ABSM.
    ///
    /// # Important notes
    ///
    /// The method loads multiple animation resources at once and it will fail even if one of them
    /// is faulty. Animation retargeting creates multiple animation instances in the scene, you
    /// **must** delete them manually when deleting the ABSM instance.
    ///
    /// The method is intended to be used with the ABSM resources made in the Fyroxed, any
    /// "hand-crafted" resources may contain invalid data which may cause errors during instantiation
    /// or even panic.  
    pub async fn instantiate(
        &self,
        root: Handle<Node>,
        scene: &mut Scene,
        resource_manager: ResourceManager,
    ) -> Result<Handle<Machine>, MachineInstantiationError> {
        let mut machine = Machine::new();

        // Initialize parameters.
        for definition in self.parameters.container.iter() {
            machine.set_parameter(&definition.name, definition.value);
        }

        // Instantiate nodes.
        let mut node_map = FxHashMap::default();
        for (definition_handle, node_definition) in self.nodes.pair_iter() {
            let node = match node_definition {
                PoseNodeDefinition::PlayAnimation(play_animation) => {
                    let animation = match resource_manager
                        .request_model(&play_animation.animation)
                        .await
                    {
                        Ok(animation) => *animation
                            .retarget_animations(root, scene)
                            .first()
                            .ok_or(MachineInstantiationError::InvalidAnimation)?,
                        Err(e) => {
                            Log::err(format!(
                                "Failed to load animation {} for PlayAnimation node {}. Reason: {:?}",
                                play_animation.animation,
                                definition_handle,
                                e
                            ));

                            Handle::NONE
                        }
                    };

                    PoseNode::make_play_animation(animation)
                }
                PoseNodeDefinition::BlendAnimations(blend_animations) => {
                    PoseNode::make_blend_animations(
                        blend_animations
                            .pose_sources
                            .iter()
                            .map(|p| BlendPose {
                                weight: p.weight.clone(),
                                // Will be assigned on the next stage.
                                pose_source: Default::default(),
                            })
                            .collect(),
                    )
                }
                PoseNodeDefinition::BlendAnimationsByIndex(blend_animations) => {
                    PoseNode::make_blend_animations_by_index(
                        blend_animations.index_parameter.clone(),
                        blend_animations
                            .inputs
                            .iter()
                            .map(|i| IndexedBlendInput {
                                blend_time: i.blend_time,
                                // Will be assigned on the next stage.
                                pose_source: Default::default(),
                            })
                            .collect(),
                    )
                }
            };

            let instance_handle = machine.add_node(node);

            node_map.insert(definition_handle, instance_handle);
        }

        // Link nodes.
        for (definition_handle, instance_handle) in node_map.iter() {
            let definition = &self.nodes[*definition_handle];
            let instance = machine.node_mut(*instance_handle);

            match instance {
                PoseNode::PlayAnimation(_) => {
                    // Do nothing, has no links to other nodes.
                }
                PoseNode::BlendAnimations(blend_animations) => {
                    if let PoseNodeDefinition::BlendAnimations(blend_animations_definition) =
                        definition
                    {
                        for (blend_pose, blend_pose_definition) in blend_animations
                            .pose_sources
                            .iter_mut()
                            .zip(blend_animations_definition.pose_sources.iter())
                        {
                            blend_pose.pose_source = node_map
                                .get(&blend_pose_definition.pose_source)
                                .cloned()
                                .unwrap_or_default();
                        }
                    } else {
                        unreachable!()
                    }
                }
                PoseNode::BlendAnimationsByIndex(blend_animations) => {
                    if let PoseNodeDefinition::BlendAnimationsByIndex(blend_animations_definition) =
                        definition
                    {
                        for (input, input_definition) in blend_animations
                            .inputs
                            .iter_mut()
                            .zip(blend_animations_definition.inputs.iter())
                        {
                            input.pose_source = node_map
                                .get(&input_definition.pose_source)
                                .cloned()
                                .unwrap_or_default();
                        }
                    } else {
                        unreachable!()
                    }
                }
            }
        }

        // Instantiate states.
        let mut state_map = FxHashMap::default();
        for (definition_handle, state_definition) in self.states.pair_iter() {
            let instance_handle = machine.add_state(State::new(
                state_definition.name.as_ref(),
                node_map
                    .get(&state_definition.root)
                    .cloned()
                    .unwrap_or_default(),
            ));

            state_map.insert(definition_handle, instance_handle);
        }

        // Instantiate transitions.
        for transition_definition in self.transitions.iter() {
            machine.add_transition(Transition::new(
                transition_definition.name.as_ref(),
                state_map
                    .get(&transition_definition.source)
                    .cloned()
                    .expect("There must be a respective source state!"),
                state_map
                    .get(&transition_definition.dest)
                    .cloned()
                    .expect("There must be a respective dest state!"),
                transition_definition.transition_time,
                transition_definition.rule.as_str(),
            ));
        }

        machine.set_entry_state(
            state_map
                .get(&self.entry_state)
                .cloned()
                .unwrap_or_default(),
        );

        Ok(scene.animation_machines.add(machine))
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

    pub fn nodes(&self) -> impl Iterator<Item = &PoseNode> {
        self.nodes.iter()
    }

    pub fn node_mut(&mut self, handle: Handle<PoseNode>) -> &mut PoseNode {
        &mut self.nodes[handle]
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
                    if transition.dest() == self.active_state
                        || transition.source() != self.active_state
                    {
                        continue;
                    }
                    if let Some(Parameter::Rule(active)) = self.parameters.get(transition.rule()) {
                        if *active {
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

        &self.final_pose
    }
}
