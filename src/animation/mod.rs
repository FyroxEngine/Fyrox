pub mod machine;

use crate::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        math::{clampf, wrapf},
        pool::{
            Handle, Pool, PoolIterator, PoolIteratorMut, PoolPairIterator, PoolPairIteratorMut,
            Ticket,
        },
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::{model::Model, ResourceState},
    scene::{graph::Graph, node::Node},
    utils::log::{Log, MessageKind},
};
use std::{
    collections::{HashMap, VecDeque},
    ops::{Index, IndexMut},
};

#[derive(Copy, Clone, Debug)]
pub struct KeyFrame {
    pub position: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub time: f32,
}

impl KeyFrame {
    pub fn new(
        time: f32,
        position: Vector3<f32>,
        scale: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
    ) -> Self {
        Self {
            time,
            position,
            scale,
            rotation,
        }
    }
}

impl Default for KeyFrame {
    fn default() -> Self {
        Self {
            position: Default::default(),
            scale: Default::default(),
            rotation: Default::default(),
            time: 0.0,
        }
    }
}

impl Visit for KeyFrame {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.scale.visit("Scale", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        self.time.visit("Time", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PoseEvaluationFlags {
    pub ignore_position: bool,
    pub ignore_rotation: bool,
    pub ignore_scale: bool,
}

impl Visit for PoseEvaluationFlags {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.ignore_position.visit("IgnorePosition", visitor)?;
        self.ignore_rotation.visit("IgnoreRotation", visitor)?;
        self.ignore_scale.visit("IgnoreScale", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Debug)]
pub struct Track {
    // Frames are not serialized, because it makes no sense to store them in save file,
    // they will be taken from resource on Resolve stage.
    frames: Vec<KeyFrame>,
    enabled: bool,
    max_time: f32,
    node: Handle<Node>,
    flags: PoseEvaluationFlags,
}

impl Clone for Track {
    fn clone(&self) -> Self {
        Self {
            frames: self.frames.clone(),
            enabled: self.enabled,
            max_time: self.max_time,
            node: self.node,
            flags: self.flags,
        }
    }
}

impl Default for Track {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            enabled: true,
            max_time: 0.0,
            node: Default::default(),
            flags: Default::default(),
        }
    }
}

impl Visit for Track {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.enabled.visit("Enabled", visitor)?;
        self.max_time.visit("MaxTime", visitor)?;
        self.node.visit("Node", visitor)?;
        let _ = self.flags.visit("Flags", visitor);

        visitor.leave_region()
    }
}

impl Track {
    pub fn new() -> Track {
        Default::default()
    }

    pub fn set_node(&mut self, node: Handle<Node>) {
        self.node = node;
    }

    pub fn get_node(&self) -> Handle<Node> {
        self.node
    }

    pub fn add_key_frame(&mut self, key_frame: KeyFrame) {
        if key_frame.time > self.max_time {
            self.frames.push(key_frame);

            self.max_time = key_frame.time;
        } else {
            // Find a place to insert
            let mut index = 0;
            for (i, other_key_frame) in self.frames.iter().enumerate() {
                if key_frame.time < other_key_frame.time {
                    index = i;
                    break;
                }
            }

            self.frames.insert(index, key_frame)
        }
    }

    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_key_frames(&mut self, key_frames: &[KeyFrame]) {
        self.frames = key_frames.to_vec();
        self.max_time = 0.0;

        for key_frame in self.frames.iter() {
            if key_frame.time > self.max_time {
                self.max_time = key_frame.time;
            }
        }
    }

    pub fn get_key_frames(&self) -> &[KeyFrame] {
        &self.frames
    }

    pub fn get_local_pose(&self, mut time: f32) -> Option<LocalPose> {
        if self.frames.is_empty() {
            return None;
        }

        if time >= self.max_time {
            return self.frames.last().map(|k| LocalPose {
                node: self.node,
                position: k.position,
                scale: k.scale,
                rotation: k.rotation,
            });
        }

        time = clampf(time, 0.0, self.max_time);

        let mut right_index = 0;
        for (i, keyframe) in self.frames.iter().enumerate() {
            if keyframe.time >= time {
                right_index = i;
                break;
            }
        }

        if right_index == 0 {
            self.frames.first().map(|k| LocalPose {
                node: self.node,
                position: k.position,
                scale: k.scale,
                rotation: k.rotation,
            })
        } else {
            let left = &self.frames[right_index - 1];
            let right = &self.frames[right_index];
            let interpolator = (time - left.time) / (right.time - left.time);

            Some(LocalPose {
                node: self.node,
                position: if self.flags.ignore_position {
                    Vector3::new(0.0, 0.0, 0.0)
                } else {
                    left.position.lerp(&right.position, interpolator)
                },
                scale: if self.flags.ignore_scale {
                    Vector3::new(1.0, 1.0, 1.0)
                } else {
                    left.scale.lerp(&right.scale, interpolator)
                },
                rotation: if self.flags.ignore_rotation {
                    UnitQuaternion::default()
                } else {
                    left.rotation.nlerp(&right.rotation, interpolator)
                },
            })
        }
    }

