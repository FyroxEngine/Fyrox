//! Animation player is a node that contains multiple animations. It updates and plays all the animations.
//! See [`AnimationPlayer`] docs for more info.

use crate::{
    core::{
        log::{Log, MessageKind},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    generic_animation::value::{BoundValueCollection, TrackValue, ValueBinding},
    scene::{
        base::{Base, BaseBuilder},
        graph::{Graph, NodePool},
        node::{Node, NodeTrait, UpdateContext},
    },
};
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

pub mod absm;
pub mod spritesheet;

/// Scene specific animation.
pub type Animation = crate::generic_animation::Animation<Handle<Node>>;
/// Scene specific animation track.
pub type Track = crate::generic_animation::track::Track<Handle<Node>>;
/// Scene specific animation container.
pub type AnimationContainer = crate::generic_animation::AnimationContainer<Handle<Node>>;
/// Scene specific animation pose.
pub type AnimationPose = crate::generic_animation::AnimationPose<Handle<Node>>;
/// Scene specific animation node pose.
pub type NodePose = crate::generic_animation::NodePose<Handle<Node>>;

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
    fn update_animations(&mut self, nodes: &mut NodePool, apply: bool, dt: f32);
}

impl AnimationContainerExt for AnimationContainer {
    fn update_animations(&mut self, nodes: &mut NodePool, apply: bool, dt: f32) {
        for animation in self.iter_mut().filter(|anim| anim.is_enabled()) {
            animation.tick(dt);
            if apply {
                animation.pose().apply_internal(nodes);
            }
        }
    }
}

/// Extension trait for [`AnimationPose`].
pub trait AnimationPoseExt {
    /// Tries to set each value to the each property from the animation pose to respective scene nodes.
    fn apply_internal(&self, nodes: &mut NodePool);

    /// Tries to set each value to the each property from the animation pose to respective scene nodes.
    fn apply(&self, graph: &mut Graph);

    /// Calls given callback function for each node and allows you to apply pose with your own
    /// rules. This could be useful if you need to ignore transform some part of pose for a node.
    fn apply_with<C>(&self, graph: &mut Graph, callback: C)
    where
        C: FnMut(&mut Node, Handle<Node>, &NodePose);
}

impl AnimationPoseExt for AnimationPose {
    fn apply_internal(&self, nodes: &mut NodePool) {
        for (node, local_pose) in self.poses() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = nodes.try_borrow_mut(*node) {
                local_pose.values.apply(node);
            }
        }
    }

    fn apply(&self, graph: &mut Graph) {
        for (node, local_pose) in self.poses() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = graph.try_get_mut(*node) {
                local_pose.values.apply(node);
            }
        }
    }

    fn apply_with<C>(&self, graph: &mut Graph, mut callback: C)
    where
        C: FnMut(&mut Node, Handle<Node>, &NodePose),
    {
        for (node, local_pose) in self.poses() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node_ref) = graph.try_get_mut(*node) {
                callback(node_ref, *node, local_pose);
            }
        }
    }
}

/// Extension trait for [`BoundValueCollection`].
pub trait BoundValueCollectionExt {
    /// Tries to set each value from the collection to the respective property (by binding) of the given scene node.
    fn apply(&self, node_ref: &mut Node);
}

