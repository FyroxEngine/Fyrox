//! Material is a set of parameters for a shader. This module contains everything related to materials.
//!
//! See [Material struct docs](self::Material) for more info.

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

/// A value of a property that will be used for rendering with a shader.
///
/// # Limitations
///
/// There is a limited set of possible types that can be passed to a shader, most of them are
/// just simple data types.
#[derive(Debug, Visit, Clone)]
pub enum PropertyValue {
    /// Real number.
    Float(f32),

    /// Integer number.
    Int(i32),

    /// Unsigned integer number.
    UInt(u32),

    /// Two-dimensional vector.
    Vector2(Vector2<f32>),

    /// Three-dimensional vector.
    Vector3(Vector3<f32>),

    /// Four-dimensional vector.
    Vector4(Vector4<f32>),

    /// Boolean value.
    Bool(bool),

    /// An sRGB color.
    ///
    /// # Conversion
    ///
    /// The colors you see on your monitor are in sRGB color space, this is fine for simple cases
    /// of rendering, but not for complex things like lighting. Such things require color to be
    /// linear. Value of this variant will be automatically **converted to linear color space**
    /// before it passed to shader.  
    Color(Color),

    /// A texture with fallback option.
    ///
    /// # Fallback
    ///
    /// Sometimes you don't want to set a value to a sampler, or you even don't have the appropriate
    /// one. There is fallback value that helps you with such situations, it defines a values that
    /// will be fetched from a sampler when there is no texture.
    ///
    /// For example, standard shader has a lot of samplers defined: diffuse, normal, height, emission,
    /// mask, metallic, roughness, etc. In some situations you may not have all the textures, you have
    /// only diffuse texture, to keep rendering correct, each other property has appropriate fallback
    /// value. Normal sampler - a normal vector pointing up (+Y), height - zero, emission - zero, etc.
    ///
    /// Fallback value is also helpful to catch missing textures, you'll definitely know the texture is
    /// missing by very specific value in the fallback texture.
    Sampler {
        value: Option<Texture>,
        fallback: SamplerFallback,
    },
}

macro_rules! define_as {
    ($name:ident = $variant:ident -> $ty:ty) => {
        pub fn $name(&self) -> Option<$ty> {
            if let PropertyValue::$variant(v) = self {
                Some(*v)
            } else {
                None
            }
        }
    };
}

impl PropertyValue {
    define_as!(as_float = Float -> f32);
    define_as!(as_int = Int -> i32);
    define_as!(as_uint = UInt -> u32);
    define_as!(as_bool = Bool -> bool);
    define_as!(as_color = Color -> Color);
    define_as!(as_vector2 = Vector2 -> Vector2<f32>);
    define_as!(as_vector3 = Vector3 -> Vector3<f32>);
    define_as!(as_vector4 = Vector4 -> Vector4<f32>);

    pub fn as_sampler(&self) -> Option<Texture> {
        if let PropertyValue::Sampler { value, .. } = self {
            value.clone()
        } else {
            None
        }
    }
}

impl Default for PropertyValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Material defines a set of values for a shader. Materials usually contains textures (diffuse,
/// normal, height, emission, etc. maps), numerical values (floats, integers), vectors, booleans,
/// matrices and arrays of each type, except textures. Each parameter can be changed in runtime
/// giving you the ability to create animated materials. However in practice, most materials are
/// static, this means that once it created, it won't be changed anymore.
///
/// Please keep in mind that the actual "rules" of drawing an entity are stored in the shader,
/// **material is only a storage** for specific uses of the shader.
///
/// Multiple materials can share the same shader, for example standard shader covers 95% of most
/// common use cases and it is shared across multiple materials. The only difference are property
/// values, for example you can draw multiple cubes using the same shader, but with different
/// textures.
///
/// Material itself can be shared across multiple places as well as the shader. This gives you the
/// ability to render multiple objects with the same material efficiently.
///
/// # Performance
///
/// It is very important re-use materials as much as possible, because the amount of materials used
/// per frame significantly correlates with performance. The more unique materials you have per frame,
/// the more work has to be done by the renderer and video driver to render a frame and the more time
/// the frame will require for rendering, thus lowering your FPS.
///
/// # Examples
///
/// A material can only be created using a shader instance, every material must have a shader. The
/// shader provides information about its properties, and this information is used to populate a set
/// of properties with default values. Default values of each property defined in the shader.
///
/// ## Standard material
///
/// Usually standard shader is enough for most cases, `Material` even has a `.standard()` method to
/// create a material with standard shader:
///
/// ```no_run
/// use rg3d::{
///     material::shader::{Shader, SamplerFallback},
///     engine::resource_manager::ResourceManager,
///     material::{Material, PropertyValue}
/// };
///
/// fn create_brick_material(resource_manager: ResourceManager) -> Material {
///     let mut material = Material::standard();
///
///     material.set_property(
///         "diffuseTexture",
///         PropertyValue::Sampler {
///             value: Some(resource_manager.request_texture("Brick_DiffuseTexture.jpg", None)),
///             fallback: SamplerFallback::White
///         })
///         .unwrap();
///     
///     material
/// }
/// ```
///
/// As you can see it is pretty simple with standard material, all you need is to set values to desired
/// properties and you good to go. All you need to do is to apply the material, for example it could be
/// mesh surface or some other place that supports materials. For the full list of properties of the
/// standard shader see [shader module docs](crate::material::shader).
///
/// ## Custom material
///
/// Custom materials is a bit more complex, you need to get a shader instance using the resource manager
/// and then create the material and populate it with a set of property values.
///
/// ```no_run
/// use rg3d::{
///     engine::resource_manager::ResourceManager,
///     material::{Material, PropertyValue},
///     core::algebra::Vector3
/// };
///
/// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
///     let shader = resource_manager.request_shader("my_grass_shader.ron").await.unwrap();
///     
///     // Here we assume that the material really has the properties defined below.
///     let mut material = Material::from_shader(shader, Some(resource_manager));
///
///     material.set_property(
///         "windDirection",
///         PropertyValue::Vector3(Vector3::new(1.0, 0.0, 0.5))
///         )
///         .unwrap();
///
///     material
/// }
/// ```
///
/// As you can see it is only a bit more hard that with the standard shader. The main difference here is
/// that we using resource manager to get shader instance and the we just use the instance to create
/// material instance. Then we populate properties as usual.
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