    pub fn flags(&self) -> PoseEvaluationFlags {
        self.flags
    }

    pub fn set_flags(&mut self, flags: PoseEvaluationFlags) {
        self.flags = flags;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AnimationEvent {
    pub signal_id: u64,
}

#[derive(Clone, Debug)]
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

impl Visit for AnimationSignal {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.id.visit("Id", visitor)?;
        self.time.visit("Time", visitor)?;
        self.enabled.visit("Enabled", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Debug)]
pub struct Animation {
    // TODO: Extract into separate struct AnimationTimeline
    tracks: Vec<Track>,
    length: f32,
    time_position: f32,
    ///////////////////////////////////////////////////////
    speed: f32,
    looped: bool,
    enabled: bool,
    pub(in crate) resource: Option<Model>,
    pose: AnimationPose,
    signals: Vec<AnimationSignal>,
    events: VecDeque<AnimationEvent>,
}

/// Snapshot of scene node local transform state.
#[derive(Clone, Debug)]
pub struct LocalPose {
    node: Handle<Node>,
    position: Vector3<f32>,
    scale: Vector3<f32>,
    rotation: UnitQuaternion<f32>,
}

impl Default for LocalPose {
    fn default() -> Self {
        Self {
            node: Handle::NONE,
            position: Vector3::default(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            rotation: UnitQuaternion::identity(),
        }
    }
}

impl LocalPose {
    fn weighted_clone(&self, weight: f32) -> Self {
        Self {
            node: self.node,
            position: self.position.scale(weight),
            rotation: UnitQuaternion::identity().nlerp(&self.rotation, weight),
            scale: Vector3::new(1.0, 1.0, 1.0), // TODO: Implement scale blending
        }
    }

    pub fn blend_with(&mut self, other: &LocalPose, weight: f32) {
        self.position += other.position.scale(weight);
        self.rotation = self.rotation.nlerp(&other.rotation, weight);
        // TODO: Implement scale blending
    }

    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    pub fn scale(&self) -> Vector3<f32> {
        self.scale
    }

    pub fn rotation(&self) -> UnitQuaternion<f32> {
        self.rotation
    }
}

#[derive(Default, Debug)]
pub struct AnimationPose {
    local_poses: HashMap<Handle<Node>, LocalPose>,
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
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargetting failed!".to_owned());
            } else {
                graph[*node]
                    .local_transform_mut()
                    .set_position(local_pose.position)
                    .set_rotation(local_pose.rotation)
                    .set_scale(local_pose.scale);
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
                Log::writeln(MessageKind::Error, "Invalid node handle found for animation pose, most likely it means that animation retargetting failed!".to_owned());
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
        }
    }
}

impl Animation {
    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track);

