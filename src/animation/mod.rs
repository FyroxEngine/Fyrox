use crate::{
    animation::{track::Track, value::BoundValueCollection},
    asset::ResourceState,
    core::{
        math::wrapf,
        pool::{Handle, Pool, Ticket},
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::model::Model,
    scene::{graph::Graph, node::Node},
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
    pub signal_id: u64,
}

#[derive(Clone, Debug, Visit)]
pub struct AnimationSignal {
    id: u64,
    time: f32,
    enabled: bool,
}

impl AnimationSignal {
    pub fn new(id: u64, time: f32) -> Self {
        Self {
            id,
            time,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, value: bool) {
        self.enabled = value;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn time(&self) -> f32 {
        self.time
    }
}

impl Default for AnimationSignal {
    fn default() -> Self {
        Self {
            id: 0,
            time: 0.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Visit)]
pub struct Animation {
    // TODO: Extract into separate struct AnimationTimeline
    tracks: Vec<Track>,
    length: f32,
    time_position: f32,
    #[visit(optional)] // Backward compatibility
    time_slice: Option<Range<f32>>,
    ///////////////////////////////////////////////////////
    speed: f32,
    looped: bool,
    enabled: bool,
    pub(crate) resource: Option<Model>,
    #[visit(skip)]
    pose: AnimationPose,
    signals: Vec<AnimationSignal>,
    #[visit(skip)]
    events: VecDeque<AnimationEvent>,
}

#[derive(Clone, Debug)]
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
}

#[derive(Default, Debug, Clone)]
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

    pub fn apply(&self, graph: &mut Graph) {
        for (node, local_pose) in self.local_poses.iter() {
            if node.is_none() {
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargeting failed!");
            } else {
                local_pose.values.apply(&mut graph[*node]);
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
            } else {
                callback(&mut graph[*node], *node, local_pose);
            }
        }
    }
}

impl Clone for Animation {
    fn clone(&self) -> Self {
        Self {
            tracks: self.tracks.clone(),
            speed: self.speed,
            length: self.length,
            time_position: self.time_position,
            looped: self.looped,
            enabled: self.enabled,
            resource: self.resource.clone(),
            pose: Default::default(),
            signals: self.signals.clone(),
            events: Default::default(),
            time_slice: self.time_slice.clone(),
        }
    }
}

impl Animation {
    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);

        for track in self.tracks.iter_mut() {
            if track.time_length() > self.length {
                self.length = track.time_length();
            }
        }
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn set_time_position(&mut self, time: f32) -> &mut Self {
        let time_slice = self.time_slice.clone().unwrap_or(Range {
            start: 0.0,
            end: self.length,
        });

        if self.looped {
            self.time_position = wrapf(time, time_slice.start, time_slice.end);
        } else {
            self.time_position = time.clamp(time_slice.start, time_slice.end);
        }

        self
    }

    pub fn set_time_slice(&mut self, time_slice: Option<Range<f32>>) {
        if let Some(time_slice) = time_slice.clone() {
            assert!(time_slice.start <= time_slice.end);
        }

        self.time_slice = time_slice;

        // Ensure time position is in given time slice.
        self.set_time_position(self.time_position);
    }

    pub fn rewind(&mut self) -> &mut Self {
        self.set_time_position(0.0)
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    fn tick(&mut self, dt: f32) {
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
        !self.looped && (self.time_position - self.length).abs() <= f32::EPSILON
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

    pub fn tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks
    }

    pub fn resource(&self) -> Option<Model> {
        self.resource.clone()
    }

    pub fn signals(&self) -> &[AnimationSignal] {
        &self.signals
    }

    pub fn retain_tracks<F>(&mut self, filter: F)
    where
        F: FnMut(&Track) -> bool,
    {
        self.tracks.retain(filter)
    }

    pub fn add_signal(&mut self, signal: AnimationSignal) -> &mut Self {
        self.signals.push(signal);
        self
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
                if track.node() == node {
                    track.enable(enabled);
                    break;
                }
            }
            for child in graph[node].children() {
                stack.push(*child);
            }
        }
    }

    pub fn set_node_track_enabled(&mut self, handle: Handle<Node>, enabled: bool) {
        for track in self.tracks.iter_mut() {
            if track.node() == handle {
                track.enable(enabled);
            }
        }
    }

    pub fn track_of(&self, handle: Handle<Node>) -> Option<&Track> {
        self.tracks.iter().find(|&track| track.node() == handle)
    }

    pub fn track_of_mut(&mut self, handle: Handle<Node>) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|track| track.node() == handle)
    }

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        if let Some(resource) = self.resource.as_mut() {
            let new_resource = resource_manager.request_model(resource.state().path());
            *resource = new_resource;
        }
    }

    pub(crate) fn resolve(&mut self, graph: &Graph) {
        // Copy key frames from resource for each animation. This is needed because we
        // do not store key frames in save file, but just keep reference to resource
        // from which key frames should be taken on load.
        if let Some(resource) = self.resource.clone() {
            let resource = resource.state();
            match *resource {
                ResourceState::Ok(ref data) => {
                    // TODO: Here we assume that resource contains only *one* animation.
                    if let Some(ref_animation) = data.get_scene().animations.pool.at(0) {
                        for track in self.tracks_mut() {
                            // This may panic if animation has track that refers to a deleted node,
                            // it can happen if you deleted a node but forgot to remove animation
                            // that uses this node.
                            let track_node = &graph[track.node()];

                            // Find corresponding track in resource using names of nodes, not
                            // original handles of instantiated nodes. We can't use original
                            // handles here because animation can be targeted to a node that
                            // wasn't instantiated from animation resource. It can be instantiated
                            // from some other resource. For example you have a character with
                            // multiple animations. Character "lives" in its own file without animations
                            // but with skin. Each animation "lives" in its own file too, then
                            // you did animation retargeting from animation resource to your character
                            // instantiated model, which is essentially copies key frames to new
                            // animation targeted to character instance.
                            let mut found = false;
                            for ref_track in ref_animation.tracks().iter() {
                                if track_node.name()
                                    == data.get_scene().graph[ref_track.node()].name()
                                {
                                    track
                                        .set_frames_container(ref_track.frames_container().clone());
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                Log::write(
                                    MessageKind::Error,
                                    format!(
                                        "Failed to copy key frames for node {}!",
                                        track_node.name()
                                    ),
                                );
                            }
                        }
                    }
                }
                ResourceState::LoadError {
                    ref path,
                    ref error,
                } => Log::err(format!(
                    "Unable to restore animation key frames from {} resource. Reason: {:?}",
                    path.display(),
                    error
                )),
                ResourceState::Pending { ref path, .. } => {
                    panic!(
                        "Animation resource {} must be fully loaded before resolving!",
                        path.display()
                    )
                }
            }
        }
    }

    fn update_pose(&mut self) {
        self.pose.reset();
        for track in self.tracks.iter() {
            if track.is_enabled() {
                if let Some(bound_value) = track.fetch(self.time_position) {
                    match self.pose.local_poses.entry(track.node()) {
                        Entry::Occupied(entry) => {
                            entry.into_mut().values.values.push(bound_value);
                        }
                        Entry::Vacant(entry) => {
                            entry.insert(LocalPose {
                                node: track.node(),
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
            tracks: Vec::new(),
            speed: 1.0,
            length: 0.0,
            time_position: 0.0,
            enabled: true,
            looped: true,
            resource: Default::default(),
            pose: Default::default(),
            signals: Default::default(),
            events: Default::default(),
            time_slice: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
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

    pub fn resolve(&mut self, graph: &Graph) {
        Log::writeln(MessageKind::Information, "Resolving animations...");
        for animation in self.pool.iter_mut() {
            animation.resolve(graph)
        }
        Log::writeln(
            MessageKind::Information,
            "Animations resolved successfully!",
        );
    }

    pub fn update_animations(&mut self, dt: f32) {
        for animation in self.pool.iter_mut().filter(|anim| anim.enabled) {
            animation.tick(dt);
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
