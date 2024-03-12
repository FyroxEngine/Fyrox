//! Animation player is a node that contains multiple animations. It updates and plays all the animations.
//! See [`AnimationPlayer`] docs for more info.

use crate::MessageDirection;
use crate::{
    core::{
        log::{Log, MessageKind},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    define_constructor, define_widget_deref,
    generic_animation::value::{BoundValueCollection, TrackValue, ValueBinding},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationPlayerMessage {
    EnableAnimation { animation: String, enabled: bool },
    RewindAnimation { animation: String },
    TimePosition { animation: String, time: f32 },
}

impl AnimationPlayerMessage {
    define_constructor!(
        /// Creates a new [Self::EnableAnimation] message.
        AnimationPlayerMessage:EnableAnimation => fn enable_animation(animation: String, enabled: bool), layout: false
    );
    define_constructor!(
        /// Creates a new [Self::RewindAnimation] message.
        AnimationPlayerMessage:RewindAnimation => fn rewind_animation(animation: String), layout: false
    );
    define_constructor!(
        /// Creates a new [Self::TimePosition] message.
        AnimationPlayerMessage:TimePosition => fn time_position(animation: String, time: f32), layout: false
    );
}

/// UI-specific animation.
pub type Animation = crate::generic_animation::Animation<Handle<UiNode>>;
/// UI-specific animation track.
pub type Track = crate::generic_animation::track::Track<Handle<UiNode>>;
/// UI-specific animation container.
pub type AnimationContainer = crate::generic_animation::AnimationContainer<Handle<UiNode>>;
/// UI-specific animation pose.
pub type AnimationPose = crate::generic_animation::AnimationPose<Handle<UiNode>>;
/// UI-specific animation node pose.
pub type NodePose = crate::generic_animation::NodePose<Handle<UiNode>>;

/// Standard prelude for animations, that contains all most commonly used types and traits.
pub mod prelude {
    pub use super::{
        Animation, AnimationContainer, AnimationContainerExt, AnimationPlayer,
        AnimationPlayerBuilder, AnimationPose, AnimationPoseExt, BoundValueCollectionExt, NodePose,
        Track,
    };
    pub use crate::generic_animation::{
        container::{TrackDataContainer, TrackValueKind},
        signal::AnimationSignal,
        value::{BoundValueCollection, TrackValue, ValueBinding, ValueType},
        AnimationEvent,
    };
}

/// Extension trait for [`AnimationContainer`].
pub trait AnimationContainerExt {
    /// Updates all animations in the container and applies their poses to respective nodes. This method is intended to
    /// be used only by the internals of the engine!
    fn update_animations(&mut self, nodes: &mut UserInterface, apply: bool, dt: f32);
}

impl AnimationContainerExt for AnimationContainer {
    fn update_animations(&mut self, ui: &mut UserInterface, apply: bool, dt: f32) {
        for animation in self.iter_mut().filter(|anim| anim.is_enabled()) {
            animation.tick(dt);
            if apply {
                animation.pose().apply(ui);
            }
        }
    }
}

/// Extension trait for [`AnimationPose`].
pub trait AnimationPoseExt {
    /// Tries to set each value to the each property from the animation pose to respective widgets.
    fn apply(&self, ui: &mut UserInterface);
}

impl AnimationPoseExt for AnimationPose {
    fn apply(&self, ui: &mut UserInterface) {
        for (node, local_pose) in self.poses() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = ui.try_get_mut(*node) {
                node.invalidate_layout();

                local_pose.values.apply(node);
            }
        }
    }
}

/// Extension trait for [`BoundValueCollection`].
pub trait BoundValueCollectionExt {
    /// Tries to set each value from the collection to the respective property (by binding) of the
    /// given widget.
    fn apply(&self, node_ref: &mut UiNode);
}

impl BoundValueCollectionExt for BoundValueCollection {
    fn apply(&self, node_ref: &mut UiNode) {
        for bound_value in self.values.iter() {
            match bound_value.binding {
                ValueBinding::Position => {
                    if let TrackValue::Vector2(v) = bound_value.value {
                        node_ref.set_desired_local_position(v);
                    } else {
                        Log::err(
                            "Unable to apply position, because underlying type is not Vector2!",
                        )
                    }
                }
                ValueBinding::Scale => Log::warn("Implement me!"),
                ValueBinding::Rotation => Log::warn("Implement me!"),
                ValueBinding::Property {
                    name: ref property_name,
                    value_type,
                } => bound_value.apply_to_object(node_ref, property_name, value_type),
            }
        }
    }
}

/// Animation player is a node that contains multiple animations. It updates and plays all the animations.
/// The node could be a source of animations for animation blending state machines. To learn more about
/// animations, see [`Animation`] docs.
#[derive(Visit, Reflect, Clone, Debug, ComponentProvider)]
pub struct AnimationPlayer {
    widget: Widget,
    #[component(include)]
    pub(crate) animations: InheritableVariable<AnimationContainer>,
    #[component(include)]
    auto_apply: bool,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            widget: Default::default(),
            animations: Default::default(),
            auto_apply: true,
        }
    }
}

