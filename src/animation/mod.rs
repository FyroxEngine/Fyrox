use crate::{
    animation::{track::Track, value::BoundValueCollection},
    core::{
        math::wrapf,
        pool::{Handle, Pool, Ticket},
        reflect::prelude::*,
        uuid::Uuid,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        graph::{Graph, NodePool},
        node::Node,
    },
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use std::{
    collections::{hash_map::Entry, VecDeque},
    fmt::Debug,
    ops::{Index, IndexMut, Range},
};

pub mod container;
pub mod machine;
pub mod spritesheet;
pub mod track;
pub mod value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AnimationEvent {
    pub signal_id: Uuid,
}

#[derive(Clone, Debug, Visit, Reflect, PartialEq)]
pub struct AnimationSignal {
    pub id: Uuid,
    pub name: String,
    pub time: f32,
    pub enabled: bool,
}

impl Default for AnimationSignal {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Default::default(),
            time: 0.0,
            enabled: true,
        }
    }
}

pub type NodeTrack = Track<Handle<Node>>;

#[derive(Debug, Reflect, Visit, PartialEq)]
pub struct Animation {
    #[visit(optional)]
    name: String,
    tracks: Vec<NodeTrack>,
    time_position: f32,
    #[visit(optional)]
    time_slice: Range<f32>,
    speed: f32,
    looped: bool,
    enabled: bool,
    signals: Vec<AnimationSignal>,

    // Non-serialized
    #[reflect(hidden)]
    #[visit(skip)]
    pose: AnimationPose,
    // Non-serialized
    #[reflect(hidden)]
    #[visit(skip)]
    events: VecDeque<AnimationEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalPose {
    node: Handle<Node>,
    values: BoundValueCollection,
}

impl Default for LocalPose {
    fn default() -> Self {
        Self {
            node: Handle::NONE,
            values: Default::default(),
        }
    }
}

impl LocalPose {
    fn weighted_clone(&self, weight: f32) -> Self {
        Self {
            node: self.node,
            values: self.values.weighted_clone(weight),
        }
    }

    pub fn blend_with(&mut self, other: &LocalPose, weight: f32) {
        self.values.blend_with(&other.values, weight)
    }

    pub fn values(&self) -> &BoundValueCollection {
        &self.values
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct AnimationPose {
    local_poses: FxHashMap<Handle<Node>, LocalPose>,
}

impl AnimationPose {
    pub fn clone_into(&self, dest: &mut AnimationPose) {
        dest.reset();
        for (handle, local_pose) in self.local_poses.iter() {
            dest.local_poses.insert(*handle, local_pose.clone());
        }
    }

    pub fn blend_with(&mut self, other: &AnimationPose, weight: f32) {
        for (handle, other_pose) in other.local_poses.iter() {
            if let Some(current_pose) = self.local_poses.get_mut(handle) {
                current_pose.blend_with(other_pose, weight);
            } else {
                // There are no corresponding local pose, do fake blend between identity
                // pose and other.
                self.add_local_pose(other_pose.weighted_clone(weight));
            }
        }
    }

    fn add_local_pose(&mut self, local_pose: LocalPose) {
        self.local_poses.insert(local_pose.node, local_pose);
    }

    pub fn reset(&mut self) {
        self.local_poses.clear();
    }

    pub(crate) fn apply_internal(&self, nodes: &mut NodePool) {
        for (node, local_pose) in self.local_poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = nodes.try_borrow_mut(*node) {
                local_pose.values.apply(node);
            }
        }
    }

    pub fn apply(&self, graph: &mut Graph) {
        for (node, local_pose) in self.local_poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node) = graph.try_get_mut(*node) {
                local_pose.values.apply(node);
            }
        }
    }

