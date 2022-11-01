use fyrox::core::{
    algebra::Vector3,
    inspect::{Inspect, PropertyInfo},
    reflect::prelude::*,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect)]
pub struct ModelSettings {
    #[inspect(
        description = "Initial scale the root of the instance will have after instantiation.\
        Useful when you have lots of huge models and don't want to rescale them manually."
    )]
    pub instantiation_scale: Vector3<f32>,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            instantiation_scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
