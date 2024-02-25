//! Animation blending state machine is a node that takes multiple animations from an animation player and
//! mixes them in arbitrary way into one animation. See [`AnimationBlendingStateMachine`] docs for more info.

use crate::{
    animation::{AnimationPlayer, AnimationPoseExt},
    core::{
        pool::Handle, reflect::prelude::*, type_traits::prelude::*, variable::InheritableVariable,
        visitor::prelude::*,
    },
    define_widget_deref,
    message::{KeyCode, MouseButton, UiMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_animation::machine::Parameter;
use fyrox_graph::{SceneGraph, SceneGraphNode};
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// UI-specific root motion settings.
pub type RootMotionSettings = crate::generic_animation::RootMotionSettings<Handle<UiNode>>;
/// UI-specific animation pose node.
pub type PoseNode = crate::generic_animation::machine::PoseNode<Handle<UiNode>>;
/// UI-specific animation pose node.
pub type PlayAnimation =
    crate::generic_animation::machine::node::play::PlayAnimation<Handle<UiNode>>;
/// UI-specific animation blending state machine BlendAnimations node.
pub type BlendAnimations =
    crate::generic_animation::machine::node::blend::BlendAnimations<Handle<UiNode>>;
/// UI-specific animation blending state machine BlendAnimationsByIndex node.
pub type BlendAnimationsByIndex =
    crate::generic_animation::machine::node::blend::BlendAnimationsByIndex<Handle<UiNode>>;
/// UI-specific animation blending state machine BlendPose node.
pub type BlendPose = crate::generic_animation::machine::node::blend::BlendPose<Handle<UiNode>>;
/// UI-specific animation blending state machine IndexedBlendInput node.
pub type IndexedBlendInput =
    crate::generic_animation::machine::node::blend::IndexedBlendInput<Handle<UiNode>>;
/// UI-specific animation blending state machine BlendSpace node.
pub type BlendSpace =
    crate::generic_animation::machine::node::blendspace::BlendSpace<Handle<UiNode>>;
/// UI-specific animation blending state machine blend space point.
pub type BlendSpacePoint =
    crate::generic_animation::machine::node::blendspace::BlendSpacePoint<Handle<UiNode>>;
/// UI-specific animation blending state machine layer mask.
pub type LayerMask = crate::generic_animation::machine::mask::LayerMask<Handle<UiNode>>;
/// UI-specific animation blending state machine layer mask.
pub type Event = crate::generic_animation::machine::event::Event<Handle<UiNode>>;
/// UI-specific animation blending state machine.
pub type Machine = crate::generic_animation::machine::Machine<Handle<UiNode>>;
/// UI-specific animation blending state machine layer.
pub type MachineLayer = crate::generic_animation::machine::MachineLayer<Handle<UiNode>>;
/// UI-specific animation blending state machine transition.
pub type Transition = crate::generic_animation::machine::transition::Transition<Handle<UiNode>>;
/// UI-specific animation blending state machine state.
pub type State = crate::generic_animation::machine::state::State<Handle<UiNode>>;
/// UI-specific animation blending state machine base pose node.
pub type BasePoseNode = crate::generic_animation::machine::node::BasePoseNode<Handle<UiNode>>;
/// UI-specific animation blending state machine state action.
pub type StateAction = crate::generic_animation::machine::state::StateAction<Handle<UiNode>>;
/// UI-specific animation blending state machine state action wrapper.
pub type StateActionWrapper =
    crate::generic_animation::machine::state::StateActionWrapper<Handle<UiNode>>;
/// UI-specific animation blending state machine logic node.
pub type LogicNode = crate::generic_animation::machine::transition::LogicNode<Handle<UiNode>>;
/// UI-specific animation blending state machine And logic node.
pub type AndNode = crate::generic_animation::machine::transition::AndNode<Handle<UiNode>>;
/// UI-specific animation blending state machine Xor logic nde.
pub type XorNode = crate::generic_animation::machine::transition::XorNode<Handle<UiNode>>;
/// UI-specific animation blending state machine Or logic node.
pub type OrNode = crate::generic_animation::machine::transition::OrNode<Handle<UiNode>>;
/// UI-specific animation blending state machine Not logic node.
pub type NotNode = crate::generic_animation::machine::transition::NotNode<Handle<UiNode>>;
/// UI-specific animation blending state machine layer animation events collection.
pub type LayerAnimationEventsCollection =
    crate::generic_animation::machine::layer::LayerAnimationEventsCollection<Handle<UiNode>>;
/// UI-specific animation blending state machine animation events source.
pub type AnimationEventsSource =
    crate::generic_animation::machine::layer::AnimationEventsSource<Handle<UiNode>>;

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

/// Animation blending state machine (ABSM) is a node that takes multiple animations from an animation player and
/// mixes them in arbitrary way into one animation. Usually, ABSMs are used to animate humanoid characters in games,
/// by blending multiple states with one or more animations. More info about state machines can be found in
/// [`Machine`] docs.
///
/// # Important notes
///
/// The node does **not** contain any animations, instead it just takes animations from an animation
/// player node and mixes them.
#[derive(Visit, Reflect, Clone, Debug, Default, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "4b08c753-2a10-41e3-8fb2-4fd0517e86bc")]
pub struct AnimationBlendingStateMachine {
    widget: Widget,
    #[component(include)]
    machine: InheritableVariable<Machine>,
    #[component(include)]
    animation_player: InheritableVariable<Handle<UiNode>>,
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
    pub fn set_animation_player(&mut self, animation_player: Handle<UiNode>) {
        self.animation_player
            .set_value_and_mark_modified(animation_player);
    }

    /// Returns an animation player used by the node.
    pub fn animation_player(&self) -> Handle<UiNode> {
        *self.animation_player
    }
}

define_widget_deref!(AnimationBlendingStateMachine);

impl Control for AnimationBlendingStateMachine {
    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        if let Some(animation_player) = ui
            .nodes
            .try_borrow_mut(*self.animation_player)
            .and_then(|n| n.component_mut::<AnimationPlayer>())
        {
            // Prevent animation player to apply animation to scene nodes. The animation will
            // do than instead.
            animation_player.set_auto_apply(false);

            let pose = self
                .machine
                .get_value_mut_silent()
                .evaluate_pose(animation_player.animations.get_value_mut_silent(), dt);

            pose.apply(ui);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

/// Animation blending state machine builder allows you to create state machines in declarative manner.
pub struct AnimationBlendingStateMachineBuilder {
    widget_builder: WidgetBuilder,
    machine: Machine,
    animation_player: Handle<UiNode>,
}

impl AnimationBlendingStateMachineBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
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
    pub fn with_animation_player(mut self, animation_player: Handle<UiNode>) -> Self {
        self.animation_player = animation_player;
        self
    }

    /// Creates new node.
    pub fn build_node(self) -> UiNode {
        UiNode::new(AnimationBlendingStateMachine {
            widget: self.widget_builder.with_need_update(true).build(),
            machine: self.machine.into(),
            animation_player: self.animation_player.into(),
        })
    }

    /// Creates new node and adds it to the user interface.
    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        ui.add_node(self.build_node())
    }
}

#[derive(
    Visit,
    Reflect,
    Clone,
    Debug,
    Default,
    PartialEq,
    TypeUuidProvider,
    AsRefStr,
    EnumString,
    VariantNames,
)]
#[type_uuid(id = "291e8734-47df-408e-8b2c-57bfb941d8ec")]
pub enum EventKind {
    #[default]
    MouseEnter,
    MouseLeave,
    MouseMove,
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseWheel,
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    Focus,
    Unfocus,
    TouchStarted,
    TouchEnded,
    TouchMoved,
    TouchCancelled,
    DoubleTap,
}

