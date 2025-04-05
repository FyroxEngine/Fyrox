// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    command::{CommandContext, CommandTrait},
    fyrox::{
        asset::ResourceData,
        core::{log::Log, sstorage::ImmutableString},
        material::{
            shader::ShaderResource, Material, MaterialProperty, MaterialResource,
            MaterialResourceBinding,
        },
    },
};
use std::path::{Path, PathBuf};

fn try_save(path: Option<&Path>, material: &MaterialResource) {
    if let Some(path) = path {
        Log::verify(material.data_ref().save(path));
    } else {
        Log::warn("The edited material cannot be saved, because it does not have a path!")
    }
}

#[derive(Debug)]
pub struct SetMaterialBindingCommand {
    material: MaterialResource,
    name: ImmutableString,
    binding: Option<MaterialResourceBinding>,
    path: Option<PathBuf>,
}

impl SetMaterialBindingCommand {
    pub fn new(
        material: MaterialResource,
        name: ImmutableString,
        binding: MaterialResourceBinding,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            material,
            name,
            binding: Some(binding),
            path,
        }
    }

    fn swap(&mut self) {
        let mut material = self.material.data_ref();

        let old_value = material.binding_ref(self.name.clone()).cloned();
        let new_value = std::mem::replace(&mut self.binding, old_value);
        if let Some(new_value) = new_value {
            material.bind(self.name.clone(), new_value);
        } else {
            material.unbind(self.name.clone());
        }

        drop(material);
        try_save(self.path.as_deref(), &self.material);
    }
}

impl CommandTrait for SetMaterialBindingCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Set Material {} Property Value", self.name)
    }

    fn execute(&mut self, _: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
pub struct SetMaterialPropertyGroupPropertyValueCommand {
    material: MaterialResource,
    group_name: ImmutableString,
    property_name: ImmutableString,
    value: Option<MaterialProperty>,
    path: Option<PathBuf>,
}

impl SetMaterialPropertyGroupPropertyValueCommand {
    pub fn new(
        material: MaterialResource,
        group_name: ImmutableString,
        property_name: ImmutableString,
        value: MaterialProperty,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            material,
            group_name,
            property_name,
            value: Some(value),
            path,
        }
    }

    fn swap(&mut self) {
        let mut material = self.material.data_ref();

        let group = material.try_get_or_insert_property_group(self.group_name.clone());
        let old_value = group.property_ref(self.property_name.clone()).cloned();
        let new_value = std::mem::replace(&mut self.value, old_value);
        if let Some(new_value) = new_value {
            group.set_property(self.property_name.clone(), new_value);
        } else {
            group.unset_property(self.property_name.clone());
        }

        drop(material);
        try_save(self.path.as_deref(), &self.material);
    }
}

impl CommandTrait for SetMaterialPropertyGroupPropertyValueCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        format!("Set Material {} Property Value", self.property_name)
    }

    fn execute(&mut self, _: &mut dyn CommandContext) {
        self.swap();
    }

    fn revert(&mut self, _: &mut dyn CommandContext) {
        self.swap();
    }
}

#[derive(Debug)]
enum SetMaterialShaderCommandState {
    Undefined,
    NonExecuted { new_shader: ShaderResource },
    Executed { old_material: Material },
    Reverted { new_material: Material },
}

#[derive(Debug)]
pub struct SetMaterialShaderCommand {
    material: MaterialResource,
    path: Option<PathBuf>,
    state: SetMaterialShaderCommandState,
}

impl SetMaterialShaderCommand {
    pub fn new(material: MaterialResource, shader: ShaderResource, path: Option<PathBuf>) -> Self {
        Self {
            material,
            state: SetMaterialShaderCommandState::NonExecuted { new_shader: shader },
            path,
        }
    }

    fn swap(&mut self, _context: &mut dyn CommandContext) {
        match std::mem::replace(&mut self.state, SetMaterialShaderCommandState::Undefined) {
            SetMaterialShaderCommandState::Undefined => {
                unreachable!()
            }
            SetMaterialShaderCommandState::NonExecuted { new_shader } => {
                let mut material = self.material.data_ref();

                let old_material =
                    std::mem::replace(&mut *material, Material::from_shader(new_shader));

                self.state = SetMaterialShaderCommandState::Executed { old_material };
            }
            SetMaterialShaderCommandState::Executed { old_material } => {
                let mut material = self.material.data_ref();

                let new_material = std::mem::replace(&mut *material, old_material);

                self.state = SetMaterialShaderCommandState::Reverted { new_material };
            }
            SetMaterialShaderCommandState::Reverted { new_material } => {
                let mut material = self.material.data_ref();

                let old_material = std::mem::replace(&mut *material, new_material);

                self.state = SetMaterialShaderCommandState::Executed { old_material };
            }
        }

        try_save(self.path.as_deref(), &self.material);
    }
}

impl CommandTrait for SetMaterialShaderCommand {
    fn name(&mut self, _: &dyn CommandContext) -> String {
        "Set Material Shader".to_owned()
    }

    fn execute(&mut self, ctx: &mut dyn CommandContext) {
        self.swap(ctx);
    }

    fn revert(&mut self, ctx: &mut dyn CommandContext) {
        self.swap(ctx);
    }
}
