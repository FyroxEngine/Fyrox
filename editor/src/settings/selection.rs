use fyrox::core::{inspect::prelude::*, reflect::Reflect};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Inspect, Reflect)]
pub struct SelectionSettings {
    pub ignore_back_faces: bool,
}
