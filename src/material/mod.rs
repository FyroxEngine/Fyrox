use crate::{
    asset::ResourceState,
    core::{
        algebra::{Vector2, Vector3, Vector4},
        color::Color,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    material::shader::{PropertyKind, SamplerFallback, Shader},
    resource::texture::Texture,
};
use std::collections::HashMap;

pub mod shader;

#[derive(Debug, Visit, Clone)]
pub enum PropertyValue {
    Float(f32),
    Int(i32),
    UInt(u32),
    Vector2(Vector2<f32>),
    Vector3(Vector3<f32>),
    Vector4(Vector4<f32>),
    Bool(bool),
    Color(Color),
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
    #[error(
        "Attempt to set a value of wrong type to {} property. Expected: {:?}, given {:?}",
        property_name,
        expected,
        given
    )]
    TypeMismatch {
        property_name: String,
        expected: PropertyValue,
        given: PropertyValue,
    },
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
                let data = texture.state();
                let path = data.path().to_path_buf();
                match &*data {
                    // Try to reload texture even if it failed to load.
                    ResourceState::LoadError { .. } => {
                        drop(data);
                        *texture = resource_manager.request_texture(path, None);
                    }
                    ResourceState::Ok(texture_state) => {
                        // Do not resolve procedural textures.
                        if !texture_state.is_procedural() {
                            drop(data);
                            *texture = resource_manager.request_texture(path, None);
                        }
                    }
                    ResourceState::Pending { .. } => {}
                }
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
            match (value, new_value) {
                (
                    PropertyValue::Sampler {
                        value: old_value,
                        fallback: old_fallback,
                    },
                    PropertyValue::Sampler { value, fallback },
                ) => {
                    *old_value = value;
                    *old_fallback = fallback;
                }
                (PropertyValue::Float(old_value), PropertyValue::Float(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Int(old_value), PropertyValue::Int(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Bool(old_value), PropertyValue::Bool(value)) => {
                    *old_value = value;
                }
                (PropertyValue::UInt(old_value), PropertyValue::UInt(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector2(old_value), PropertyValue::Vector2(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector3(old_value), PropertyValue::Vector3(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector4(old_value), PropertyValue::Vector4(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Color(old_value), PropertyValue::Color(value)) => {
                    *old_value = value;
                }
                (value, new_value) => {
                    return Err(MaterialError::TypeMismatch {
                        property_name: name.as_ref().to_owned(),
                        expected: value.clone(),
                        given: new_value,
                    })
                }
            }

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
