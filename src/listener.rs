use rg3d_core::{
    math::{
        vec3::Vec3,
        mat4::Mat4
    },
    visitor::{Visit, VisitResult, Visitor},
};
use crate::error::SoundError;

pub struct Listener {
    pub(in crate) position: Vec3,
    pub(in crate) look_axis: Vec3,
    pub(in crate) up_axis: Vec3,
    pub(in crate) ear_axis: Vec3,
    pub(in crate) view_matrix: Mat4,
}

impl Listener {
    pub(in crate) fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            look_axis: Vec3::new(0.0, 0.0, 1.0),
            up_axis: Vec3::new(0.0, 1.0, 0.0),
            ear_axis: Vec3::new(1.0, 0.0, 0.0),
            view_matrix: Default::default(),
        }
    }

    pub fn set_orientation(&mut self, look: &Vec3, up: &Vec3) -> Result<(), SoundError> {
        self.ear_axis = up.cross(look).normalized().ok_or_else(|| SoundError::MathError("|v| == 0.0".to_string()))?;
        self.look_axis = look.normalized().ok_or_else(|| SoundError::MathError("|v| == 0.0".to_string()))?;
        self.up_axis = up.normalized().ok_or_else(|| SoundError::MathError("|v| == 0.0".to_string()))?;
        Ok(())
    }

    pub fn update(&mut self) {
        self.view_matrix = Mat4 {
            f: [
                self.ear_axis.x, self.up_axis.x, -self.look_axis.x, 0.0,
                self.ear_axis.y, self.up_axis.y, -self.look_axis.y, 0.0,
                self.ear_axis.z, self.up_axis.z, -self.look_axis.z, 0.0,
                -self.ear_axis.dot(&self.position), -self.up_axis.dot(&self.position), -self.look_axis.dot(&self.position), 1.0,
            ]};
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