        for track in self.tracks.iter_mut() {
            if track.max_time > self.length {
                self.length = track.max_time;
            }
        }
    }

    pub fn get_tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn set_time_position(&mut self, time: f32) -> &mut Self {
        if self.looped {
            self.time_position = wrapf(time, 0.0, self.length);
        } else {
            self.time_position = clampf(time, 0.0, self.length);
        }
        self
    }

    pub fn rewind(&mut self) -> &mut Self {
        self.set_time_position(0.0)
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    fn tick(&mut self, dt: f32) {
        self.update_pose();

        let current_time_position = self.get_time_position();
        let new_time_position = current_time_position + dt * self.get_speed();

        for signal in self.signals.iter_mut() {
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

    pub fn get_time_position(&self) -> f32 {
        self.time_position
    }

    pub fn get_speed(&self) -> f32 {
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
        !self.looped && (self.time_position - self.length).abs() <= std::f32::EPSILON
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

    pub fn get_tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks
    }

    pub fn get_resource(&self) -> Option<Model> {
        self.resource.clone()
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
    /// use rg3d::scene::node::Node;
    /// use rg3d::animation::Animation;
    /// use rg3d::core::pool::Handle;
    /// use rg3d::scene::graph::Graph;
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
                if track.node == node {
                    track.enabled = enabled;
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
            if track.node == handle {
                track.enabled = enabled;
            }
        }
    }

    pub fn track_of(&self, handle: Handle<Node>) -> Option<&Track> {
        for track in self.tracks.iter() {
            if track.node == handle {
                return Some(track);
            }
        }
        None
    }

    pub fn track_of_mut(&mut self, handle: Handle<Node>) -> Option<&mut Track> {
        for track in self.tracks.iter_mut() {
            if track.node == handle {
                return Some(track);
            }
        }
        None
    }

    pub(in crate) fn resolve(&mut self, graph: &Graph) {
        // Copy key frames from resource for each animation. This is needed because we
        // do not store key frames in save file, but just keep reference to resource
        // from which key frames should be taken on load.
        if let Some(resource) = self.resource.clone() {
            let resource = resource.state();
            if let ResourceState::Ok(ref data) = *resource {
                // TODO: Here we assume that resource contains only *one* animation.
                if let Some(ref_animation) = data.get_scene().animations.pool.at(0) {
                    for track in self.get_tracks_mut() {
                        // This may panic if animation has track that refers to a deleted node,
                        // it can happen if you deleted a node but forgot to remove animation
                        // that uses this node.
                        let track_node = &graph[track.get_node()];

                        // Find corresponding track in resource using names of nodes, not
                        // original handles of instantiated nodes. We can't use original
                        // handles here because animation can be targetted to a node that
                        // wasn't instantiated from animation resource. It can be instantiated
                        // from some other resource. For example you have a character with
                        // multiple animations. Character "lives" in its own file without animations
                        // but with skin. Each animation "lives" in its own file too, then
                        // you did animation retargetting from animation resource to your character
                        // instantiated model, which is essentially copies key frames to new
                        // animation targetted to character instance.
                        let mut found = false;
                        for ref_track in ref_animation.get_tracks().iter() {
                            if track_node.name()
                                == data.get_scene().graph[ref_track.get_node()].name()
                            {
                                track.set_key_frames(ref_track.get_key_frames());
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
            } else {
                unreachable!()
            }
        }
    }

    fn update_pose(&mut self) {
        self.pose.reset();
        for track in self.tracks.iter() {
            if track.is_enabled() {
                if let Some(local_pose) = track.get_local_pose(self.time_position) {
                    self.pose.add_local_pose(local_pose);
                }
            }
        }
    }

    pub fn get_pose(&self) -> &AnimationPose {
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
        }
    }
}

impl Visit for Animation {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.tracks.visit("Tracks", visitor)?;
        self.speed.visit("Speed", visitor)?;
        self.length.visit("Length", visitor)?;
        self.time_position.visit("TimePosition", visitor)?;
        self.resource.visit("Resource", visitor)?;
        self.looped.visit("Looped", visitor)?;
        self.enabled.visit("Enabled", visitor)?;
        self.signals.visit("Signals", visitor)?;

        visitor.leave_region()
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
    pub(in crate) fn new() -> Self {
        Self { pool: Pool::new() }
    }

    #[inline]
    pub fn iter(&self) -> PoolIterator<Animation> {
        self.pool.iter()
    }

    #[inline]
    pub fn pair_iter(&self) -> PoolPairIterator<Animation> {
        self.pool.pair_iter()
    }

    #[inline]
    pub fn pair_iter_mut(&mut self) -> PoolPairIteratorMut<Animation> {
        self.pool.pair_iter_mut()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<Animation> {
        self.pool.iter_mut()
    }

    #[inline]
    pub fn add(&mut self, animation: Animation) -> Handle<Animation> {
        self.pool.spawn(animation)
    }

    #[inline]
    pub fn remove(&mut self, handle: Handle<Animation>) {
        self.pool.free(handle);
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
    pub fn retain<P>(&mut self, pred: P)
    where
        P: FnMut(&Animation) -> bool,
    {
        self.pool.retain(pred)
    }

    pub fn resolve(&mut self, graph: &Graph) {
        Log::writeln(
            MessageKind::Information,
            "Resolving animations...".to_owned(),
        );
        for animation in self.pool.iter_mut() {
            animation.resolve(graph)
        }
        Log::writeln(
            MessageKind::Information,
            "Animations resolved successfully!".to_owned(),
        );
    }

    pub fn update_animations(&mut self, dt: f32) {
        for animation in self.pool.iter_mut().filter(|anim| anim.enabled) {
            animation.tick(dt);
        }
    }
}

impl Visit for AnimationContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() && self.pool.get_capacity() != 0 {
            panic!("Animation pool must be empty on load!");
        }

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
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
