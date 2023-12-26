//! Animation blending state machine.
//!
//! Machine is used to blend multiple animation as well as perform automatic "smooth transition
//! between states. See [`Machine`] docs for more info and examples.

#![warn(missing_docs)]

use crate::{
    core::{
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
    AnimationContainer, AnimationPose, EntityId,
};

pub use event::Event;
use fyrox_core::{find_by_name_mut, find_by_name_ref};
pub use layer::MachineLayer;
pub use mask::LayerMask;
pub use node::{
    blend::{BlendAnimations, BlendAnimationsByIndex, BlendPose, IndexedBlendInput},
    play::PlayAnimation,
    AnimationPoseSource, PoseNode,
};
pub use parameter::{Parameter, ParameterContainer, PoseWeight};
pub use state::State;
pub use transition::Transition;

pub mod event;
pub mod layer;
pub mod mask;
pub mod node;
pub mod parameter;
pub mod state;
pub mod transition;

/// Animation blending state machine is used to blend multiple animation as well as perform automatic smooth transitions
/// between states.
///
/// # Terminology
///
/// `Node` - is a part of sub-graph that backs _states_ with animations. Typical nodes are `PlayAnimation`, `BlendAnimations`,
/// `BlendAnimationsByIndex`, etc. Nodes can be connected forming a tree, some node could be marked as output - its animation
/// will be used in parent state.
/// `State` - is a final source of animation for blending. There could be any number of states, for example typical
/// states are: `run`, `idle`, `jump` etc. A state could be marked as _entry_ state - it will be active at the first frame
/// when using the machine. There is always one state active.
/// `Transition` - is a connection between states that has transition time, a link to a parameter that defines whether the
/// transition should be performed or not. Transition is directional; there could be any number of transitions between any
/// number of states (loops are allowed).
/// `Parameter` - is a named variable of a fixed type (see `Parameters` section for more info).
/// `Layer` - is a separate state graph, there could be any number of layers - each with its own mask.
/// `Mask` - a set of handles to nodes which will be excluded from animation on a layer.
/// `Pose` - a final result of blending multiple animation into one.
///
/// Summarizing everything of this, we can describe animation blending state machine as a state graph, where each state has its
/// own sub-graph (tree) that provides animation for blending. States can be connected via transitions.
///
/// # Parameters
///
/// Parameter is a named variable of a fixed type. Parameters are used as a data source in various places in the animation
/// blending state machines. There are three main types of parameters:
///
/// `Rule` - boolean value that used as a trigger for transitions. When transition is using some rule, it checks the value
/// of the parameter and if it is `true` transition starts.
/// `Weight` - real number (`f32`) that is used a weight when you blending multiple animations into one.
/// `Index` - natural number (`i32`) that is used as an animation selector.
///
/// Each parameter has a name, it could be pretty much any string.
///
/// # Layers
///
/// Layer is a separate state graph. Layers mainly used to animate different parts of humanoid (but not only) characters. For
/// example there could a layer for upper body and a layer for lower body. Upper body layer could contain animations for aiming,
/// melee attacks while lower body layer could contain animations for standing, running, crouching, etc. This gives you an
/// ability to have running character that could aim or melee attack, or crouching and aiming, and so on with any combination.
/// Both layers use the same set of parameters, so a change in a parameter will affect all layers that use it.
///
/// # Examples
///
/// Let have a quick look at simple state machine graph with a single layer:
///
/// ```text
///                                                  +-------------+
///                                                  |  Idle Anim  |
///                                                  +------+------+
///                                                         |
///           Walk Weight                                   |
/// +-----------+      +-------+           Walk->Idle Rule  |
/// | Walk Anim +------+       |                            |
/// +-----------+      |       |      +-------+         +---+---+
///                    | Blend |      |       +-------->+       |
///                    |       +------+ Walk  |         |  Idle |
/// +-----------+      |       |      |       +<--------+       |
/// | Aim Anim  +------+       |      +--+----+         +---+---+
/// +-----------+      +-------+         |                  ^
///           Aim Weight                 | Idle->Walk Rule  |
///                                      |                  |
///                       Walk->Run Rule |    +---------+   | Run->Idle Rule
///                                      |    |         |   |
///                                      +--->+   Run   +---+
///                                           |         |
///                                           +----+----+
///                                                |
///                                                |
///                                         +------+------+
///                                         |  Run Anim   |
///                                         +-------------+
/// ```
///
/// Here we have `Walk`, `Idle`, `Run` _states_ which uses different sources of poses:
///
/// - `Run` and `Idle` both directly uses respective animations as a pose source.
/// - `Walk` - is the most complex here - it uses result of blending between `Aim` and `Walk` animations with different
/// weights. This is useful if your character can only walk or can walk *and* aim at the same time. Desired pose
/// determined by `Walk Weight` and `Aim Weight` parameters combination (see `Parameters` section for more info).
/// **Note:** Such blending is almost never used on practice, instead you should use multiple animation layers. This
/// serves only as an example that the machine can blend animations.
///
/// There are four transitions between three states each with its own _rule_. Rule is just Rule parameter which can
/// have boolean value that indicates that transition should be activated. The machine on the image above can be created
/// using code like so:
///
/// ```no_run
/// use fyrox_animation::{
///     machine::{
///         Machine, State, Transition, PoseNode,
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
/// let mut machine = Machine::<ErasedHandle>::new();
///
/// let root_layer = &mut machine.layers_mut()[0];
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
///
/// This creates a machine with a single animation layer, fills it with some states that are backed by animation
/// sources (either simple animation playback or animation blending). You can use multiple layers to animate a single
/// model - for example one layer could be used for upper body of a character and other is lower body. This means that
/// locomotion machine will take control over lower body and combat machine will control upper body.
///
/// Complex state machines quite hard to create from code, you should use ABSM editor instead whenever possible.
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq)]
pub struct Machine<T: EntityId> {
    parameters: ParameterContainer,

