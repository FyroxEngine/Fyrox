use crate::{
    math::{
        vec3::Vec3,
        quat::Quat
    },
    utils::rcpool::RcHandle,
    resource::Resource
};

pub struct KeyFrame {
    position: Vec3,
    scale: Vec3,
    rotation: Quat,
    time: f32,
}

pub struct Track {
    frames: Vec<KeyFrame>
}

pub struct Animation {
    tracks: Vec<Track>,
    speed: f32,
    length: f32,
    time_position: f32,
    weight: f32,
    fade_step: f32,
    resource: RcHandle<Resource>
}