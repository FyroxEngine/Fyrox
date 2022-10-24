use crate::{
    animation::value::{TrackValue, Value},
    core::algebra::{UnitQuaternion, Vector3},
    core::visitor::prelude::*,
};

#[derive(Default, Visit, Debug, Clone)]
pub struct Frame<T: Value> {
    value: T,
    time: f32,
}

#[derive(Default, Visit, Debug, Clone)]
pub struct GenericTrackFramesContainer<T: Value> {
    frames: Vec<Frame<T>>,
    max_time: f32,
}

impl<T: Value> GenericTrackFramesContainer<T> {
    pub fn add_key_frame(&mut self, frame: Frame<T>) {
        if frame.time > self.max_time {
            self.max_time = frame.time;
            self.frames.push(frame);
        } else {
            // Find a place to insert
            let mut index = 0;
            for (i, other_frame) in self.frames.iter().enumerate() {
                if frame.time < other_frame.time {
                    index = i;
                    break;
                }
            }
            self.frames.insert(index, frame)
        }
    }

    pub fn set_frames(&mut self, frames: Vec<Frame<T>>) {
        self.frames = frames;
        self.max_time = 0.0;

        for frame in self.frames.iter() {
            if frame.time > self.max_time {
                self.max_time = frame.time;
            }
        }
    }

    pub fn fetch(&self, mut time: f32) -> Option<T> {
        if self.frames.is_empty() {
            return None;
        }

        if time >= self.max_time {
            return self.frames.last().map(|k| k.value.clone());
        }

        time = time.clamp(0.0, self.max_time);

        let mut right_index = 0;
        for (i, keyframe) in self.frames.iter().enumerate() {
            if keyframe.time >= time {
                right_index = i;
                break;
            }
        }

        if right_index == 0 {
            self.frames.first().map(|k| k.value.clone())
        } else {
            let left = &self.frames[right_index - 1];
            let right = &self.frames[right_index];
            let interpolator = (time - left.time) / (right.time - left.time);
            Some(left.value.interpolate(&right.value, interpolator))
        }
    }
}

#[derive(Visit, Debug, Clone)]
pub enum TrackFramesContainer {
    Vector3(GenericTrackFramesContainer<Vector3<f32>>),
    UnitQuaternion(GenericTrackFramesContainer<UnitQuaternion<f32>>),
}

impl TrackFramesContainer {
    pub fn add(&mut self, time: f32, value: TrackValue) {
        match (self, value) {
            (Self::Vector3(container), TrackValue::Vector3(value)) => {
                container.add_key_frame(Frame { time, value })
            }
            (Self::UnitQuaternion(container), TrackValue::UnitQuaternion(value)) => {
                container.add_key_frame(Frame { time, value })
            }
            _ => (),
        }
    }

    pub fn fetch(&self, time: f32) -> Option<TrackValue> {
        match self {
            TrackFramesContainer::Vector3(vec3) => vec3.fetch(time).map(TrackValue::Vector3),
            TrackFramesContainer::UnitQuaternion(quat) => {
                quat.fetch(time).map(TrackValue::UnitQuaternion)
            }
        }
    }

    pub fn time_length(&self) -> f32 {
        match self {
            TrackFramesContainer::Vector3(v) => v.max_time,
            TrackFramesContainer::UnitQuaternion(v) => v.max_time,
        }
    }
}
