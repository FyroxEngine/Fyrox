use crate::{
    math::{
        vec3::Vec3,
        quat::Quat,
    },
    resource::Resource,
    utils::visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
    scene::node::Node,
    utils::pool::Handle,
    math::clampf,
};
use std::{
    rc::Rc,
    cell::RefCell,
};
use crate::math::{lerpf, wrapf};

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
            rotation
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
            node: self.node.clone(),
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

        self.frames.visit("Frames", visitor)?;
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
        self.node.clone()
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
        } else {
            if let Some(left) = self.frames.get(right_index - 1) {
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
    resource: Option<Rc<RefCell<Resource>>>,
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
            resource: self.resource.clone()
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
    
    pub fn update_fading(&mut self, dt: f32) {
        if self.fade_step != 0.0 {
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
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            speed: 1.0,
            length: 0.0,
            time_position: 0.0,
            weight: 0.0,
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