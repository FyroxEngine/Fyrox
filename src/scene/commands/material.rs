use crate::command::Command;
use crate::scene::commands::SceneContext;
use rg3d::material::{Material, PropertyValue};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct SetMaterialPropertyValueCommand {
    material: Arc<Mutex<Material>>,
    name: String,
    value: PropertyValue,
}

impl SetMaterialPropertyValueCommand {
    pub fn new(material: Arc<Mutex<Material>>, name: String, value: PropertyValue) -> Self {
        Self {
            material,
            name,
            value,
        }
    }

    fn swap(&mut self) {
        let mut material = self.material.lock().unwrap();

        let old_value = material.property_ref(&self.name).unwrap().clone();

        material
            .set_property(&self.name, std::mem::replace(&mut self.value, old_value))
            .unwrap();
    }
}

impl<'a> Command<'a> for SetMaterialPropertyValueCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _: &Self::Context) -> String {
        format!("Set Material {} Property Value", self.name)
    }

    fn execute(&mut self, _: &mut Self::Context) {
        self.swap();
    }

    fn revert(&mut self, _: &mut Self::Context) {
        self.swap();
    }
}
