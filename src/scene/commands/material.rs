use crate::{command::Command, scene::commands::SceneContext};
use rg3d::material::{shader::Shader, Material, PropertyValue};
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

#[derive(Debug)]
enum SetMaterialShaderCommandState {
    Undefined,
    NonExecuted { new_shader: Shader },
    Executed { old_material: Material },
    Reverted { new_material: Material },
}

#[derive(Debug)]
pub struct SetMaterialShaderCommand {
    material: Arc<Mutex<Material>>,
    state: SetMaterialShaderCommandState,
}

impl SetMaterialShaderCommand {
    pub fn new(material: Arc<Mutex<Material>>, shader: Shader) -> Self {
        Self {
            material,
            state: SetMaterialShaderCommandState::NonExecuted { new_shader: shader },
        }
    }

    fn swap(&mut self, context: &mut SceneContext) {
        match std::mem::replace(&mut self.state, SetMaterialShaderCommandState::Undefined) {
            SetMaterialShaderCommandState::Undefined => {
                unreachable!()
            }
            SetMaterialShaderCommandState::NonExecuted { new_shader } => {
                let mut material = self.material.lock().unwrap();

                let old_material = std::mem::replace(
                    &mut *material,
                    Material::from_shader(new_shader, Some(context.resource_manager.clone())),
                );

                self.state = SetMaterialShaderCommandState::Executed { old_material };
            }
            SetMaterialShaderCommandState::Executed { old_material } => {
                let mut material = self.material.lock().unwrap();

                let new_material = std::mem::replace(&mut *material, old_material);

                self.state = SetMaterialShaderCommandState::Reverted { new_material };
            }
            SetMaterialShaderCommandState::Reverted { new_material } => {
                let mut material = self.material.lock().unwrap();

                let old_material = std::mem::replace(&mut *material, new_material);

                self.state = SetMaterialShaderCommandState::Executed { old_material };
            }
        }
    }
}

impl<'a> Command<'a> for SetMaterialShaderCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _: &Self::Context) -> String {
        "Set Material Shader".to_owned()
    }

    fn execute(&mut self, ctx: &mut Self::Context) {
        self.swap(ctx);
    }

    fn revert(&mut self, ctx: &mut Self::Context) {
        self.swap(ctx);
    }
}