impl BoundValueCollectionExt for BoundValueCollection {
    fn apply(&self, node_ref: &mut Node) {
        for bound_value in self.values.iter() {
            match bound_value.binding {
                ValueBinding::Position => {
                    if let TrackValue::Vector3(v) = bound_value.value {
                        node_ref.local_transform_mut().set_position(v);
                    } else {
                        Log::err(
                            "Unable to apply position, because underlying type is not Vector3!",
                        )
                    }
                }
                ValueBinding::Scale => {
                    if let TrackValue::Vector3(v) = bound_value.value {
                        node_ref.local_transform_mut().set_scale(v);
                    } else {
                        Log::err("Unable to apply scaling, because underlying type is not Vector3!")
                    }
                }
                ValueBinding::Rotation => {
                    if let TrackValue::UnitQuaternion(v) = bound_value.value {
                        node_ref.local_transform_mut().set_rotation(v);
                    } else {
                        Log::err("Unable to apply rotation, because underlying type is not UnitQuaternion!")
                    }
                }
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
///
/// # Examples
///
/// Always prefer using animation editor to create animation player nodes. It has rich functionality and
/// an ability to preview the result of animations. If you need to create an animation procedurally, the
/// next code snippet is for you.
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::{
/// #         math::curve::{Curve, CurveKey, CurveKeyKind},
/// #         pool::Handle,
/// #     },
/// #     scene::{animation::prelude::*, base::BaseBuilder, graph::Graph, node::Node},
/// # };
///
/// fn create_bounce_animation(animated_node: Handle<Node>) -> Animation {
///     let mut frames_container = TrackDataContainer::new(TrackValueKind::Vector3);
///
///     // We'll animate only Y coordinate (at index 1).
///     frames_container.curves_mut()[1] = Curve::from(vec![
///         CurveKey::new(0.1, 1.0, CurveKeyKind::Linear),
///         CurveKey::new(0.2, 0.0, CurveKeyKind::Linear),
///         CurveKey::new(0.3, 0.75, CurveKeyKind::Linear),
///         CurveKey::new(0.4, 0.0, CurveKeyKind::Linear),
///         CurveKey::new(0.5, 0.25, CurveKeyKind::Linear),
///         CurveKey::new(0.6, 0.0, CurveKeyKind::Linear),
///     ]);
///
///     // Create a track that will animated the node using the curve above.
///     let mut track = Track::new(frames_container, ValueBinding::Position);
///     track.set_target(animated_node);
///
///     // Finally create an animation and set its time slice and turn it on.
///     let mut animation = Animation::default();
///     animation.add_track(track);
///     animation.set_time_slice(0.0..0.6);
///     animation.set_enabled(true);
///     animation
/// }
///
/// fn create_bounce_animation_player(
///     animated_node: Handle<Node>,
///     graph: &mut Graph,
/// ) -> Handle<Node> {
///     let mut animations = AnimationContainer::new();
///
///     // Create a bounce animation.
///     animations.add(create_bounce_animation(animated_node));
///
///     AnimationPlayerBuilder::new(BaseBuilder::new())
///         .with_animations(animations)
///         .build(graph)
/// }
/// ```
///
/// As you can see, the example is quite big. That's why you should always prefer using the editor to create animations.
/// The example creates a bounce animation first - it is a simple animation that animates position of a given node
/// (`animated_node`). Only then it creates an animation player node with an animation container with a single animation.
/// To understand why this is so complicated, see the docs of [`Animation`].
#[derive(Visit, Reflect, Clone, Debug)]
pub struct AnimationPlayer {
    base: Base,
    animations: InheritableVariable<AnimationContainer>,
    auto_apply: bool,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            base: Default::default(),
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
}

impl TypeUuidProvider for AnimationPlayer {
    fn type_uuid() -> Uuid {
        uuid!("44d1c94e-354f-4f9a-b918-9d31c28aa16a")
    }
}

impl Deref for AnimationPlayer {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for AnimationPlayer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for AnimationPlayer {
    crate::impl_query_component!(
        animations: InheritableVariable<AnimationContainer>,
        auto_apply: bool
    );

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
        self.animations.get_value_mut_silent().update_animations(
            context.nodes,
            self.auto_apply,
            context.dt,
        );
    }
}

/// A builder for [`AnimationPlayer`] node.
pub struct AnimationPlayerBuilder {
    base_builder: BaseBuilder,
    animations: AnimationContainer,
    auto_apply: bool,
}

impl AnimationPlayerBuilder {
    /// Creates new builder instance.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
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
    pub fn build_node(self) -> Node {
        Node::new(AnimationPlayer {
            base: self.base_builder.build_base(),
            animations: self.animations.into(),
            auto_apply: self.auto_apply,
        })
    }

    /// Creates an instance of [`AnimationPlayer`] node and adds it to the given scene graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
