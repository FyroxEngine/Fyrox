use crate::effects::reverb::Reverb;

pub mod reverb;

pub enum Effect {
    Reverb(Reverb)
}

impl Effect {
    pub fn feed(&mut self, left: f32, right: f32) -> (f32, f32) {
        match self {
            Effect::Reverb(reverb) => reverb.feed(left, right),
        }
    }
}