    #[visit(optional)]
    layers: Vec<MachineLayer<T>>,

    #[visit(skip)]
    #[reflect(hidden)]
    final_pose: AnimationPose<T>,
}

impl<T: EntityId> Machine<T> {
    /// Creates a new animation blending state machine with a single animation layer.
    #[inline]
    pub fn new() -> Self {
        Self {
            parameters: Default::default(),
            layers: vec![MachineLayer::new()],
            final_pose: Default::default(),
        }
    }

    /// Sets a value for existing parameter with given id or registers new parameter with given id and provided value.
    /// The method returns a reference to the machine, so the calls could be chained:
    ///
    /// ```rust
    /// use fyrox_animation::machine::{Machine, Parameter};
    /// use fyrox_core::pool::ErasedHandle;
    ///
    /// let mut machine = Machine::<ErasedHandle>::new();
    ///
    /// machine
    ///     .set_parameter("Run", Parameter::Rule(true))
    ///     .set_parameter("Jump", Parameter::Rule(false));
    /// ```
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

    /// Returns a shared reference to the container with all parameters used by the animation blending state machine.
    #[inline]
    pub fn parameters(&self) -> &ParameterContainer {
        &self.parameters
    }

    /// Returns a mutable reference to the container with all parameters used by the animation blending state machine.
    #[inline]
    pub fn parameters_mut(&mut self) -> &mut ParameterContainer {
        &mut self.parameters
    }

    /// Adds a new layer to the animation blending state machine.
    #[inline]
    pub fn add_layer(&mut self, layer: MachineLayer<T>) {
        self.layers.push(layer)
    }

    /// Removes a layer at given index. Panics if index is out-of-bounds.
    #[inline]
    pub fn remove_layer(&mut self, index: usize) -> MachineLayer<T> {
        self.layers.remove(index)
    }

    /// Inserts a layer at given position, panics in index is out-of-bounds.
    #[inline]
    pub fn insert_layer(&mut self, index: usize, layer: MachineLayer<T>) {
        self.layers.insert(index, layer)
    }

    /// Removes last layer from the list.
    #[inline]
    pub fn pop_layer(&mut self) -> Option<MachineLayer<T>> {
        self.layers.pop()
    }

    /// Returns a shared reference to the list of layers.
    #[inline]
    pub fn layers(&self) -> &[MachineLayer<T>] {
        &self.layers
    }

    /// Returns a mutable reference to the list of layers.
    #[inline]
    pub fn layers_mut(&mut self) -> &mut [MachineLayer<T>] {
        &mut self.layers
    }

    /// Tries to find a layer by its name. Returns index of the layer and its reference.
    #[inline]
    pub fn find_layer_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(usize, &MachineLayer<T>)> {
        find_by_name_ref(self.layers.iter().enumerate(), name)
    }

    /// Tries to find a layer by its name. Returns index of the layer and its reference.
    #[inline]
    pub fn find_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(usize, &mut MachineLayer<T>)> {
        find_by_name_mut(self.layers.iter_mut().enumerate(), name)
    }

    /// Returns final pose of the machine.
    #[inline]
    pub fn pose(&self) -> &AnimationPose<T> {
        &self.final_pose
    }

    /// Computes final animation pose that could be then applied to a set of entities graph.
    #[inline]
    pub fn evaluate_pose(
        &mut self,
        animations: &mut AnimationContainer<T>,
        dt: f32,
    ) -> &AnimationPose<T> {
        self.final_pose.reset();

        for layer in self.layers.iter_mut() {
            let weight = layer.weight();
            let pose = layer.evaluate_pose(animations, &self.parameters, dt);

            self.final_pose.blend_with(pose, weight);
        }

        &self.final_pose
    }
}
