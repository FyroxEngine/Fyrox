use crate::material::shader::SamplerFallback;
use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        color::Color,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    material::shader::{PropertyKind, Shader},
    resource::texture::Texture,
};
use std::collections::HashMap;

pub mod shader;

#[derive(Debug, Visit)]
pub enum PropertyValue {
    Float(f32),
    Int(i32),
    UInt(u32),
    Vector2(Vector2<f32>),
    Vector3(Vector3<f32>),
    Vector4(Vector4<f32>),
    Color(Color),
    Bool(bool),
    Sampler {
        value: Option<Texture>,
        fallback: SamplerFallback,
    },
}

impl Default for PropertyValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

#[derive(Default, Debug, Visit)]
pub struct Material {
    shader: Shader,
    properties: HashMap<String, PropertyValue>,
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum MaterialError {
    #[error("Unable to find material property {}", property_name)]
    NoSuchProperty { property_name: String },
}

impl Material {
    pub fn standard() -> Self {
        Self::from_shader(shader::STANDARD.clone(), None)
    }

    pub fn from_shader(shader: Shader, resource_manager: Option<ResourceManager>) -> Self {
        let data = shader.data_ref();

        let mut property_values = HashMap::new();
        for property_definition in data.definition.properties.iter() {
            let value = match &property_definition.kind {
                PropertyKind::Float(value) => PropertyValue::Float(*value),
                PropertyKind::Int(value) => PropertyValue::Int(*value),
                PropertyKind::UInt(value) => PropertyValue::UInt(*value),
                PropertyKind::Vector2 { x, y } => PropertyValue::Vector2(Vector2::new(*x, *y)),
                PropertyKind::Vector3 { x, y, z } => {
                    PropertyValue::Vector3(Vector3::new(*x, *y, *z))
                }
                PropertyKind::Vector4 { x, y, z, w } => {
                    PropertyValue::Vector4(Vector4::new(*x, *y, *z, *w))
                }
                PropertyKind::Color { r, g, b, a } => {
                    PropertyValue::Color(Color::from_rgba(*r, *g, *b, *a))
                }
                PropertyKind::Bool(value) => PropertyValue::Bool(*value),
                PropertyKind::Sampler {
                    default,
                    fallback: usage,
                } => PropertyValue::Sampler {
                    value: default.as_ref().and_then(|path| {
                        resource_manager
                            .clone()
                            .map(|rm| rm.request_texture(path, None))
                    }),
                    fallback: *usage,
                },
            };

            property_values.insert(property_definition.name.clone(), value);
        }

        drop(data);

        Self {
            shader: shader::STANDARD.clone(),
            properties: property_values,
        }
    }

    pub fn resolve(&mut self, resource_manager: ResourceManager) {
        for value in self.properties.values_mut() {
            if let PropertyValue::Sampler {
                value: Some(texture),
                ..
            } = value
            {
                let shallow_texture = texture.state();
                let path = shallow_texture.path().to_path_buf();
                drop(shallow_texture);
                *texture = resource_manager.request_texture(path, None);
            }
        }
    }

    pub fn property_ref<N: AsRef<str>>(&self, name: N) -> Option<&PropertyValue> {
        self.properties.get(name.as_ref())
    }

    pub fn set_property<N: AsRef<str>>(
        &mut self,
        name: N,
        new_value: PropertyValue,
    ) -> Result<(), MaterialError> {
        if let Some(value) = self.properties.get_mut(name.as_ref()) {
            *value = new_value;
            Ok(())
        } else {
            Err(MaterialError::NoSuchProperty {
                property_name: name.as_ref().to_owned(),
            })
        }
    }

    pub fn shader(&self) -> &Shader {
        &self.shader
    }

    pub fn properties(&self) -> &HashMap<String, PropertyValue> {
        &self.properties
    }
}
