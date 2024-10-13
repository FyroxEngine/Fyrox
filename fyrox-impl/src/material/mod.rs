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

//! Material is a set of parameters for a shader. This module contains everything related to materials.
//!
//! See [Material struct docs](self::Material) for more info.

// #![warn(missing_docs)] TODO: docs needs rework

use crate::{
    asset::{io::ResourceIo, manager::ResourceManager, Resource, ResourceData},
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        io::FileLoadError,
        log::Log,
        parking_lot::Mutex,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        visitor::{prelude::*, RegionGuard},
        TypeUuidProvider,
    },
    material::shader::{
        SamplerFallback, ShaderResource, ShaderResourceExtension, ShaderResourceKind,
    },
    resource::texture::{Texture, TextureResource},
};
use fxhash::FxHashMap;
use fyrox_graphics::gpu_program::ShaderPropertyKind;
use fyrox_resource::manager::BuiltInResource;
use fyrox_resource::state::ResourceState;
use fyrox_resource::untyped::ResourceKind;
use lazy_static::lazy_static;
use std::error::Error;
use std::{
    any::Any,
    fmt::{Display, Formatter},
    ops::Deref,
    path::Path,
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod loader;
pub mod shader;

/// A value of a property that will be used for rendering with a shader.
///
/// # Limitations
///
/// There is a limited set of possible types that can be passed to a shader, most of them are
/// just simple data types.
#[derive(Debug, Visit, Clone, Reflect, TypeUuidProvider, AsRefStr, EnumString, VariantNames)]
#[type_uuid(id = "2df8f1e5-0075-4d0d-9860-70fc27d3e165")]
pub enum MaterialResourceBindingValue {
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
        /// Actual value of the sampler. Could be [`None`], in this case `fallback` will be used.
        value: Option<TextureResource>,

        /// Sampler fallback value.
        fallback: SamplerFallback,
    },

    PropertyGroup(MaterialPropertyGroup),
}

impl Default for MaterialResourceBindingValue {
    fn default() -> Self {
        Self::PropertyGroup(Default::default())
    }
}

impl MaterialResourceBindingValue {
    /// Creates a resource binding from its shader's representation.
    pub fn from_property_kind(
        kind: &ShaderResourceKind,
        resource_manager: Option<&ResourceManager>,
    ) -> Self {
        match kind {
            ShaderResourceKind::Sampler {
                default,
                fallback: usage,
                ..
            } => Self::Sampler {
                value: default
                    .as_ref()
                    .and_then(|path| resource_manager.map(|rm| rm.request::<Texture>(path))),
                fallback: *usage,
            },
            ShaderResourceKind::PropertyGroup(group) => {
                Self::PropertyGroup(MaterialPropertyGroup {
                    properties: group
                        .iter()
                        .map(|property| MaterialProperty {
                            name: property.name.as_str().into(),
                            value: MaterialPropertyValue::from_property_kind(&property.kind),
                        })
                        .collect::<Vec<_>>(),
                })
            }
        }
    }

    /// Tries to extract a texture from the resource binding.
    pub fn as_sampler(&self) -> Option<TextureResource> {
        if let Self::Sampler { value, .. } = self {
            value.clone()
        } else {
            None
        }
    }
}

#[derive(Default, Debug, Visit, Clone, Reflect, TypeUuidProvider)]
#[type_uuid(id = "4d8bd5b1-8996-4681-bd34-fd3f46f8179f")]
pub struct MaterialResourceBinding {
    pub name: ImmutableString,
    pub value: MaterialResourceBindingValue,
}

#[derive(Default, Debug, Visit, Clone, Reflect, TypeUuidProvider)]
#[type_uuid(id = "29af996e-a2d3-4d72-adee-5db16e46f379")]
pub struct MaterialProperty {
    pub name: ImmutableString,
    pub value: MaterialPropertyValue,
}

#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct MaterialPropertyGroup {
    properties: Vec<MaterialProperty>,
}

impl MaterialPropertyGroup {
    /// Searches for a property with given name.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    /// # use fyrox_impl::material::Material;
    ///
    /// let mut material = Material::standard();
    ///
    /// let color = material.property_ref(&ImmutableString::new("diffuseColor")).unwrap().as_color();
    /// ```
    pub fn property_ref(&self, name: impl Into<ImmutableString>) -> Option<&MaterialPropertyValue> {
        let name = name.into();
        self.properties.iter().find_map(|property| {
            if property.name == name {
                Some(&property.value)
            } else {
                None
            }
        })
    }

    fn property_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialPropertyValue> {
        let name = name.into();
        self.properties.iter_mut().find_map(|property| {
            if property.name == name {
                Some(&mut property.value)
            } else {
                None
            }
        })
    }

