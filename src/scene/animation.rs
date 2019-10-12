use rg3d_core::{
    math::{
        vec3::Vec3,
        quat::Quat,
        clampf,
        lerpf,
        wrapf,
    },
    visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
    pool::{
        Pool,
        Handle,
        PoolIterator,
        PoolIteratorMut,
    },
};
use std::sync::{Mutex, Arc};
use crate::{
    scene::{
        SceneInterface,
        node::Node,
        graph::Graph,
    },
    resource::model::Model,
};

#[derive(Copy, Clone)]
pub struct KeyFrame {
    pub position: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
    pub time: f32,
}

impl KeyFrame {
    pub fn new(time: f32, position: Vec3, scale: Vec3, rotation: Quat) -> Self {
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

pub struct Track {
    // Frames are not serialized, because it makes no sense to store them in save file,
    // they will be taken from resource on Resolve stage.
    frames: Vec<KeyFrame>,
    enabled: bool,
    max_time: f32,
    node: Handle<Node>,
}

impl Clone for Track {
    fn clone(&self) -> Self {
        Self {
            frames: self.frames.clone(),
            enabled: self.enabled,
            max_time: self.max_time,
            node: self.node,
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
        }
    }
}

impl Visit for Track {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.enabled.visit("Enabled", visitor)?;
        self.max_time.visit("MaxTime", visitor)?;
        self.node.visit("Node", visitor)?;

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

    pub fn get_key_frame(&self, mut time: f32) -> Option<KeyFrame> {
        if self.frames.is_empty() {
            return None;
        }

        if time >= self.max_time {
            return Some(*self.frames.last().unwrap());
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
            return Some(*self.frames.first().unwrap());
        } else if let Some(left) = self.frames.get(right_index - 1) {
            if let Some(right) = self.frames.get(right_index) {
                let interpolator = (time - left.time) / (right.time - left.time);

                return Some(KeyFrame {
                    time: lerpf(left.time, right.time, interpolator),
                    position: left.position.lerp(&right.position, interpolator),
                    scale: left.scale.lerp(&right.scale, interpolator),
                    rotation: left.rotation.slerp(&right.rotation, interpolator),
                });
            }
        }

        None
    }
}

pub struct Animation {
    tracks: Vec<Track>,
    speed: f32,
    length: f32,
    time_position: f32,
    looped: bool,
    weight: f32,
    fade_step: f32,
    enabled: bool,
    pub(in crate) resource: Option<Arc<Mutex<Model>>>,
}

impl Clone for Animation {
    fn clone(&self) -> Self {
        Self {
            tracks: self.tracks.clone(),
            speed: self.speed,
            length: self.length,
            time_position: self.time_position,
            weight: self.weight,
            fade_step: self.fade_step,
            looped: self.looped,
            enabled: self.enabled,
            resource: self.resource.clone(),
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

    pub fn set_time_position(&mut self, time: f32) {
        if self.looped {
            self.time_position = wrapf(time, 0.0, self.length);
        } else {
            self.time_position = clampf(time, 0.0, self.length);
        }
    }

    pub fn get_time_position(&self) -> f32 {
        self.time_position
    }

    pub fn get_speed(&self) -> f32 {
        self.speed
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn get_tracks_mut(&mut self) -> &mut [Track] {
        &mut self.tracks
    }

    pub fn get_resource(&self) -> Option<Arc<Mutex<Model>>> {
        self.resource.clone()
    }

    pub fn fade_in(&mut self, speed: f32) {
        self.fade_step = speed.abs();
    }

    pub fn fade_out(&mut self, speed: f32) {
        self.fade_step = -speed.abs()
    }

    pub fn get_weight(&self) -> f32 {
        self.weight
    }

    pub fn is_fading(&self) -> bool {
        self.fade_step != 0.0
    }

    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight
    }

    pub fn update_fading(&mut self, dt: f32) {
        if self.is_fading() {
            self.weight += self.fade_step * dt;
            if self.fade_step < 0.0 && self.weight <= 0.0 {
                self.weight = 0.0;
                self.enabled = false;
                self.fade_step = 0.0;
            } else if self.fade_step > 0.0 && self.weight >= 1.0 {
                self.weight = 1.0;
                self.fade_step = 0.0;
            }
        }
    }

    pub(in crate) fn resolve(&mut self, graph: &Graph) {
        // Copy key frames from resource for each animation. This is needed because we
        // do not store key frames in save file, but just keep reference to resource
        // from which key frames should be taken on load.
        if let Some(resource) = self.resource.clone() {
            let resource = resource.lock().unwrap();
            // TODO: Here we assume that resource contains only *one* animation.
            let SceneInterface {
                animations: resource_animations,
                graph: resource_graph, ..
            } = resource.get_scene().interface();
            if let Some(ref_animation) = resource_animations.pool.at(0) {
                for track in self.get_tracks_mut() {
                    let track_node = graph.get(track.get_node());
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
                        if track_node.get_name() == resource_graph.get(ref_track.get_node()).get_name() {
                            track.set_key_frames(ref_track.get_key_frames());
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        println!("Failed to copy key frames for node {}!", track_node.get_name());
                    }
                }
            }
        }
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            speed: 1.0,
            length: 0.0,
            time_position: 0.0,
            weight: 1.0,
            enabled: true,
            fade_step: 0.0,
            looped: true,
            resource: Default::default(),
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
        self.weight.visit("Weight", visitor)?;
        self.fade_step.visit("FadeStep", visitor)?;
        self.resource.visit("Resource", visitor)?;
        self.looped.visit("Looped", visitor)?;
        self.enabled.visit("Enabled", visitor)?;

        visitor.leave_region()
    }
}


pub struct AnimationContainer {
    pool: Pool<Animation>
}

impl Default for AnimationContainer {
    fn default() -> Self {
        Self {
            pool: Pool::new()
        }
    }
}

impl AnimationContainer {
    pub(in crate) fn new() -> Self {
        Self {
            pool: Pool::new()
        }
    }

    #[inline]
    pub fn iter(&self) -> PoolIterator<Animation> {
        self.pool.iter()
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
        self.pool.free(handle)
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

    pub fn resolve(&mut self, graph: &Graph) {
        println!("Resolving animations...");
        for animation in self.pool.iter_mut() {
            animation.resolve(graph)
        }
        println!("Animations resolved successfully!");
    }

    pub fn update_animations(&mut self, dt: f32, graph: &mut Graph) {
        // Reset local transform of animated nodes first
        for animation in self.pool.iter() {
            for track in animation.get_tracks() {
                let node = graph.get_mut(track.get_node());
                let transform = node.get_local_transform_mut();
                transform.set_position(Default::default());
                transform.set_rotation(Default::default());
                // TODO: transform.set_scale(Vec3::make(1.0, 1.0, 1.0));
            }
        }

        // Then apply animation.
        for animation in self.pool.iter_mut() {
            if !animation.is_enabled() {
                continue;
            }

            let next_time_pos = animation.get_time_position() + dt * animation.get_speed();

            let weight = animation.get_weight();

            for track in animation.get_tracks() {
                if !track.is_enabled() {
                    continue;
                }

                if let Some(keyframe) = track.get_key_frame(animation.get_time_position()) {
                    let node = graph.get_mut(track.get_node());
                    let transform = node.get_local_transform_mut();
                    transform.set_rotation(transform.get_rotation().nlerp(&keyframe.rotation, weight));
                    transform.set_position(transform.get_position() + keyframe.position.scale(weight));
                    // TODO: transform.set_scale(transform.get_scale().lerp(&keyframe.scale, weight));
                }
            }

            animation.set_time_position(next_time_pos);
            animation.update_fading(dt);
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