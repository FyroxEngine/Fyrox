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
//! // A handle to root node of animated object.
//! let root = Handle::default();
//!
//! let mut machine = Machine::new(root);
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
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crate::animation::{Animation, AnimationHolder};
use crate::resource::animation::AnimationResource;
use crate::resource::model::Model;
use crate::{
    animation::{
        machine::{
            event::LimitedEventQueue,
            node::{BasePoseNode, PoseNodeDefinition},
            parameter::ParameterContainerDefinition,
            state::StateDefinition,
            transition::TransitionDefinition,
        },
        AnimationContainer, AnimationPose,
    },
    core::futures::future::join_all,
    core::{
        io::FileLoadError,
        pool::{Handle, Pool},
        visitor::VisitError,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::{absm::AbsmResource, model::ModelLoadError},
    scene::{graph::Graph, node::Node, Scene},
    utils::log::{Log, MessageKind},
};
pub use event::Event;
use fyrox_resource::ResourceState;
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
    pub(crate) root: Handle<Node>,
    pub(crate) resource: Option<AbsmResource>,
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
#[derive(Debug)]
pub enum MachineInstantiationError {
    /// An i/o error has occurred.
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),

    /// An error that may occur during instantiation of the ABSM. It means that an external
    /// animation resource wasn't able to load correctly.
    AnimationLoadError(Option<Arc<ModelLoadError>>),

    /// An animation is not valid.
    InvalidAnimation,
}

impl Display for MachineInstantiationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineInstantiationError::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            MachineInstantiationError::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
            MachineInstantiationError::AnimationLoadError(v) => {
                write!(
                    f,
                    "An error that may occur during instantiation of the ABSM. {v:?}"
                )
            }
            MachineInstantiationError::InvalidAnimation => {
                write!(f, "An animation is not valid.")
            }
        }
    }
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

fn instantiate_animation_holder(
    holder: &AnimationHolder,
    root: Handle<Node>,
    graph: &Graph,
    animations: &mut AnimationContainer,
) -> Handle<Animation> {
    match holder {
        AnimationHolder::Model(model) => {
            if let Some(model) = model {
                model
                    .retarget_animations_internal(root, graph, animations)
                    .first()
                    .cloned()
                    .unwrap_or_default()
            } else {
                Handle::NONE
            }
        }
        AnimationHolder::Animation(animation) => {
            if let Some(animation) = animation {
                animation.instantiate(root, graph, animations)
            } else {
                Handle::NONE
            }
        }
    }
}