impl AnimationPlayer {
    /// Enables or disables automatic animation pose applying. Every animation in the node is updated first, and
    /// then their output pose could be applied to the graph, so the animation takes effect. Automatic applying
    /// is useful when you need your animations to be applied immediately to the graph, but in some cases (if you're
    /// using animation blending state machines for example) this functionality is undesired.
    pub fn set_auto_apply(&mut self, auto_apply: bool) {
        self.auto_apply = auto_apply;
    }

    /// Returns `true` if the node is automatically applying output poses of animations to the graph, `false` -
    /// otherwise.
    pub fn is_auto_apply(&self) -> bool {
        self.auto_apply
    }

    /// Returns a reference to internal animations container.
    pub fn animations(&self) -> &InheritableVariable<AnimationContainer> {
        &self.animations
    }

    /// Returns a reference to internal animations container. Keep in mind that mutable access to [`InheritableVariable`]
    /// may have side effects if used inappropriately. Checks docs for [`InheritableVariable`] for more info.
    pub fn animations_mut(&mut self) -> &mut InheritableVariable<AnimationContainer> {
        &mut self.animations
    }

    /// Sets new animations container of the animation player.
    pub fn set_animations(&mut self, animations: AnimationContainer) {
        self.animations.set_value_and_mark_modified(animations);
    }

    fn find_animation(&mut self, name: &str) -> Option<&mut Animation> {
        self.animations.find_by_name_mut(name).map(|(_, a)| a)
    }
}

impl TypeUuidProvider for AnimationPlayer {
    fn type_uuid() -> Uuid {
        uuid!("44d1c94e-354f-4f9a-b918-9d31c28aa16a")
    }
}

define_widget_deref!(AnimationPlayer);

impl Control for AnimationPlayer {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<AnimationPlayerMessage>() {
            match msg {
                AnimationPlayerMessage::EnableAnimation { animation, enabled } => {
                    if let Some(animation) = self.find_animation(animation) {
                        animation.set_enabled(*enabled);
                    }
                }
                AnimationPlayerMessage::RewindAnimation { animation } => {
                    if let Some(animation) = self.find_animation(animation) {
                        animation.rewind();
                    }
                }
                AnimationPlayerMessage::TimePosition { animation, time } => {
                    if let Some(animation) = self.find_animation(animation) {
                        animation.set_time_position(*time);
                    }
                }
            }
        }
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.animations
            .get_value_mut_silent()
            .update_animations(ui, self.auto_apply, dt);
    }
}

/// A builder for [`AnimationPlayer`] node.
pub struct AnimationPlayerBuilder {
    widget_builder: WidgetBuilder,
    animations: AnimationContainer,
    auto_apply: bool,
}

impl AnimationPlayerBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            animations: AnimationContainer::new(),
            auto_apply: true,
        }
    }

    /// Sets a container with desired animations.
    pub fn with_animations(mut self, animations: AnimationContainer) -> Self {
        self.animations = animations;
        self
    }

    /// Enables or disables automatic pose applying. See [`AnimationPlayer::set_auto_apply`] docs for more info.
    pub fn with_auto_apply(mut self, auto_apply: bool) -> Self {
        self.auto_apply = auto_apply;
        self
    }

    /// Creates an instance of [`AnimationPlayer`] node.
    pub fn build_node(self) -> UiNode {
        UiNode::new(AnimationPlayer {
            widget: self.widget_builder.with_need_update(true).build(),
            animations: self.animations.into(),
            auto_apply: self.auto_apply,
        })
    }

    /// Creates an instance of [`AnimationPlayer`] node and adds it to the given user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node())
    }
}
