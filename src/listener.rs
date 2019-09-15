#![allow(clippy::or_fun_call)]

use rg3d_core::{
    math::vec3::Vec3,
    visitor::{Visit, VisitResult, Visitor}
};
use crate::error::SoundError;

pub struct Listener {
    pub(in crate) position: Vec3,
    pub(in crate) look_axis: Vec3,
    pub(in crate) up_axis: Vec3,
    pub(in crate) ear_axis: Vec3
}

impl Listener {
    pub(in crate) fn new() -> Self {
        Self {
            position: Vec3::zero(),
            look_axis: Vec3::make(0.0, 0.0, 1.0),
            up_axis: Vec3::make(0.0, 1.0, 0.0),
            ear_axis: Vec3::make(1.0, 0.0, 0.0)
        }
    }

    pub fn set_orientation(&mut self, look: &Vec3, up: &Vec3) -> Result<(), SoundError>{
        self.ear_axis = look.cross(up).normalized().ok_or(SoundError::MathError("|v| == 0.0".to_string()))?;
        self.look_axis = look.normalized().ok_or(SoundError::MathError("|v| == 0.0".to_string()))?;
        self.up_axis = up.normalized().ok_or(SoundError::MathError("|v| == 0.0".to_string()))?;
        Ok(())
    }

    pub fn set_position(&mut self, position: &Vec3) {
        self.position = *position;
    }

    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    pub fn get_up_axis(&self) -> Vec3 {
        self.up_axis
    }

    pub fn get_look_axis(&self) -> Vec3 {
        self.look_axis
    }

    pub fn get_ear_axis(&self) -> Vec3 {
        self.ear_axis
    }
}

impl Visit for Listener {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.look_axis.visit("LookAxis", visitor)?;
        self.up_axis.visit("UpAxis", visitor)?;
        self.ear_axis.visit("EarAxis", visitor)?;

        visitor.leave_region()
    }
}