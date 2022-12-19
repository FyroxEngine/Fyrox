use crate::{
    animation::track::Track,
    core::{
        math::wrapf,
        pool::{Handle, Pool, Ticket},
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        graph::{Graph, NodePool},
        node::Node,
    },
    utils::{self, NameProvider},
};
use std::{
    collections::VecDeque,
    fmt::Debug,
    ops::{Index, IndexMut, Range},
};

pub use pose::{AnimationPose, NodePose};
pub use signal::{AnimationEvent, AnimationSignal};

pub mod container;
pub mod machine;
pub mod pose;
pub mod signal;
pub mod spritesheet;
pub mod track;
pub mod value;

#[derive(Debug, Reflect, Visit, PartialEq)]
pub struct Animation {
    #[visit(optional)]
    name: String,
    tracks: Vec<Track>,
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

impl NameProvider for Animation {
    fn name(&self) -> &str {
        &self.name
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
    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);
    }

    /// Removes a track at given index.
    pub fn remove_track(&mut self, index: usize) -> Track {
        self.tracks.remove(index)
    }

    /// Inserts a track at given index.
    pub fn insert_track(&mut self, index: usize, track: Track) {
        self.tracks.insert(index, track)
    }

    /// Removes last track from the list of tracks of the animation.
    pub fn pop_track(&mut self) -> Option<Track> {
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

    pub fn tracks(&self) -> &[Track] {
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

    pub fn events_ref(&self) -> &VecDeque<AnimationEvent> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut VecDeque<AnimationEvent> {
        &mut self.events
    }

    pub fn events(&self) -> VecDeque<AnimationEvent> {
        self.events.clone()
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

    pub fn tracks_mut(&mut self) -> &mut [Track] {
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
        F: FnMut(&Track) -> bool,
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

    pub fn tracks_of(&self, handle: Handle<Node>) -> impl Iterator<Item = &Track> {
        self.tracks
            .iter()
            .filter(move |track| track.target() == handle)
    }

    pub fn tracks_of_mut(&mut self, handle: Handle<Node>) -> impl Iterator<Item = &mut Track> {
        self.tracks
            .iter_mut()
            .filter(move |track| track.target() == handle)
    }

    /// Tries to find a layer by its name. Returns index of the signal and its reference.
    #[inline]
    pub fn find_signal_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(usize, &AnimationSignal)> {
        utils::find_by_name_ref(self.signals.iter().enumerate(), name)
    }

    /// Tries to find a signal by its name. Returns index of the signal and its reference.
    #[inline]
    pub fn find_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(usize, &mut AnimationSignal)> {
        utils::find_by_name_mut(self.signals.iter_mut().enumerate(), name)
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
                    self.pose.add_to_node_pose(track.target(), bound_value);
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
    pub fn find_by_name_ref<S: AsRef<str>>(
        &self,
        name: S,
    ) -> Option<(Handle<Animation>, &Animation)> {
        utils::find_by_name_ref(self.pool.pair_iter(), name)
    }

    #[inline]
    pub fn find_by_name_mut<S: AsRef<str>>(
        &mut self,
        name: S,
    ) -> Option<(Handle<Animation>, &mut Animation)> {
        utils::find_by_name_mut(self.pool.pair_iter_mut(), name)
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