#[derive(Visit, Reflect, Clone, Debug, Default, PartialEq, TypeUuidProvider)]
#[type_uuid(id = "15f306b8-3bb8-4b35-87bd-6e9e5d748454")]
pub struct EventAction {
    kind: EventKind,
    parameter_name: String,
    parameter_value: Parameter,
}

/// A widget that listens for particular events and sets parameters in an ABSM accordingly.
#[derive(Visit, Reflect, Clone, Debug, Default, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "15f306b8-3bb8-4b35-87bd-6e9e5d748455")]
pub struct AbsmEventProvider {
    widget: Widget,
    actions: InheritableVariable<Vec<EventAction>>,
    absm: InheritableVariable<Handle<UiNode>>,
}

define_widget_deref!(AbsmEventProvider);

impl AbsmEventProvider {
    fn on_event(&self, ui: &mut UserInterface, kind: EventKind) {
        let Some(action) = self.actions.iter().find(|a| a.kind == kind) else {
            return;
        };

        let Some(absm) = ui.try_get_mut_of_type::<AnimationBlendingStateMachine>(*self.absm) else {
            return;
        };

        absm.machine_mut()
            .set_parameter(&action.parameter_name, action.parameter_value);
    }
}

impl Control for AbsmEventProvider {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        let Some(msg) = message.data::<WidgetMessage>() else {
            return;
        };

        use WidgetMessage as Msg;
        match msg {
            Msg::MouseDown { button, .. } => self.on_event(ui, EventKind::MouseDown(*button)),
            Msg::MouseUp { button, .. } => self.on_event(ui, EventKind::MouseUp(*button)),
            Msg::MouseMove { .. } => self.on_event(ui, EventKind::MouseMove),
            Msg::MouseWheel { .. } => self.on_event(ui, EventKind::MouseWheel),
            Msg::MouseLeave => self.on_event(ui, EventKind::MouseLeave),
            Msg::MouseEnter => self.on_event(ui, EventKind::MouseEnter),
            Msg::Focus => self.on_event(ui, EventKind::Focus),
            Msg::Unfocus => self.on_event(ui, EventKind::Unfocus),
            Msg::TouchStarted { .. } => self.on_event(ui, EventKind::TouchStarted),
            Msg::TouchEnded { .. } => self.on_event(ui, EventKind::TouchEnded),
            Msg::TouchMoved { .. } => self.on_event(ui, EventKind::TouchMoved),
            Msg::TouchCancelled { .. } => self.on_event(ui, EventKind::TouchCancelled),
            Msg::DoubleTap { .. } => self.on_event(ui, EventKind::DoubleTap),
            Msg::KeyUp(key) => self.on_event(ui, EventKind::KeyUp(*key)),
            Msg::KeyDown(key) => self.on_event(ui, EventKind::KeyDown(*key)),
            _ => (),
        }
    }
}

pub struct AbsmEventProviderBuilder {
    widget_builder: WidgetBuilder,
    actions: Vec<EventAction>,
    absm: Handle<UiNode>,
}

impl AbsmEventProviderBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            actions: Default::default(),
            absm: Default::default(),
        }
    }

    pub fn with_actions(mut self, actions: Vec<EventAction>) -> Self {
        self.actions = actions;
        self
    }

    pub fn with_absm(mut self, absm: Handle<UiNode>) -> Self {
        self.absm = absm;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let provider = AbsmEventProvider {
            widget: self.widget_builder.build(),
            actions: self.actions.into(),
            absm: self.absm.into(),
        };

        ctx.add_node(UiNode::new(provider))
    }
}