    /// Calls given callback function for each node and allows you to apply pose with your own
    /// rules. This could be useful if you need to ignore transform some part of pose for a node.
    pub fn apply_with<C>(&self, graph: &mut Graph, mut callback: C)
    where
        C: FnMut(&mut Node, Handle<Node>, &LocalPose),
    {
        for (node, local_pose) in self.local_poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else if let Some(node_ref) = graph.try_get_mut(*node) {
                callback(node_ref, *node, local_pose);
            }
        }
    }
}

impl Clone for Animation {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            tracks: self.tracks.clone(),
            speed: self.speed,
            time_position: self.time_position,
            looped: self.looped,
            enabled: self.enabled,
            pose: Default::default(),
            signals: self.signals.clone(),
            events: Default::default(),
            time_slice: self.time_slice.clone(),
        }
    }
}

impl Animation {
    pub fn set_name<S: AsRef<str>>(&mut self, name: S) {
        self.name = name.as_ref().to_owned();
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Adds new track to the animation. Animation can have unlimited number of tracks, each track is responsible
    /// for animation of a single scene node.
    pub fn add_track(&mut self, track: NodeTrack) {
        self.tracks.push(track);
    }

    /// Removes a track at given index.
    pub fn remove_track(&mut self, index: usize) -> NodeTrack {
        self.tracks.remove(index)
    }

    /// Inserts a track at given index.
    pub fn insert_track(&mut self, index: usize, track: NodeTrack) {
        self.tracks.insert(index, track)
    }

    /// Removes last track from the list of tracks of the animation.
    pub fn pop_track(&mut self) -> Option<NodeTrack> {
        self.tracks.pop()
    }

    /// Calculates new length of the animation based on the content of its tracks. It looks for the most "right"
    /// curve key in all curves of all tracks and treats it as length of the animation. The method could be used
    /// in case if you formed animation from code using just curves and don't know the actual length of the
    /// animation.  
    pub fn fit_length_to_content(&mut self) {
        self.time_slice.start = 0.0;
        for track in self.tracks.iter_mut() {
            if track.time_length() > self.time_slice.end {
                self.time_slice.end = track.time_length();
            }
        }
    }

    pub fn tracks(&self) -> &[NodeTrack] {
        &self.tracks
    }

    pub fn set_time_position(&mut self, time: f32) -> &mut Self {
        if self.looped {
            self.time_position = wrapf(time, self.time_slice.start, self.time_slice.end);
        } else {
            self.time_position = time.clamp(self.time_slice.start, self.time_slice.end);
        }

        self
    }

    /// Sets new time slice of the animation in seconds. It defines a time interval in which the animation will
    /// be played. Current playback position will be clamped to fit to new bounds.
    pub fn set_time_slice(&mut self, time_slice: Range<f32>) {
        assert!(time_slice.start <= time_slice.end);

        self.time_slice = time_slice;

        // Ensure time position is in given time slice.
        self.set_time_position(self.time_position);
    }

    pub fn time_slice(&self) -> Range<f32> {
        self.time_slice.clone()
    }

    pub fn rewind(&mut self) -> &mut Self {
        self.set_time_position(0.0)
    }

    /// Returns length of the animation in seconds.
    pub fn length(&self) -> f32 {
        self.time_slice.end - self.time_slice.start
    }

    pub(crate) fn tick(&mut self, dt: f32) {
        self.update_pose();

        let current_time_position = self.time_position();
        let new_time_position = current_time_position + dt * self.speed();

        for signal in self.signals.iter_mut().filter(|s| s.enabled) {
            if self.speed >= 0.0
                && (current_time_position < signal.time && new_time_position >= signal.time)
                || self.speed < 0.0
                    && (current_time_position > signal.time && new_time_position <= signal.time)
            {
                // TODO: Make this configurable.
                if self.events.len() < 32 {
                    self.events.push_back(AnimationEvent {
                        signal_id: signal.id,
                    });
                }
            }
        }

        self.set_time_position(new_time_position);
    }

    pub fn pop_event(&mut self) -> Option<AnimationEvent> {
        self.events.pop_front()
    }

    pub fn time_position(&self) -> f32 {
        self.time_position
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_loop(&mut self, state: bool) -> &mut Self {
        self.looped = state;
        self
    }

    pub fn is_loop(&self) -> bool {
        self.looped
    }

    pub fn has_ended(&self) -> bool {
        !self.looped && (self.time_position - self.time_slice.end).abs() <= f32::EPSILON
    }

    pub fn set_enabled(&mut self, enabled: bool) -> &mut Self {
        self.enabled = enabled;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    pub fn tracks_mut(&mut self) -> &mut [NodeTrack] {
        &mut self.tracks
    }

    pub fn add_signal(&mut self, signal: AnimationSignal) -> &mut Self {
        self.signals.push(signal);
        self
    }

    pub fn pop_signal(&mut self) -> Option<AnimationSignal> {
        self.signals.pop()
    }

    pub fn insert_signal(&mut self, index: usize, signal: AnimationSignal) {
        self.signals.insert(index, signal)
    }

    pub fn remove_signal(&mut self, index: usize) -> AnimationSignal {
        self.signals.remove(index)
    }

    pub fn signals(&self) -> &[AnimationSignal] {
        &self.signals
    }

    pub fn signals_mut(&mut self) -> &mut [AnimationSignal] {
        &mut self.signals
    }

    pub fn retain_tracks<F>(&mut self, filter: F)
    where
        F: FnMut(&NodeTrack) -> bool,
    {
        self.tracks.retain(filter)
    }

    /// Enables or disables animation tracks for nodes in hierarchy starting from given root.
    /// Could be useful to enable or disable animation for skeleton parts, i.e. you don't want
    /// legs to be animated and you know that legs starts from torso bone, then you could do
    /// this.
    ///
    /// ```
    /// use fyrox::scene::node::Node;
    /// use fyrox::animation::Animation;
    /// use fyrox::core::pool::Handle;
    /// use fyrox::scene::graph::Graph;
    ///
    /// fn disable_legs(torso_bone: Handle<Node>, aim_animation: &mut Animation, graph: &Graph) {
    ///     aim_animation.set_tracks_enabled_from(torso_bone, false, graph)
    /// }
    /// ```
    ///
    /// After this legs won't be animated and animation could be blended together with run
    /// animation so it will produce new animation - run and aim.
    pub fn set_tracks_enabled_from(&mut self, handle: Handle<Node>, enabled: bool, graph: &Graph) {
        let mut stack = vec![handle];
        while let Some(node) = stack.pop() {
            for track in self.tracks.iter_mut() {
                if track.target() == node {
                    track.enable(enabled);
                }
            }
            for child in graph[node].children() {
                stack.push(*child);
            }
        }
    }

    pub fn set_node_track_enabled(&mut self, handle: Handle<Node>, enabled: bool) {
        for track in self.tracks.iter_mut() {
            if track.target() == handle {
                track.enable(enabled);
            }
        }
    }

    pub fn tracks_of(&self, handle: Handle<Node>) -> impl Iterator<Item = &NodeTrack> {
        self.tracks
            .iter()
            .filter(move |track| track.target() == handle)
    }

    pub fn tracks_of_mut(&mut self, handle: Handle<Node>) -> impl Iterator<Item = &mut NodeTrack> {
        self.tracks
            .iter_mut()
            .filter(move |track| track.target() == handle)
    }

    pub fn remove_tracks(&mut self) {
        self.tracks.clear();
        self.time_slice = 0.0..0.0;
    }

    fn update_pose(&mut self) {
        self.pose.reset();
        for track in self.tracks.iter() {
            if track.is_enabled() {
                if let Some(bound_value) = track.fetch(self.time_position) {
                    match self.pose.local_poses.entry(track.target()) {
                        Entry::Occupied(entry) => {
                            entry.into_mut().values.values.push(bound_value);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(LocalPose {
                                node: track.target(),
                                values: BoundValueCollection {
                                    values: vec![bound_value],
                                },
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn pose(&self) -> &AnimationPose {
        &self.pose
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            name: Default::default(),
            tracks: Vec::new(),
            speed: 1.0,
            time_position: 0.0,
            enabled: true,
            looped: true,
            pose: Default::default(),
            signals: Default::default(),
            events: Default::default(),
            time_slice: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Reflect, PartialEq)]
pub struct AnimationContainer {
    pool: Pool<Animation>,
}

impl Default for AnimationContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationContainer {
    pub(crate) fn new() -> Self {
        Self { pool: Pool::new() }
    }

    #[inline]
    pub fn alive_count(&self) -> u32 {
        self.pool.alive_count()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Animation> {
        self.pool.iter()
    }

    #[inline]
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Animation>, &Animation)> {
        self.pool.pair_iter()
    }

    #[inline]
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Animation>, &mut Animation)> {
        self.pool.pair_iter_mut()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Animation> {
        self.pool.iter_mut()
    }

    #[inline]
    pub fn add(&mut self, animation: Animation) -> Handle<Animation> {
        self.pool.spawn(animation)
    }

    #[inline]
    pub fn remove(&mut self, handle: Handle<Animation>) -> Option<Animation> {
        self.pool.try_free(handle)
    }

    /// Extracts animation from container and reserves its handle. It is used to temporarily take
    /// ownership over animation, and then put animation back using given ticket.
    pub fn take_reserve(&mut self, handle: Handle<Animation>) -> (Ticket<Animation>, Animation) {
        self.pool.take_reserve(handle)
    }

    /// Puts animation back by given ticket.
    pub fn put_back(
        &mut self,
        ticket: Ticket<Animation>,
        animation: Animation,
    ) -> Handle<Animation> {
        self.pool.put_back(ticket, animation)
    }

    /// Makes animation handle vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<Animation>) {
        self.pool.forget_ticket(ticket)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    #[inline]
    pub fn get(&self, handle: Handle<Animation>) -> &Animation {
        self.pool.borrow(handle)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<Animation>) -> &mut Animation {
        self.pool.borrow_mut(handle)
    }

    #[inline]
    pub fn try_get(&self, handle: Handle<Animation>) -> Option<&Animation> {
        self.pool.try_borrow(handle)
    }

    #[inline]
    pub fn try_get_mut(&mut self, handle: Handle<Animation>) -> Option<&mut Animation> {
        self.pool.try_borrow_mut(handle)
    }

    #[inline]
    pub fn retain<P>(&mut self, pred: P)
    where
        P: FnMut(&Animation) -> bool,
    {
        self.pool.retain(pred)
    }

    pub fn update_animations(&mut self, nodes: &mut NodePool, apply: bool, dt: f32) {
        for animation in self.pool.iter_mut().filter(|anim| anim.enabled) {
            animation.tick(dt);
            if apply {
                animation.pose.apply_internal(nodes);
            }
        }
    }

    /// Removes queued animation events from every animation in the container.
    ///
    /// # Potential use cases
    ///
    /// Sometimes there is a need to use animation events only from one frame,
    /// in this case you should clear events each frame. This situation might come up
    /// when you have multiple animations with signals, but at each frame not every
    /// event gets processed. This might result in unwanted side effects, like multiple
    /// attack events may result in huge damage in a single frame.
    pub fn clear_animation_events(&mut self) {
        for animation in self.pool.iter_mut() {
            animation.events.clear();
        }
    }
}

impl Visit for AnimationContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() && self.pool.get_capacity() != 0 {
            panic!("Animation pool must be empty on load!");
        }

        let mut region = visitor.enter_region(name)?;

        self.pool.visit("Pool", &mut region)?;

        Ok(())
    }
}

impl Index<Handle<Animation>> for AnimationContainer {
    type Output = Animation;

    fn index(&self, index: Handle<Animation>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Animation>> for AnimationContainer {
    fn index_mut(&mut self, index: Handle<Animation>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}