    /// Sets new value of the property with given name.
    ///
    /// # Type checking
    ///
    /// A new value must have the same type as in shader, otherwise an error will be generated.
    /// This helps to catch subtle bugs when you passing "almost" identical values to shader, like
    /// signed and unsigned integers - both have positive values, but GPU is very strict of what
    /// it expects as input value.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::material::{Material, MaterialPropertyValue};
    /// # use fyrox_impl::core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// assert!(material.set_property("diffuseColor", Color::WHITE).is_ok());
    /// ```
    pub fn set_property(
        &mut self,
        name: impl Into<ImmutableString>,
        new_value: impl Into<MaterialPropertyValue>,
    ) -> Result<(), MaterialError> {
        let name = name.into();
        let new_value = new_value.into();
        if let Some(value) = self.property_mut(name.clone()) {
            match (value, new_value) {
                (MaterialPropertyValue::Float(old_value), MaterialPropertyValue::Float(value)) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::FloatArray(old_value),
                    MaterialPropertyValue::FloatArray(value),
                ) => {
                    *old_value = value;
                }
                (MaterialPropertyValue::Int(old_value), MaterialPropertyValue::Int(value)) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::IntArray(old_value),
                    MaterialPropertyValue::IntArray(value),
                ) => {
                    *old_value = value;
                }
                (MaterialPropertyValue::Bool(old_value), MaterialPropertyValue::Bool(value)) => {
                    *old_value = value;
                }
                (MaterialPropertyValue::UInt(old_value), MaterialPropertyValue::UInt(value)) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::UIntArray(old_value),
                    MaterialPropertyValue::UIntArray(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Vector2(old_value),
                    MaterialPropertyValue::Vector2(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Vector2Array(old_value),
                    MaterialPropertyValue::Vector2Array(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Vector3(old_value),
                    MaterialPropertyValue::Vector3(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Vector3Array(old_value),
                    MaterialPropertyValue::Vector3Array(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Vector4(old_value),
                    MaterialPropertyValue::Vector4(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Vector4Array(old_value),
                    MaterialPropertyValue::Vector4Array(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Matrix2(old_value),
                    MaterialPropertyValue::Matrix2(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Matrix2Array(old_value),
                    MaterialPropertyValue::Matrix2Array(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Matrix3(old_value),
                    MaterialPropertyValue::Matrix3(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Matrix3Array(old_value),
                    MaterialPropertyValue::Matrix3Array(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Matrix4(old_value),
                    MaterialPropertyValue::Matrix4(value),
                ) => {
                    *old_value = value;
                }
                (
                    MaterialPropertyValue::Matrix4Array(old_value),
                    MaterialPropertyValue::Matrix4Array(value),
                ) => {
                    *old_value = value;
                }
                (MaterialPropertyValue::Color(old_value), MaterialPropertyValue::Color(value)) => {
                    *old_value = value;
                }
                (value, new_value) => {
                    return Err(MaterialError::PropertyTypeMismatch {
                        property_name: name.deref().to_owned(),
                        expected: Box::new(value.clone()),
                        given: Box::new(new_value),
                    })
                }
            }

            Ok(())
        } else {
            Err(MaterialError::NoSuchProperty {
                property_name: name.deref().to_owned(),
            })
        }
    }

    /// Returns immutable reference to internal property storage.
    pub fn properties(&self) -> &[MaterialProperty] {
        &self.properties
    }
}

#[derive(Debug, Visit, Clone, Reflect, AsRefStr, EnumString, VariantNames, TypeUuidProvider)]
#[type_uuid(id = "1c25018d-ab6e-4dca-99a6-e3d9639bc33c")]
pub enum MaterialPropertyValue {
    /// Real number.
    Float(f32),

    /// Real number array.
    FloatArray(Vec<f32>),

    /// Integer number.
    Int(i32),

    /// Integer number array.
    IntArray(Vec<i32>),

    /// Natural number.
    UInt(u32),

    /// Natural number array.
    UIntArray(Vec<u32>),

    /// Two-dimensional vector.
    Vector2(Vector2<f32>),

    /// Two-dimensional vector array.
    Vector2Array(Vec<Vector2<f32>>),

    /// Three-dimensional vector.
    Vector3(Vector3<f32>),

    /// Three-dimensional vector array.
    Vector3Array(Vec<Vector3<f32>>),

    /// Four-dimensional vector.
    Vector4(Vector4<f32>),

    /// Four-dimensional vector array.
    Vector4Array(Vec<Vector4<f32>>),

    /// 2x2 Matrix.
    Matrix2(Matrix2<f32>),

    /// 2x2 Matrix array.
    Matrix2Array(Vec<Matrix2<f32>>),

    /// 3x3 Matrix.
    Matrix3(Matrix3<f32>),

    /// 3x3 Matrix array.
    Matrix3Array(Vec<Matrix3<f32>>),

    /// 4x4 Matrix.
    Matrix4(Matrix4<f32>),

    /// 4x4 Matrix array.
    Matrix4Array(Vec<Matrix4<f32>>),

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
}

macro_rules! impl_from {
    ($variant:ident => $value_type:ty) => {
        impl From<$value_type> for MaterialPropertyValue {
            fn from(value: $value_type) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_from!(Float => f32);
impl_from!(FloatArray => Vec<f32>);
impl_from!(Int => i32);
impl_from!(IntArray => Vec<i32>);
impl_from!(UInt => u32);
impl_from!(UIntArray => Vec<u32>);
impl_from!(Vector2 => Vector2<f32>);
impl_from!(Vector2Array => Vec<Vector2<f32>>);
impl_from!(Vector3 => Vector3<f32>);
impl_from!(Vector3Array => Vec<Vector3<f32>>);
impl_from!(Vector4 => Vector4<f32>);
impl_from!(Vector4Array => Vec<Vector4<f32>>);
impl_from!(Matrix2 => Matrix2<f32>);
impl_from!(Matrix2Array => Vec<Matrix2<f32>>);
impl_from!(Matrix3 => Matrix3<f32>);
impl_from!(Matrix3Array => Vec<Matrix3<f32>>);
impl_from!(Matrix4 => Matrix4<f32>);
impl_from!(Matrix4Array => Vec<Matrix4<f32>>);
impl_from!(Bool => bool);
impl_from!(Color => Color);

impl From<Option<TextureResource>> for MaterialResourceBindingValue {
    fn from(value: Option<TextureResource>) -> Self {
        Self::Sampler {
            value,
            fallback: Default::default(),
        }
    }
}

impl From<TextureResource> for MaterialResourceBindingValue {
    fn from(value: TextureResource) -> Self {
        Self::Sampler {
            value: Some(value),
            fallback: Default::default(),
        }
    }
}

macro_rules! define_as {
    ($(#[$meta:meta])* $name:ident = $variant:ident -> $ty:ty) => {
        $(#[$meta])*
        pub fn $name(&self) -> Option<$ty> {
            if let MaterialPropertyValue::$variant(v) = self {
                Some(*v)
            } else {
                None
            }
        }
    };
}

macro_rules! define_as_ref {
    ($(#[$meta:meta])* $name:ident = $variant:ident -> $ty:ty) => {
        $(#[$meta])*
        pub fn $name(&self) -> Option<&$ty> {
            if let MaterialPropertyValue::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }
    };
}

impl MaterialPropertyValue {
    /// Creates property value from its shader's representation.
    pub fn from_property_kind(kind: &ShaderPropertyKind) -> Self {
        match kind {
            ShaderPropertyKind::Float(value) => MaterialPropertyValue::Float(*value),
            ShaderPropertyKind::Int(value) => MaterialPropertyValue::Int(*value),
            ShaderPropertyKind::UInt(value) => MaterialPropertyValue::UInt(*value),
            ShaderPropertyKind::Vector2(value) => MaterialPropertyValue::Vector2(*value),
            ShaderPropertyKind::Vector3(value) => MaterialPropertyValue::Vector3(*value),
            ShaderPropertyKind::Vector4(value) => MaterialPropertyValue::Vector4(*value),
            ShaderPropertyKind::Color { r, g, b, a } => {
                MaterialPropertyValue::Color(Color::from_rgba(*r, *g, *b, *a))
            }
            ShaderPropertyKind::Matrix2(value) => MaterialPropertyValue::Matrix2(*value),
            ShaderPropertyKind::Matrix3(value) => MaterialPropertyValue::Matrix3(*value),
            ShaderPropertyKind::Matrix4(value) => MaterialPropertyValue::Matrix4(*value),
            ShaderPropertyKind::Bool(value) => MaterialPropertyValue::Bool(*value),

            ShaderPropertyKind::FloatArray { value, .. } => {
                MaterialPropertyValue::FloatArray(value.clone())
            }
            ShaderPropertyKind::IntArray { value, .. } => {
                MaterialPropertyValue::IntArray(value.clone())
            }
            ShaderPropertyKind::UIntArray { value, .. } => {
                MaterialPropertyValue::UIntArray(value.clone())
            }
            ShaderPropertyKind::Vector2Array { value, .. } => {
                MaterialPropertyValue::Vector2Array(value.clone())
            }
            ShaderPropertyKind::Vector3Array { value, .. } => {
                MaterialPropertyValue::Vector3Array(value.clone())
            }
            ShaderPropertyKind::Vector4Array { value, .. } => {
                MaterialPropertyValue::Vector4Array(value.clone())
            }
            ShaderPropertyKind::Matrix2Array { value, .. } => {
                MaterialPropertyValue::Matrix2Array(value.clone())
            }
            ShaderPropertyKind::Matrix3Array { value, .. } => {
                MaterialPropertyValue::Matrix3Array(value.clone())
            }
            ShaderPropertyKind::Matrix4Array { value, .. } => {
                MaterialPropertyValue::Matrix4Array(value.clone())
            }
        }
    }

    define_as!(
        /// Tries to unwrap property value as float.
        as_float = Float -> f32
    );
    define_as_ref!(
        /// Tries to unwrap property value as float array.
        as_float_array = FloatArray -> [f32]
    );
    define_as!(
        /// Tries to unwrap property value as integer.
        as_int = Int -> i32
    );
    define_as_ref!(
        /// Tries to unwrap property value as integer array.
        as_int_array = IntArray -> [i32]
    );
    define_as!(
        /// Tries to unwrap property value as unsigned integer.
        as_uint = UInt -> u32
    );
    define_as_ref!(
        /// Tries to unwrap property value as unsigned integer array.
        as_uint_array = UIntArray -> [u32]
    );
    define_as!(
        /// Tries to unwrap property value as boolean.
        as_bool = Bool -> bool
    );
    define_as!(
        /// Tries to unwrap property value as color.
        as_color = Color -> Color
    );
    define_as!(
        /// Tries to unwrap property value as two-dimensional vector.
        as_vector2 = Vector2 -> Vector2<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as two-dimensional vector array.
        as_vector2_array = Vector2Array -> [Vector2<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as three-dimensional vector.
        as_vector3 = Vector3 -> Vector3<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as three-dimensional vector array.
        as_vector3_array = Vector3Array -> [Vector3<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as four-dimensional vector.
        as_vector4 = Vector4 -> Vector4<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as four-dimensional vector array.
        as_vector4_array = Vector4Array -> [Vector4<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as 2x2 matrix.
        as_matrix2 = Matrix2 -> Matrix2<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as 2x2 matrix array.
        as_matrix2_array = Matrix2Array -> [Matrix2<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as 3x3 matrix.
        as_matrix3 = Matrix3 -> Matrix3<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as 3x3 matrix array.
        as_matrix3_array = Matrix3Array -> [Matrix3<f32>]
    );
    define_as!(
        /// Tries to unwrap property value as 4x4 matrix.
        as_matrix4 = Matrix4 -> Matrix4<f32>
    );
    define_as_ref!(
        /// Tries to unwrap property value as 4x4 matrix array.
        as_matrix4_array = Matrix4Array -> [Matrix4<f32>]
    );
}

impl Default for MaterialPropertyValue {
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
/// Usually standard shader is enough for most cases, [`Material`] even has a [`Material::standard()`]
/// method to create a material with standard shader:
///
/// ```no_run
/// # use fyrox_impl::{
/// #     material::shader::{ShaderResource, SamplerFallback},
/// #     asset::manager::ResourceManager,
/// #     material::{Material, MaterialPropertyValue},
/// #     core::sstorage::ImmutableString,
/// # };
/// # use fyrox_impl::resource::texture::Texture;
///
/// fn create_brick_material(resource_manager: ResourceManager) -> Material {
///     let mut material = Material::standard();
///
///     material.set_property(
///         "diffuseTexture",
///         resource_manager.request::<Texture>("Brick_DiffuseTexture.jpg")
///     ).unwrap();
///
///     material
/// }
/// ```
///
/// As you can see it is pretty simple with standard material, all you need is to set values to desired
/// properties and you good to go. All you need to do is to apply the material, for example it could be
/// mesh surface or some other place that supports materials. For the full list of properties of the
/// standard shader see [shader module docs](self::shader).
///
/// ## Custom material
///
/// Custom materials is a bit more complex, you need to get a shader instance using the resource manager
/// and then create the material and populate it with a set of property values.
///
/// ```no_run
/// # use fyrox_impl::{
/// #     asset::manager::ResourceManager,
/// #     material::{Material, MaterialPropertyValue},
/// #     core::{sstorage::ImmutableString, algebra::Vector3}
/// # };
/// # use fyrox_impl::material::shader::Shader;
///
/// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
///     let shader = resource_manager.request::<Shader>("my_grass_shader.ron").await.unwrap();
///
///     // Here we assume that the material really has the properties defined below.
///     let mut material = Material::from_shader(shader, Some(resource_manager));
///
///     material.set_property("windDirection", Vector3::new(1.0, 0.0, 0.5)).unwrap();
///
///     material
/// }
/// ```
///
/// As you can see it is only a bit more hard that with the standard shader. The main difference here is
/// that we using resource manager to get shader instance, and then we just use the instance to create
/// material instance. Then we populate properties as usual.
#[derive(Debug, Clone, Reflect)]
pub struct Material {
    shader: ShaderResource,
    // Why not hash map here? Simply because linear search works faster here. It happens, because
    // there's a limited amount of resources a material could contain, usually it is 32 at max.
    // This limitation comes from GPUs. Even PBR materials have just 13 resources required for
    // rendering. Linear search in such small amounts outperforms hash map lookups.
    resource_bindings: Vec<MaterialResourceBinding>,
}

#[derive(Debug, Visit, Clone, Reflect)]
enum OldMaterialProperty {
    Float(f32),
    FloatArray(Vec<f32>),
    Int(i32),
    IntArray(Vec<i32>),
    UInt(u32),
    UIntArray(Vec<u32>),
    Vector2(Vector2<f32>),
    Vector2Array(Vec<Vector2<f32>>),
    Vector3(Vector3<f32>),
    Vector3Array(Vec<Vector3<f32>>),
    Vector4(Vector4<f32>),
    Vector4Array(Vec<Vector4<f32>>),
    Matrix2(Matrix2<f32>),
    Matrix2Array(Vec<Matrix2<f32>>),
    Matrix3(Matrix3<f32>),
    Matrix3Array(Vec<Matrix3<f32>>),
    Matrix4(Matrix4<f32>),
    Matrix4Array(Vec<Matrix4<f32>>),
    Bool(bool),
    Color(Color),
    Sampler {
        value: Option<TextureResource>,
        fallback: SamplerFallback,
    },
}

impl Default for OldMaterialProperty {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

impl Visit for Material {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut shader = if region.is_reading() {
            // It is very important to give a proper default state to the shader resource
            // here. Its standard default is set to shared "Standard" shader. If it is left
            // as is, deserialization will modify the "Standard" shader and this will lead
            // to "amazing" results and hours of debugging.
            ShaderResource::default()
        } else {
            self.shader.clone()
        };
        shader.visit("Shader", &mut region)?;
        self.shader = shader;

        if region.is_reading() {
            // Backward compatibility.
            let mut old_properties = FxHashMap::<ImmutableString, OldMaterialProperty>::default();
            if old_properties.visit("Properties", &mut region).is_ok() {
                for (name, old_property) in &old_properties {
                    if let OldMaterialProperty::Sampler { value, fallback } = old_property {
                        if let Some(binding) = self.binding_mut(name.clone()) {
                            *binding = MaterialResourceBindingValue::Sampler {
                                value: value.clone(),
                                fallback: *fallback,
                            };
                        }
                    }
                }

                if let Some(MaterialResourceBindingValue::PropertyGroup(properties)) =
                    self.binding_mut("properties")
                {
                    for (name, old_property) in old_properties {
                        let result = match old_property {
                            OldMaterialProperty::Float(v) => properties.set_property(name, v),
                            OldMaterialProperty::FloatArray(v) => properties.set_property(name, v),
                            OldMaterialProperty::Int(v) => properties.set_property(name, v),
                            OldMaterialProperty::IntArray(v) => properties.set_property(name, v),
                            OldMaterialProperty::UInt(v) => properties.set_property(name, v),
                            OldMaterialProperty::UIntArray(v) => properties.set_property(name, v),
                            OldMaterialProperty::Vector2(v) => properties.set_property(name, v),
                            OldMaterialProperty::Vector2Array(v) => {
                                properties.set_property(name, v)
                            }
                            OldMaterialProperty::Vector3(v) => properties.set_property(name, v),
                            OldMaterialProperty::Vector3Array(v) => {
                                properties.set_property(name, v)
                            }
                            OldMaterialProperty::Vector4(v) => properties.set_property(name, v),
                            OldMaterialProperty::Vector4Array(v) => {
                                properties.set_property(name, v)
                            }
                            OldMaterialProperty::Matrix2(v) => properties.set_property(name, v),
                            OldMaterialProperty::Matrix2Array(v) => {
                                properties.set_property(name, v)
                            }
                            OldMaterialProperty::Matrix3(v) => properties.set_property(name, v),
                            OldMaterialProperty::Matrix3Array(v) => {
                                properties.set_property(name, v)
                            }
                            OldMaterialProperty::Matrix4(v) => properties.set_property(name, v),
                            OldMaterialProperty::Matrix4Array(v) => {
                                properties.set_property(name, v)
                            }
                            OldMaterialProperty::Bool(v) => properties.set_property(name, v),
                            OldMaterialProperty::Color(v) => properties.set_property(name, v),
                            _ => Ok(()),
                        };
                        Log::verify(result);
                    }
                }
            } else {
                self.resource_bindings
                    .visit("ResourceBindings", &mut region)?;
            }
        } else {
            self.resource_bindings
                .visit("ResourceBindings", &mut region)?;
        }

        Ok(())
    }
}

impl Default for Material {
    fn default() -> Self {
        Material::standard()
    }
}

impl TypeUuidProvider for Material {
    fn type_uuid() -> Uuid {
        uuid!("0e54fe44-0c58-4108-a681-d6eefc88c234")
    }
}

impl ResourceData for Material {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("Material", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

/// A set of possible errors that can occur when working with materials.
#[derive(Debug)]
pub enum MaterialError {
    /// A resource binding is missing.
    NoSuchResource {
        /// Name of the binding.
        property_name: String,
    },
    /// A property is missing.
    NoSuchProperty {
        /// Name of the property.
        property_name: String,
    },
    /// Attempt to set a value of wrong type to a property.
    PropertyTypeMismatch {
        /// Name of the property.
        property_name: String,
        /// Expected property value.
        expected: Box<MaterialPropertyValue>,
        /// Given property value.
        given: Box<MaterialPropertyValue>,
    },
    /// Attempt to set a value of wrong type to a property.
    ResourceBindingTypeMismatch {
        /// Name of the resource binding.
        binding_name: String,
        /// Expected binding value.
        expected: Box<MaterialResourceBindingValue>,
        /// Given binding value.
        given: Box<MaterialResourceBindingValue>,
    },
    /// Unable to read data source.
    Visit(VisitError),
}

impl From<VisitError> for MaterialError {
    fn from(value: VisitError) -> Self {
        Self::Visit(value)
    }
}

impl From<FileLoadError> for MaterialError {
    fn from(value: FileLoadError) -> Self {
        Self::Visit(VisitError::FileLoadError(value))
    }
}

impl Display for MaterialError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialError::NoSuchResource { property_name } => {
                write!(
                    f,
                    "Unable to find material resource binding {property_name}"
                )
            }
            MaterialError::NoSuchProperty { property_name } => {
                write!(f, "Unable to find material property {property_name}")
            }
            MaterialError::PropertyTypeMismatch {
                property_name,
                expected,
                given,
            } => {
                write!(
                    f,
                    "Attempt to set a value of wrong type \
                to {property_name} property. Expected: {expected:?}, given {given:?}"
                )
            }
            MaterialError::ResourceBindingTypeMismatch {
                binding_name,
                expected,
                given,
            } => {
                write!(
                    f,
                    "Attempt to set a value of wrong type \
                to {binding_name} resource binding. Expected: {expected:?}, given {given:?}"
                )
            }
            MaterialError::Visit(e) => {
                write!(f, "Failed to visit data source. Reason: {e:?}")
            }
        }
    }
}

lazy_static! {
    /// Standard PBR material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD: BuiltInResource<Material> = BuiltInResource::new_no_source(
        MaterialResource::new_ok(
            "__StandardMaterial".into(),
            Material::from_shader(ShaderResource::standard(), None),
        )
    );

    /// Standard 2D material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_2D: BuiltInResource<Material> = BuiltInResource::new_no_source(
        MaterialResource::new_ok(
            "__Standard2DMaterial".into(),
            Material::from_shader(ShaderResource::standard_2d(), None),
        )
    );

    /// Standard particle system material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_PARTICLE_SYSTEM: BuiltInResource<Material> = BuiltInResource::new_no_source(
        MaterialResource::new_ok(
            "__StandardParticleSystemMaterial".into(),
            Material::from_shader(ShaderResource::standard_particle_system(), None),
        )
    );

    /// Standard sprite material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_SPRITE: BuiltInResource<Material> = BuiltInResource::new_no_source(
        MaterialResource::new_ok(
            "__StandardSpriteMaterial".into(),
            Material::from_shader(ShaderResource::standard_sprite(), None),
        )
    );

    /// Standard terrain material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_TERRAIN: BuiltInResource<Material> = BuiltInResource::new_no_source(
        MaterialResource::new_ok(
            "__StandardTerrainMaterial".into(),
           Material::from_shader(ShaderResource::standard_terrain(), None),
        )
    );

    /// Standard two-sided material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_TWOSIDES: BuiltInResource<Material> = BuiltInResource::new_no_source(
        MaterialResource::new_ok(
            "__StandardTwoSidesMaterial".into(),
          Material::from_shader(ShaderResource::standard_twosides(), None),
        )
    );
}

impl Material {
    /// Creates a new instance of material with the standard shader. For the full list
    /// of properties of the standard material see [shader module docs](self::shader).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::{
    /// #     material::shader::{ShaderResource, SamplerFallback},
    /// #     asset::manager::ResourceManager,
    /// #     material::{Material, MaterialPropertyValue},
    /// #     core::sstorage::ImmutableString
    /// # };
    /// # use fyrox_impl::resource::texture::Texture;
    ///
    /// fn create_brick_material(resource_manager: ResourceManager) -> Material {
    ///     let mut material = Material::standard();
    ///
    ///     material.set_property(
    ///         "diffuseTexture",
    ///         resource_manager.request::<Texture>("Brick_DiffuseTexture.jpg")
    ///     ).unwrap();
    ///
    ///     material
    /// }
    /// ```
    pub fn standard() -> Self {
        Self::from_shader(ShaderResource::standard(), None)
    }

    /// Creates new instance of standard 2D material.
    pub fn standard_2d() -> Self {
        Self::from_shader(ShaderResource::standard_2d(), None)
    }

    /// Creates new instance of standard 2D material.
    pub fn standard_particle_system() -> Self {
        Self::from_shader(ShaderResource::standard_particle_system(), None)
    }

    /// Creates new instance of standard sprite material.
    pub fn standard_sprite() -> Self {
        Self::from_shader(ShaderResource::standard_sprite(), None)
    }

    /// Creates new instance of standard material that renders both sides of a face.
    pub fn standard_two_sides() -> Self {
        Self::from_shader(ShaderResource::standard_twosides(), None)
    }

    /// Creates new instance of standard terrain material.
    pub fn standard_terrain() -> Self {
        Self::from_shader(ShaderResource::standard_terrain(), None)
    }

    /// Creates a new material instance with given shader. Each property will have default values
    /// defined in the shader.
    ///
    /// It is possible to pass resource manager as a second argument, it is needed to correctly resolve
    /// default values of samplers in case if they are bound to some resources - shader's definition stores
    /// only paths to textures. If you pass [`None`], no resolving will be done and every sampler will
    /// have [`None`] as default value, which in its turn will force engine to use fallback sampler value.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::{
    /// #     asset::manager::ResourceManager,
    /// #     material::{Material, MaterialPropertyValue},
    /// #     core::{sstorage::ImmutableString, algebra::Vector3}
    /// # };
    /// # use fyrox_impl::material::shader::Shader;
    ///
    /// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
    ///     let shader = resource_manager.request::<Shader>("my_grass_shader.ron").await.unwrap();
    ///
    ///     // Here we assume that the material really has the properties defined below.
    ///     let mut material = Material::from_shader(shader, Some(resource_manager));
    ///
    ///     material.set_property("windDirection", Vector3::new(1.0, 0.0, 0.5)).unwrap();
    ///
    ///     material
    /// }
    /// ```
    pub fn from_shader(shader: ShaderResource, resource_manager: Option<ResourceManager>) -> Self {
        let data = shader.data_ref();

        let mut resource_bindings = Vec::with_capacity(data.definition.resources.len());
        for resource_definition in data.definition.resources.iter() {
            let resource_binding = MaterialResourceBindingValue::from_property_kind(
                &resource_definition.kind,
                resource_manager.as_ref(),
            );
            resource_bindings.push(MaterialResourceBinding {
                name: ImmutableString::new(&resource_definition.name),
                value: resource_binding,
            });
        }

        drop(data);

        Self {
            shader,
            resource_bindings,
        }
    }

    /// Loads a material from file.
    pub async fn from_file<P>(
        path: P,
        io: &dyn ResourceIo,
        resource_manager: ResourceManager,
    ) -> Result<Self, MaterialError>
    where
        P: AsRef<Path>,
    {
        let content = io.load_file(path.as_ref()).await?;
        let mut material = Material {
            shader: Default::default(),
            resource_bindings: Default::default(),
        };
        let mut visitor = Visitor::load_from_memory(&content)?;
        visitor.blackboard.register(Arc::new(resource_manager));
        material.visit("Material", &mut visitor)?;
        Ok(material)
    }

    /// Searches for a resource binding with the given name and returns immutable reference to it
    /// (if any).
    ///
    /// # Complexity
    ///
    /// O(N)
    pub fn binding_ref(
        &self,
        name: impl Into<ImmutableString>,
    ) -> Option<&MaterialResourceBindingValue> {
        let name = name.into();
        self.resource_bindings.iter().find_map(|binding| {
            if binding.name == name {
                Some(&binding.value)
            } else {
                None
            }
        })
    }

    /// Searches for a resource binding with the given name and returns mutable reference to it,
    /// allowing you to modify the value.
    ///
    /// # Complexity
    ///
    /// O(N)
    pub fn binding_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialResourceBindingValue> {
        let name = name.into();
        self.resource_bindings.iter_mut().find_map(|binding| {
            if binding.name == name {
                Some(&mut binding.value)
            } else {
                None
            }
        })
    }

    /// Searches for a property group binding with the given name and returns immutable reference to it
    /// (if any).
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    /// # use fyrox_impl::material::Material;
    ///
    /// let mut material = Material::standard();
    ///
    /// let color = material.property_group_ref("properties").unwrap().property_ref("diffuseColor").unwrap().as_color();
    /// ```
    pub fn property_group_ref(
        &self,
        name: impl Into<ImmutableString>,
    ) -> Option<&MaterialPropertyGroup> {
        self.binding_ref(name).and_then(|binding| match binding {
            MaterialResourceBindingValue::Sampler { .. } => None,
            MaterialResourceBindingValue::PropertyGroup(group) => Some(group),
        })
    }

    /// Searches for a property group binding with the given name and returns immutable reference to it
    /// (if any).
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use fyrox_core::color::Color;
    /// use fyrox_impl::core::sstorage::ImmutableString;
    /// # use fyrox_impl::material::Material;
    ///
    /// let mut material = Material::standard();
    ///
    /// let color = material.property_group_mut("properties").unwrap().set_property("diffuseColor", Color::RED).unwrap();
    /// ```
    pub fn property_group_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialPropertyGroup> {
        self.binding_mut(name).and_then(|binding| match binding {
            MaterialResourceBindingValue::Sampler { .. } => None,
            MaterialResourceBindingValue::PropertyGroup(group) => Some(group),
        })
    }

    /// Sets new value of the property with given name.
    ///
    /// # Type checking
    ///
    /// A new value must have the same type as in shader, otherwise an error will be generated.
    /// This helps to catch subtle bugs when you passing "almost" identical values to shader, like
    /// signed and unsigned integers - both have positive values, but GPU is very strict of what
    /// it expects as input value.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::material::{Material, MaterialPropertyValue};
    /// # use fyrox_impl::core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// assert!(material.set_property("diffuseColor", Color::WHITE).is_ok());
    /// ```
    pub fn bind(
        &mut self,
        name: impl Into<ImmutableString>,
        new_value: impl Into<MaterialResourceBindingValue>,
    ) -> Result<(), MaterialError> {
        let name = name.into();
        let new_value = new_value.into();
        if let Some(value) = self.binding_mut(name.clone()) {
            match (value, new_value) {
                (
                    MaterialResourceBindingValue::Sampler {
                        value: old_value,
                        fallback: old_fallback,
                    },
                    MaterialResourceBindingValue::Sampler { value, fallback },
                ) => {
                    *old_value = value;
                    *old_fallback = fallback;
                }
                (
                    MaterialResourceBindingValue::PropertyGroup(old_group),
                    MaterialResourceBindingValue::PropertyGroup(new_group),
                ) => {
                    *old_group = new_group;
                }
                (value, new_value) => {
                    return Err(MaterialError::ResourceBindingTypeMismatch {
                        binding_name: name.deref().to_owned(),
                        expected: Box::new(value.clone()),
                        given: Box::new(new_value),
                    })
                }
            }

            Ok(())
        } else {
            Err(MaterialError::NoSuchProperty {
                property_name: name.deref().to_owned(),
            })
        }
    }

    /// Sets new value of the property with given name.
    ///
    /// # Type checking
    ///
    /// A new value must have the same type as in shader, otherwise an error will be generated.
    /// This helps to catch subtle bugs when you passing "almost" identical values to shader, like
    /// signed and unsigned integers - both have positive values, but GPU is very strict of what
    /// it expects as input value.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::material::{Material, MaterialPropertyValue};
    /// # use fyrox_impl::core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// assert!(material.set_property("diffuseColor", Color::WHITE).is_ok());
    /// ```
    pub fn set_property(
        &mut self,
        name: impl Into<ImmutableString>,
        new_value: impl Into<MaterialPropertyValue>,
    ) -> Result<(), MaterialError> {
        self.property_group_mut("properties")
            .ok_or_else(|| MaterialError::NoSuchResource {
                property_name: "properties".to_string(),
            })
            .and_then(|binding| binding.set_property(name, new_value))
    }

    /// Adds missing properties with default values, removes non-existent properties. Does not modify any existing
    /// properties. This method has limited usage, that is mostly related to shader hot reloading.
    pub fn sync_to_shader(&mut self, resource_manager: &ResourceManager) {
        let temp_shader = Self::from_shader(self.shader.clone(), Some(resource_manager.clone()));
        let mut current_bindings =
            std::mem::replace(&mut self.resource_bindings, temp_shader.resource_bindings);

        for binding in self.resource_bindings.iter_mut() {
            if let Some(position) = current_bindings
                .iter()
                .position(|current_binding| current_binding.name == binding.name)
            {
                *binding = current_bindings.remove(position);
            }
        }
    }

    /// Returns a reference to current shader.
    pub fn shader(&self) -> &ShaderResource {
        &self.shader
    }

    /// Returns immutable reference to internal property storage.
    pub fn bindings(&self) -> &[MaterialResourceBinding] {
        &self.resource_bindings
    }

    /// Tries to find a sampler with the given name and returns its texture (if any).
    pub fn texture(&self, name: &str) -> Option<TextureResource> {
        self.resource_bindings.iter().find_map(|binding| {
            if binding.name.as_str() == name {
                if let MaterialResourceBindingValue::Sampler { ref value, .. } = binding.value {
                    return value.clone();
                }
            }
            None
        })
    }
}

/// Shared material is a material instance that can be used across multiple objects. It is useful
/// when you need to have multiple objects that have the same material.
///
/// Shared material is also tells a renderer that this material can be used for efficient rendering -
/// the renderer will be able to optimize rendering when it knows that multiple objects share the
/// same material.
pub type MaterialResource = Resource<Material>;

/// Extension methods for material resource.
pub trait MaterialResourceExtension {
    /// Creates a new material resource.
    ///
    /// # Hot Reloading
    ///
    /// You must use this method to create materials, if you want hot reloading to be reliable and
    /// prevent random crashes. Unlike [`Resource::new_ok`], this method ensures that correct vtable
    /// is used.  
    fn new(material: Material) -> Self;

    /// Creates a deep copy of the material resource.
    fn deep_copy(&self) -> MaterialResource;

    /// Creates a deep copy of the material resource and marks it as procedural.
    fn deep_copy_as_embedded(&self) -> MaterialResource {
        let material = self.deep_copy();
        let mut header = material.header();
        header.kind.make_embedded();
        drop(header);
        material
    }
}

impl MaterialResourceExtension for MaterialResource {
    #[inline(never)] // Prevents vtable mismatch when doing hot reloading.
    fn new(material: Material) -> Self {
        Self::new_ok(ResourceKind::Embedded, material)
    }

    fn deep_copy(&self) -> MaterialResource {
        let material_state = self.header();
        let kind = material_state.kind.clone();
        match material_state.state {
            ResourceState::Pending { .. } => MaterialResource::new_pending(kind),
            ResourceState::LoadError { ref error } => {
                MaterialResource::new_load_error(kind.clone(), error.clone())
            }
            ResourceState::Ok(ref material) => MaterialResource::new_ok(
                kind,
                ResourceData::as_any(&**material)
                    .downcast_ref::<Material>()
                    .unwrap()
                    .clone(),
            ),
        }
    }
}

pub(crate) fn visit_old_material(region: &mut RegionGuard) -> Option<MaterialResource> {
    let mut old_material = Arc::new(Mutex::new(Material::default()));
    if let Ok(mut inner) = region.enter_region("Material") {
        if old_material.visit("Value", &mut inner).is_ok() {
            return Some(MaterialResource::new_ok(
                Default::default(),
                old_material.lock().clone(),
            ));
        }
    }
    None
}

pub(crate) fn visit_old_texture_as_material<F>(
    region: &mut RegionGuard,
    make_default_material: F,
) -> Option<MaterialResource>
where
    F: FnOnce() -> Material,
{
    let mut old_texture: Option<TextureResource> = None;
    if let Ok(mut inner) = region.enter_region("Texture") {
        if old_texture.visit("Value", &mut inner).is_ok() {
            let mut material = make_default_material();
            Log::verify(material.bind("diffuseTexture", old_texture));
            return Some(MaterialResource::new_ok(Default::default(), material));
        }
    }
    None
}