fn instantiate_node(
    node_definition: &PoseNodeDefinition,
    definition_handle: Handle<PoseNodeDefinition>,
    animations_pack: &AnimationsPack,
    root: Handle<Node>,
    graph: &Graph,
    animations: &mut AnimationContainer,
) -> Result<PoseNode, MachineInstantiationError> {
    let mut node = match node_definition {
        PoseNodeDefinition::PlayAnimation(play_animation) => {
            let animation =
                if let Some(resource) = animations_pack.sources().get(&play_animation.animation) {
                    instantiate_animation_holder(resource, root, graph, animations)
                } else {
                    Handle::NONE
                };

            if let Some(animation) = animations.try_get_mut(animation) {
                animation
                    .set_speed(play_animation.speed)
                    .set_time_slice(play_animation.time_slice.clone().map(|s| s.0));
            }

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

    node.definition = definition_handle;

    Ok(node)
}

pub struct AnimationsPack {
    sources: FxHashMap<String, AnimationHolder>,
}

impl AnimationsPack {
    pub async fn load(paths: &[String], resource_manager: ResourceManager) -> Self {
        let models = paths
            .iter()
            .map(|path| (path.clone(), resource_manager.request_model(path)))
            .collect::<FxHashMap<_, Model>>();

        let animations = paths
            .iter()
            .map(|path| (path.clone(), resource_manager.request_animation(path)))
            .collect::<FxHashMap<_, AnimationResource>>();

        join_all(animations.values().cloned()).await;
        join_all(models.values().cloned()).await;

        let mut sources = FxHashMap::default();

        for (path, animation) in animations {
            if matches!(*animation.state(), ResourceState::Ok(_)) {
                sources.insert(path, AnimationHolder::Animation(Some(animation)));
            }
        }

        for (path, model) in models {
            if matches!(*model.state(), ResourceState::Ok(_)) {
                sources.insert(path, AnimationHolder::Model(Some(model)));
            }
        }

        Self { sources }
    }

    pub fn sources(&self) -> &FxHashMap<String, AnimationHolder> {
        &self.sources
    }
}

impl MachineDefinition {
    pub(crate) fn collect_animation_paths(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|node| {
                if let PoseNodeDefinition::PlayAnimation(play_animation) = node {
                    Some(play_animation.animation.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub async fn animations(&self, resource_manager: ResourceManager) -> AnimationsPack {
        AnimationsPack::load(&self.collect_animation_paths(), resource_manager).await
    }

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
    pub(crate) fn instantiate(
        &self,
        root: Handle<Node>,
        scene: &mut Scene,
        animations: AnimationsPack,
    ) -> Result<Handle<Machine>, MachineInstantiationError> {
        let mut machine = Machine::new(root);

        // Initialize parameters.
        for definition in self.parameters.container.iter() {
            machine.set_parameter(&definition.name, definition.value);
        }

        // Instantiate nodes.
        let mut node_map = FxHashMap::default();
        for (definition_handle, node_definition) in self.nodes.pair_iter() {
            let instance_handle = machine.add_node(instantiate_node(
                node_definition,
                definition_handle,
                &animations,
                root,
                &scene.graph,
                &mut scene.animations,
            )?);

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
            let mut state = State::new(
                state_definition.name.as_ref(),
                node_map
                    .get(&state_definition.root)
                    .cloned()
                    .unwrap_or_default(),
            );

            state.definition = definition_handle;

            let instance_handle = machine.add_state(state);

            state_map.insert(definition_handle, instance_handle);
        }

        // Instantiate transitions.
        for (transition_definition_handle, transition_definition) in self.transitions.pair_iter() {
            machine.add_transition(Transition {
                definition: transition_definition_handle,
                name: transition_definition.name.clone(),
                transition_time: transition_definition.transition_time,
                elapsed_time: 0.0,
                source: state_map
                    .get(&transition_definition.source)
                    .cloned()
                    .expect("There must be a respective source state!"),
                dest: state_map
                    .get(&transition_definition.dest)
                    .cloned()
                    .expect("There must be a respective dest state!"),
                rule: transition_definition.rule.clone(),
                invert_rule: transition_definition.invert_rule,
                blend_factor: 0.0,
            });
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

fn find_state_by_definition(
    states: &Pool<State>,
    definition: Handle<StateDefinition>,
) -> Handle<State> {
    states
        .pair_iter()
        .find_map(|(h, s)| {
            if s.definition == definition {
                Some(h)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn find_node_by_definition(
    nodes: &Pool<PoseNode>,
    definition: Handle<PoseNodeDefinition>,
) -> Handle<PoseNode> {
    nodes
        .pair_iter()
        .find_map(|(h, s)| {
            if s.definition == definition {
                Some(h)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

impl Machine {
    #[inline]
    pub fn new(root: Handle<Node>) -> Self {
        Self {
            root,
            resource: None,
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

    #[inline]
    pub fn add_node(&mut self, node: PoseNode) -> Handle<PoseNode> {
        self.nodes.spawn(node)
    }

    #[inline]
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

    #[inline]
    pub fn parameters(&self) -> &ParameterContainer {
        &self.parameters
    }

    #[inline]
    pub fn set_entry_state(&mut self, entry_state: Handle<State>) {
        self.active_state = entry_state;
        self.entry_state = entry_state;
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
    pub fn resource(&self) -> Option<AbsmResource> {
        self.resource.clone()
    }

    #[inline]
    pub fn reset(&mut self) {
        for transition in self.transitions.iter_mut() {
            transition.reset();
        }

        self.active_state = self.entry_state;
    }

    #[inline]
    pub fn nodes(&self) -> impl Iterator<Item = &PoseNode> {
        self.nodes.iter()
    }

    #[inline]
    pub fn node_mut(&mut self, handle: Handle<PoseNode>) -> &mut PoseNode {
        &mut self.nodes[handle]
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
    pub fn transitions(&self) -> &Pool<Transition> {
        &self.transitions
    }

    #[inline]
    pub fn states(&self) -> &Pool<State> {
        &self.states
    }

    pub fn restore_resources(&mut self, resource_manager: ResourceManager) {
        resource_manager
            .state()
            .containers_mut()
            .absm
            .try_restore_optional_resource(&mut self.resource);
    }

    /// Synchronizes state of the machine with respective resource (if any).
    pub fn resolve(
        &mut self,
        animations_pack: &AnimationsPack,
        graph: &Graph,
        animations: &mut AnimationContainer,
    ) {
        if let Some(resource) = self.resource.clone() {
            let definition = &resource.data_ref().absm_definition;

            // Step 1. Restore integrity - add missing entities, remove nonexistent from instance.
            match definition
                .nodes
                .alive_count()
                .cmp(&self.nodes.alive_count())
            {
                Ordering::Less => {
                    // Some nodes were deleted in definition, remove respective instances.
                    let mut nodes_to_remove = Vec::new();
                    for (handle, node) in self.nodes.pair_iter() {
                        if !definition.nodes.is_valid_handle(node.definition) {
                            nodes_to_remove.push(handle);
                        }
                    }

                    for node_to_remove in nodes_to_remove {
                        self.nodes.free(node_to_remove);
                    }
                }
                Ordering::Equal => {
                    // Do nothing
                }
                Ordering::Greater => {
                    // Some nodes were added in definition, create respective instances.
                    for (node_definition_handle, node_definition) in definition.nodes.pair_iter() {
                        if self
                            .nodes
                            .iter()
                            .all(|n| n.definition != node_definition_handle)
                        {
                            let pose_node = instantiate_node(
                                node_definition,
                                node_definition_handle,
                                animations_pack,
                                self.root,
                                graph,
                                animations,
                            )
                            .unwrap();

                            let _ = self.nodes.spawn(pose_node);
                        }
                    }
                }
            }
            let definition_to_node_map = self
                .nodes
                .pair_iter()
                .map(|(h, n)| (n.definition, h))
                .collect::<FxHashMap<_, _>>();
            let fetch_node_by_definition = |definition: Handle<PoseNodeDefinition>| {
                definition_to_node_map
                    .get(&definition)
                    .cloned()
                    .unwrap_or_default()
            };

            match definition
                .states
                .alive_count()
                .cmp(&self.states.alive_count())
            {
                Ordering::Less => {
                    // Some states were deleted in definition, remove respective instances.
                    let mut states_to_remove = Vec::new();
                    for (handle, state) in self.states.pair_iter() {
                        if !definition.states.is_valid_handle(state.definition) {
                            states_to_remove.push(handle);
                        }
                    }

                    for node_to_remove in states_to_remove {
                        self.states.free(node_to_remove);
                    }
                }
                Ordering::Equal => {
                    // Do nothing.
                }
                Ordering::Greater => {
                    // Some states were added in definition, create respective instances.
                    for (state_definition_handle, state_definition) in definition.states.pair_iter()
                    {
                        if self
                            .states
                            .iter()
                            .all(|s| s.definition != state_definition_handle)
                        {
                            let root = find_node_by_definition(&self.nodes, state_definition.root);

                            let mut state = State::new(state_definition.name.as_ref(), root);

                            state.definition = state_definition_handle;

                            let _ = self.states.spawn(state);
                        }
                    }
                }
            }

            match definition
                .transitions
                .alive_count()
                .cmp(&self.transitions.alive_count())
            {
                Ordering::Less => {
                    // Some transitions were deleted in definition, remove respective instances.
                    let mut transitions_to_remove = Vec::new();
                    for (handle, transition) in self.transitions.pair_iter() {
                        if !definition
                            .transitions
                            .is_valid_handle(transition.definition)
                        {
                            transitions_to_remove.push(handle);
                        }
                    }

                    for node_to_remove in transitions_to_remove {
                        self.transitions.free(node_to_remove);
                    }
                }
                Ordering::Equal => {
                    // Do nothing.
                }
                Ordering::Greater => {
                    // Some transitions were added in definition, create respective instances.
                    for (transition_definition_handle, transition_definition) in
                        definition.transitions.pair_iter()
                    {
                        if self
                            .transitions
                            .iter()
                            .all(|t| t.definition != transition_definition_handle)
                        {
                            let mut transition = Transition::new(
                                transition_definition.name.as_ref(),
                                find_state_by_definition(
                                    &self.states,
                                    transition_definition.source,
                                ),
                                find_state_by_definition(&self.states, transition_definition.dest),
                                transition_definition.transition_time,
                                transition_definition.rule.as_str(),
                            );

                            transition.definition = transition_definition_handle;

                            let _ = self.transitions.spawn(transition);
                        }
                    }
                }
            }

            // Step 2. Sync data of instance entities with respective definitions.
            for node in self.nodes.iter_mut() {
                let node_definition = &definition.nodes[node.definition];

                match node {
                    PoseNode::PlayAnimation(play_animation) => {
                        if let PoseNodeDefinition::PlayAnimation(play_animation_definition) =
                            node_definition
                        {
                            let definition_animation = animations_pack
                                .sources()
                                .get(&play_animation_definition.animation);

                            if animations.try_get(play_animation.animation).map_or(
                                true,
                                |current_animation| {
                                    definition_animation
                                        .map_or(true, |a| a != &current_animation.resource)
                                },
                            ) {
                                animations.remove(play_animation.animation);

                                let new_animation =
                                    if let Some(definition_animation) = definition_animation {
                                        instantiate_animation_holder(
                                            definition_animation,
                                            self.root,
                                            graph,
                                            animations,
                                        )
                                    } else {
                                        Handle::NONE
                                    };

                                *play_animation = PlayAnimation {
                                    base: BasePoseNode {
                                        definition: play_animation.definition,
                                    },
                                    animation: new_animation,
                                    output_pose: Default::default(),
                                };
                            }

                            // Apply definition properties to instance.
                            if let Some(animation) =
                                animations.try_get_mut(play_animation.animation)
                            {
                                animation
                                    .set_speed(play_animation_definition.speed)
                                    .set_time_slice(
                                        play_animation_definition.time_slice.clone().map(|s| s.0),
                                    );
                            }
                        } else {
                            unreachable!()
                        }
                    }
                    PoseNode::BlendAnimations(blend_animations) => {
                        if let PoseNodeDefinition::BlendAnimations(blend_animations_definition) =
                            node_definition
                        {
                            *blend_animations = BlendAnimations {
                                base: BasePoseNode {
                                    definition: blend_animations.definition,
                                },
                                pose_sources: blend_animations_definition
                                    .pose_sources
                                    .iter()
                                    .map(|s| BlendPose {
                                        weight: s.weight.clone(),
                                        pose_source: fetch_node_by_definition(s.pose_source),
                                    })
                                    .collect(),
                                output_pose: std::mem::take(&mut blend_animations.output_pose),
                            }
                        }
                    }
                    PoseNode::BlendAnimationsByIndex(blend_animations) => {
                        if let PoseNodeDefinition::BlendAnimationsByIndex(
                            blend_animations_definition,
                        ) = node_definition
                        {
                            *blend_animations = BlendAnimationsByIndex {
                                base: BasePoseNode {
                                    definition: blend_animations.definition,
                                },
                                index_parameter: blend_animations_definition
                                    .index_parameter
                                    .clone(),
                                inputs: blend_animations_definition
                                    .inputs
                                    .iter()
                                    .map(|i| IndexedBlendInput {
                                        blend_time: i.blend_time,
                                        pose_source: fetch_node_by_definition(i.pose_source),
                                    })
                                    .collect(),
                                prev_index: blend_animations.prev_index.clone(),
                                blend_time: blend_animations.blend_time.clone(),
                                output_pose: std::mem::take(&mut blend_animations.output_pose),
                            }
                        }
                    }
                }
            }

            for state in self.states.iter_mut() {
                let state_definition = &definition.states[state.definition];

                // Reassign the entire state to trigger compiler error if there's a new field.
                *state = State {
                    definition: state.definition,
                    name: state_definition.name.clone(),
                    root: find_node_by_definition(&self.nodes, state_definition.root),
                };
            }

            for transition in self.transitions.iter_mut() {
                let transition_definition = &definition.transitions[transition.definition];

                *transition = Transition {
                    definition: transition.definition,
                    name: transition_definition.name.clone(),
                    transition_time: transition_definition.transition_time,
                    elapsed_time: transition.elapsed_time,
                    source: find_state_by_definition(&self.states, transition_definition.source),
                    dest: find_state_by_definition(&self.states, transition_definition.dest),
                    rule: transition_definition.rule.clone(),
                    invert_rule: transition_definition.invert_rule,
                    blend_factor: transition.blend_factor,
                };
            }

            // Step 3. Sync parameters.
            self.parameters.clear();
            for definition in definition.parameters.container.iter() {
                self.set_parameter(&definition.name, definition.value);
            }
        }
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
                    if let Some(Parameter::Rule(mut active)) =
                        self.parameters.get(transition.rule()).cloned()
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

        &self.final_pose
    }
}
