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

#![warn(missing_docs)]

use crate::{
    asset::{
        io::ResourceIo,
        manager::{BuiltInResource, ResourceManager},
        state::ResourceState,
        untyped::ResourceKind,
        Resource, ResourceData,
    },
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        io::FileError,
        parking_lot::Mutex,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        visitor::{prelude::*, RegionGuard},
        TypeUuidProvider,
    },
    material::shader::{SamplerFallback, ShaderResource, ShaderResourceExtension},
    resource::texture::TextureResource,
};
use fxhash::FxHashMap;
use lazy_static::lazy_static;
use std::any::Any;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::Path,
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod loader;
pub mod shader;

/// A texture binding.
#[derive(Default, Debug, Visit, Clone, Reflect, TypeUuidProvider)]
#[type_uuid(id = "e1642a47-d372-4840-a8eb-f16350f436f8")]
pub struct MaterialTextureBinding {
    /// Actual value of the texture binding. Could be [`None`], in this case fallback value of the
    /// shader will be used.
    pub value: Option<TextureResource>,
}

/// A value of a resource binding that will be used for rendering.
///
/// # Limitations
///
/// There is a limited set of possible types that can be passed to a shader, most of them are
/// just simple data types.
#[derive(Debug, Visit, Clone, Reflect, TypeUuidProvider, AsRefStr, EnumString, VariantNames)]
#[type_uuid(id = "2df8f1e5-0075-4d0d-9860-70fc27d3e165")]
pub enum MaterialResourceBinding {
    /// A texture.
    Texture(MaterialTextureBinding),
    /// A group of properties.
    PropertyGroup(MaterialPropertyGroup),
}

impl Default for MaterialResourceBinding {
    fn default() -> Self {
        Self::PropertyGroup(Default::default())
    }
}

impl MaterialResourceBinding {
    /// Tries to extract a texture from the resource binding.
    pub fn as_texture(&self) -> Option<TextureResource> {
        if let Self::Texture(binding) = self {
            binding.value.clone()
        } else {
            None
        }
    }
}

/// Property group stores a bunch of named values of a fixed set of types, that will be used for
/// rendering with some shader.
#[derive(Default, Debug, Visit, Clone, Reflect)]
pub struct MaterialPropertyGroup {
    properties: FxHashMap<ImmutableString, MaterialProperty>,
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
    /// # use fyrox_impl::material::MaterialPropertyGroup;
    ///
    /// let mut group = MaterialPropertyGroup::default();
    /// let color = group.property_ref("diffuseColor").unwrap().as_color();
    /// ```
    pub fn property_ref(&self, name: impl Into<ImmutableString>) -> Option<&MaterialProperty> {
        let name = name.into();
        self.properties.get(&name)
    }

    /// Searches for a property with given name.
    ///
    /// # Complexity
    ///
    /// O(1)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use fyrox_core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    /// # use fyrox_impl::material::{MaterialProperty, MaterialPropertyGroup};
    ///
    /// let mut group = MaterialPropertyGroup::default();
    /// *group.property_mut("diffuseColor").unwrap() = MaterialProperty::Color(Color::RED);
    /// ```
    pub fn property_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialProperty> {
        let name = name.into();
        self.properties.get_mut(&name)
    }

    /// Sets new value of the property with given name.
    ///
    /// # Type checking
    ///
    /// This method does not check if the property exists in the shader nor its type. Validation
    /// happens in the renderer, when it tries to use the property group. This is made for performance
    /// reasons.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::material::{Material, MaterialPropertyGroup};
    /// # use fyrox_impl::core::color::Color;
    ///
    /// let mut group = MaterialPropertyGroup::default();
    /// group.set_property("diffuseColor", Color::WHITE);
    /// ```
    pub fn set_property(
        &mut self,
        name: impl Into<ImmutableString>,
        new_value: impl Into<MaterialProperty>,
    ) {
        self.properties.insert(name.into(), new_value.into());
    }

    /// Removes the property from the group. The renderer will use shader defaults for this property.
    pub fn unset_property(&mut self, name: impl Into<ImmutableString>) -> Option<MaterialProperty> {
        self.properties.remove(&name.into())
    }

    /// Returns immutable reference to internal property storage.
    pub fn properties(&self) -> &FxHashMap<ImmutableString, MaterialProperty> {
        &self.properties
    }
}

/// A set of possible material property types.
#[derive(Debug, Visit, Clone, Reflect, AsRefStr, EnumString, VariantNames, TypeUuidProvider)]
#[type_uuid(id = "1c25018d-ab6e-4dca-99a6-e3d9639bc33c")]
pub enum MaterialProperty {
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
    Color(Color),
}

macro_rules! impl_from {
    ($variant:ident => $value_type:ty) => {
        impl From<$value_type> for MaterialProperty {
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

/// A set of possible material property types.
#[derive(Debug, Copy, Clone)]
pub enum MaterialPropertyRef<'a> {
    /// Real number.
    Float(&'a f32),

    /// Real number array.
    FloatArray(&'a [f32]),

    /// Integer number.
    Int(&'a i32),

    /// Integer number array.
    IntArray(&'a [i32]),

    /// Natural number.
    UInt(&'a u32),

    /// Natural number array.
    UIntArray(&'a [u32]),

    /// Two-dimensional vector.
    Vector2(&'a Vector2<f32>),

    /// Two-dimensional vector array.
    Vector2Array(&'a [Vector2<f32>]),

    /// Three-dimensional vector.
    Vector3(&'a Vector3<f32>),

    /// Three-dimensional vector array.
    Vector3Array(&'a [Vector3<f32>]),

    /// Four-dimensional vector.
    Vector4(&'a Vector4<f32>),

    /// Four-dimensional vector array.
    Vector4Array(&'a [Vector4<f32>]),

    /// 2x2 Matrix.
    Matrix2(&'a Matrix2<f32>),

    /// 2x2 Matrix array.
    Matrix2Array(&'a [Matrix2<f32>]),

    /// 3x3 Matrix.
    Matrix3(&'a Matrix3<f32>),

    /// 3x3 Matrix array.
    Matrix3Array(&'a [Matrix3<f32>]),

    /// 4x4 Matrix.
    Matrix4(&'a Matrix4<f32>),

    /// 4x4 Matrix array.
    Matrix4Array(&'a [Matrix4<f32>]),

    /// Boolean value.
    Bool(&'a bool),

    /// An sRGB color.
    Color(&'a Color),
}

impl MaterialProperty {
    /// Maps the inner value of the property to its respective reference.
    pub fn as_ref(&self) -> MaterialPropertyRef<'_> {
        match self {
            MaterialProperty::Float(v) => MaterialPropertyRef::Float(v),
            MaterialProperty::FloatArray(v) => MaterialPropertyRef::FloatArray(v),
            MaterialProperty::Int(v) => MaterialPropertyRef::Int(v),
            MaterialProperty::IntArray(v) => MaterialPropertyRef::IntArray(v),
            MaterialProperty::UInt(v) => MaterialPropertyRef::UInt(v),
            MaterialProperty::UIntArray(v) => MaterialPropertyRef::UIntArray(v),
            MaterialProperty::Vector2(v) => MaterialPropertyRef::Vector2(v),
            MaterialProperty::Vector2Array(v) => MaterialPropertyRef::Vector2Array(v),
            MaterialProperty::Vector3(v) => MaterialPropertyRef::Vector3(v),
            MaterialProperty::Vector3Array(v) => MaterialPropertyRef::Vector3Array(v),
            MaterialProperty::Vector4(v) => MaterialPropertyRef::Vector4(v),
            MaterialProperty::Vector4Array(v) => MaterialPropertyRef::Vector4Array(v),
            MaterialProperty::Matrix2(v) => MaterialPropertyRef::Matrix2(v),
            MaterialProperty::Matrix2Array(v) => MaterialPropertyRef::Matrix2Array(v),
            MaterialProperty::Matrix3(v) => MaterialPropertyRef::Matrix3(v),
            MaterialProperty::Matrix3Array(v) => MaterialPropertyRef::Matrix3Array(v),
            MaterialProperty::Matrix4(v) => MaterialPropertyRef::Matrix4(v),
            MaterialProperty::Matrix4Array(v) => MaterialPropertyRef::Matrix4Array(v),
            MaterialProperty::Bool(v) => MaterialPropertyRef::Bool(v),
            MaterialProperty::Color(v) => MaterialPropertyRef::Color(v),
        }
    }
}

macro_rules! impl_from_ref {
    ($variant:ident => $value_type:ty) => {
        impl<'a> From<&'a $value_type> for MaterialPropertyRef<'a> {
            fn from(value: &'a $value_type) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_from_ref!(Float => f32);
impl_from_ref!(FloatArray => [f32]);
impl_from_ref!(Int => i32);
impl_from_ref!(IntArray => [i32]);
impl_from_ref!(UInt => u32);
impl_from_ref!(UIntArray => [u32]);
impl_from_ref!(Vector2 => Vector2<f32>);
impl_from_ref!(Vector2Array => [Vector2<f32>]);
impl_from_ref!(Vector3 => Vector3<f32>);
impl_from_ref!(Vector3Array => [Vector3<f32>]);
impl_from_ref!(Vector4 => Vector4<f32>);
impl_from_ref!(Vector4Array => [Vector4<f32>]);
impl_from_ref!(Matrix2 => Matrix2<f32>);
impl_from_ref!(Matrix2Array => [Matrix2<f32>]);
impl_from_ref!(Matrix3 => Matrix3<f32>);
impl_from_ref!(Matrix3Array => [Matrix3<f32>]);
impl_from_ref!(Matrix4 => Matrix4<f32>);
impl_from_ref!(Matrix4Array => [Matrix4<f32>]);
impl_from_ref!(Bool => bool);
impl_from_ref!(Color => Color);

impl From<Option<TextureResource>> for MaterialResourceBinding {
    fn from(value: Option<TextureResource>) -> Self {
        Self::Texture(MaterialTextureBinding { value })
    }
}

impl From<TextureResource> for MaterialResourceBinding {
    fn from(value: TextureResource) -> Self {
        Self::Texture(MaterialTextureBinding { value: Some(value) })
    }
}

macro_rules! define_as {
    ($(#[$meta:meta])* $name:ident = $variant:ident -> $ty:ty) => {
        $(#[$meta])*
        pub fn $name(&self) -> Option<$ty> {
            if let MaterialProperty::$variant(v) = self {
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
            if let MaterialProperty::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }
    };
}

impl MaterialProperty {
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

impl Default for MaterialProperty {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Material defines a set of resource bindings for a shader. It could be textures and property groups.
/// Textures contains graphical information (such as diffuse, normal, height, emission, etc. maps),
/// while property groups contains numerical values (matrices, vectors, numbers, etc.).
///
/// Each resource binding can be changed in runtime giving you the ability to create animated materials.
/// However, in practice most materials are static, this means that once it created, it won't be
/// changed anymore.
///
/// Please keep in mind that the actual "rules" of drawing an entity are stored in the shader,
/// **material is only a storage** for specific uses of the shader.
///
/// Multiple materials can share the same shader, for example standard shader covers 95% of most
/// common use cases, and it is shared across multiple materials. The only difference are property
/// values, for example you can draw multiple cubes using the same shader, but with different
/// textures.
///
/// Material itself can be shared across multiple places as well as the shader. This gives you the
/// ability to render multiple objects with the same material efficiently.
///
/// # Performance
///
/// It is very important to re-use materials as much as possible, because the amount of materials used
/// per frame significantly correlates with performance. The more unique materials you have per frame,
/// the more work has to be done by the renderer and the video driver to render a frame and the more time
/// the frame will require for rendering, thus lowering your FPS.
///
/// # Examples
///
/// A material can only be created using a shader instance, every material must have a shader. The
/// shader provides information about its resources. Default values of each property defined in the
/// shader.
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
/// #     material::{Material, MaterialProperty},
/// #     core::sstorage::ImmutableString,
/// # };
/// # use fyrox_impl::resource::texture::Texture;
///
/// fn create_brick_material(resource_manager: ResourceManager) -> Material {
///     let mut material = Material::standard();
///
///     material.bind(
///         "diffuseTexture",
///         resource_manager.request::<Texture>("Brick_DiffuseTexture.jpg")
///     );
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
/// (or create a shader in-place by embedding its source code in the binary) and then create the material
/// and populate it with a set of property values.
///
/// ```no_run
/// # use fyrox_impl::{
/// #     asset::manager::ResourceManager,
/// #     material::{Material, MaterialProperty},
/// #     core::{sstorage::ImmutableString, algebra::Vector3}
/// # };
/// # use fyrox_impl::material::shader::Shader;
///
/// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
///     let shader = resource_manager.request::<Shader>("my_grass_shader.ron").await.unwrap();
///
///     // Here we assume that the material really has the properties defined below.
///     let mut material = Material::from_shader(shader);
///
///     material.set_property("windDirection", Vector3::new(1.0, 0.0, 0.5));
///
///     material
/// }
/// ```
///
/// As you can see it is slightly more complex that with the standard shader. The main difference here is
/// that we using resource manager to get shader instance, and then we just use the instance to create
/// material instance. Then we populate properties as usual.
#[derive(Debug, Clone, Reflect)]
pub struct Material {
    shader: ShaderResource,
    resource_bindings: FxHashMap<ImmutableString, MaterialResourceBinding>,
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
                    if let OldMaterialProperty::Sampler { value, .. } = old_property {
                        self.bind(
                            name.clone(),
                            MaterialResourceBinding::Texture(MaterialTextureBinding {
                                value: value.clone(),
                            }),
                        )
                    }
                }

                let properties = self.try_get_or_insert_property_group("properties");

                for (name, old_property) in old_properties {
                    match old_property {
                        OldMaterialProperty::Float(v) => properties.set_property(name, v),
                        OldMaterialProperty::FloatArray(v) => properties.set_property(name, v),
                        OldMaterialProperty::Int(v) => properties.set_property(name, v),
                        OldMaterialProperty::IntArray(v) => properties.set_property(name, v),
                        OldMaterialProperty::UInt(v) => properties.set_property(name, v),
                        OldMaterialProperty::UIntArray(v) => properties.set_property(name, v),
                        OldMaterialProperty::Vector2(v) => properties.set_property(name, v),
                        OldMaterialProperty::Vector2Array(v) => properties.set_property(name, v),
                        OldMaterialProperty::Vector3(v) => properties.set_property(name, v),
                        OldMaterialProperty::Vector3Array(v) => properties.set_property(name, v),
                        OldMaterialProperty::Vector4(v) => properties.set_property(name, v),
                        OldMaterialProperty::Vector4Array(v) => properties.set_property(name, v),
                        OldMaterialProperty::Matrix2(v) => properties.set_property(name, v),
                        OldMaterialProperty::Matrix2Array(v) => properties.set_property(name, v),
                        OldMaterialProperty::Matrix3(v) => properties.set_property(name, v),
                        OldMaterialProperty::Matrix3Array(v) => properties.set_property(name, v),
                        OldMaterialProperty::Matrix4(v) => properties.set_property(name, v),
                        OldMaterialProperty::Matrix4Array(v) => properties.set_property(name, v),
                        OldMaterialProperty::Bool(v) => properties.set_property(name, v),
                        OldMaterialProperty::Color(v) => properties.set_property(name, v),
                        _ => (),
                    };
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
    /// Unable to read data source.
    Visit(VisitError),
}

impl From<VisitError> for MaterialError {
    fn from(value: VisitError) -> Self {
        Self::Visit(value)
    }
}

impl From<FileError> for MaterialError {
    fn from(value: FileError) -> Self {
        Self::Visit(VisitError::FileLoadError(value))
    }
}

impl Display for MaterialError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialError::Visit(e) => {
                write!(f, "Failed to visit data source. Reason: {e:?}")
            }
        }
    }
}

lazy_static! {
    /// Standard PBR material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD: BuiltInResource<Material> = BuiltInResource::new_no_source("__StandardMaterial",
        MaterialResource::new_ok(
            uuid!("fac37721-d1b8-422e-ae0c-83196ecd0a26"),
            ResourceKind::External,
            Material::from_shader(ShaderResource::standard()),
        )
    );

    /// Standard 2D material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_2D: BuiltInResource<Material> = BuiltInResource::new_no_source("__Standard2DMaterial",
        MaterialResource::new_ok(
            uuid!("fe78a0d0-d059-4156-bc63-c3d2e36ad4b6"),
            ResourceKind::External,
            Material::from_shader(ShaderResource::standard_2d()),
        )
    );

    /// Standard particle system material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_PARTICLE_SYSTEM: BuiltInResource<Material> = BuiltInResource::new_no_source(
        "__StandardParticleSystemMaterial",
        MaterialResource::new_ok(
            uuid!("5bebe6e5-4aeb-496f-88f6-abe2b1ac798b"),
            ResourceKind::External,
            Material::from_shader(ShaderResource::standard_particle_system(),),
        )
    );

    /// Standard sprite material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_SPRITE: BuiltInResource<Material> = BuiltInResource::new_no_source(
        "__StandardSpriteMaterial",
        MaterialResource::new_ok(
            uuid!("3e331786-baae-412b-9d99-7370174bca43"),
            ResourceKind::External,
            Material::from_shader(ShaderResource::standard_sprite()),
        )
    );

    /// Standard terrain material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_TERRAIN: BuiltInResource<Material> = BuiltInResource::new_no_source(
        "__StandardTerrainMaterial",
        MaterialResource::new_ok(
            uuid!("0e407e22-41ad-4763-9adb-9d2e86351ece"),
            ResourceKind::External,
            Material::from_shader(ShaderResource::standard_terrain()),
        )
    );

    /// Standard two-sided material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_TWOSIDES: BuiltInResource<Material> = BuiltInResource::new_no_source(
        "__StandardTwoSidesMaterial",
        MaterialResource::new_ok(
            uuid!("24115321-7766-495c-bc3a-75db2f73d26d"),
            ResourceKind::External,
           Material::from_shader(ShaderResource::standard_twosides()),
        )
    );
}

impl Material {
    /// Creates a new instance of material with the standard shader.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::{
    /// #     material::shader::{ShaderResource, SamplerFallback},
    /// #     asset::manager::ResourceManager,
    /// #     material::{Material, MaterialProperty},
    /// #     core::sstorage::ImmutableString
    /// # };
    /// # use fyrox_impl::resource::texture::Texture;
    ///
    /// fn create_brick_material(resource_manager: ResourceManager) -> Material {
    ///     let mut material = Material::standard();
    ///
    ///     material.bind(
    ///         "diffuseTexture",
    ///         resource_manager.request::<Texture>("Brick_DiffuseTexture.jpg")
    ///     );
    ///
    ///     material
    /// }
    /// ```
    pub fn standard() -> Self {
        Self::from_shader(ShaderResource::standard())
    }

    /// Creates new instance of standard 2D material.
    pub fn standard_2d() -> Self {
        Self::from_shader(ShaderResource::standard_2d())
    }

    /// Creates new instance of standard 2D material.
    pub fn standard_particle_system() -> Self {
        Self::from_shader(ShaderResource::standard_particle_system())
    }

    /// Creates new instance of standard sprite material.
    pub fn standard_sprite() -> Self {
        Self::from_shader(ShaderResource::standard_sprite())
    }

    /// Creates new instance of standard material that renders both sides of a face.
    pub fn standard_two_sides() -> Self {
        Self::from_shader(ShaderResource::standard_twosides())
    }

    /// Creates new instance of standard terrain material.
    pub fn standard_terrain() -> Self {
        Self::from_shader(ShaderResource::standard_terrain())
    }

    /// Creates new instance of standard tile material.
    pub fn standard_tile() -> Self {
        Self::from_shader(ShaderResource::standard_tile())
    }

    /// Creates a new material instance with given shader. By default, a material does not store any
    /// resource bindings. In this case the renderer will use shader default values for rendering.
    /// Materials could be considered as container with values that overwrites shader values.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::{
    /// #     asset::manager::ResourceManager,
    /// #     material::{Material, MaterialProperty},
    /// #     core::{sstorage::ImmutableString, algebra::Vector3}
    /// # };
    /// # use fyrox_impl::material::shader::Shader;
    ///
    /// async fn create_grass_material(resource_manager: ResourceManager) -> Material {
    ///     let shader = resource_manager.request::<Shader>("my_grass_shader.ron").await.unwrap();
    ///
    ///     // Here we assume that the material really has the properties defined below.
    ///     let mut material = Material::from_shader(shader);
    ///
    ///     material.set_property("windDirection", Vector3::new(1.0, 0.0, 0.5));
    ///
    ///     material
    /// }
    /// ```
    pub fn from_shader(shader: ShaderResource) -> Self {
        Self {
            shader,
            resource_bindings: Default::default(),
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
    /// O(1)
    pub fn binding_ref(
        &self,
        name: impl Into<ImmutableString>,
    ) -> Option<&MaterialResourceBinding> {
        let name = name.into();
        self.resource_bindings.get(&name)
    }

    /// Searches for a resource binding with the given name and returns mutable reference to it,
    /// allowing you to modify the value.
    ///
    /// # Complexity
    ///
    /// O(1)
    pub fn binding_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialResourceBinding> {
        let name = name.into();
        self.resource_bindings.get_mut(&name)
    }

    /// Searches for a texture with the given name.
    pub fn texture_ref(&self, name: impl Into<ImmutableString>) -> Option<&MaterialTextureBinding> {
        if let Some(MaterialResourceBinding::Texture(binding)) = self.binding_ref(name) {
            Some(binding)
        } else {
            None
        }
    }

    /// Searches for a texture with the given name.
    pub fn texture_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialTextureBinding> {
        if let Some(MaterialResourceBinding::Texture(binding)) = self.binding_mut(name) {
            Some(binding)
        } else {
            None
        }
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
    /// let color = material
    ///     .property_group_ref("properties")
    ///     .unwrap()
    ///     .property_ref("diffuseColor")
    ///     .unwrap()
    ///     .as_color();
    /// ```
    pub fn property_group_ref(
        &self,
        name: impl Into<ImmutableString>,
    ) -> Option<&MaterialPropertyGroup> {
        self.binding_ref(name).and_then(|binding| match binding {
            MaterialResourceBinding::Texture { .. } => None,
            MaterialResourceBinding::PropertyGroup(group) => Some(group),
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
    /// let color = material
    ///     .property_group_mut("properties")
    ///     .unwrap()
    ///     .set_property("diffuseColor", Color::RED);
    /// ```
    pub fn property_group_mut(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> Option<&mut MaterialPropertyGroup> {
        self.binding_mut(name).and_then(|binding| match binding {
            MaterialResourceBinding::Texture { .. } => None,
            MaterialResourceBinding::PropertyGroup(group) => Some(group),
        })
    }

    /// Tries to find a property group with the given name, creates a new group with the given name
    /// if there's no such group.
    pub fn try_get_or_insert_property_group(
        &mut self,
        name: impl Into<ImmutableString>,
    ) -> &mut MaterialPropertyGroup {
        let name = name.into();
        if let MaterialResourceBinding::PropertyGroup(group) = self
            .resource_bindings
            .entry(name.clone())
            .or_insert_with(|| {
                MaterialResourceBinding::PropertyGroup(MaterialPropertyGroup::default())
            })
        {
            group
        } else {
            panic!("There's already a material resource binding with {name}!");
        }
    }

    /// Sets new value of the resource binding with given name.
    ///
    /// # Type checking
    ///
    /// A new value must have the same type as in shader, otherwise an error will be generated at
    /// attempt to render something with this material.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::material::{Material, MaterialProperty};
    /// # use fyrox_impl::core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// material.set_property("diffuseColor", Color::WHITE);
    /// ```
    pub fn bind(
        &mut self,
        name: impl Into<ImmutableString>,
        new_value: impl Into<MaterialResourceBinding>,
    ) {
        self.resource_bindings.insert(name.into(), new_value.into());
    }

    /// Tries to remove a resource bound to the given name.
    pub fn unbind(&mut self, name: impl Into<ImmutableString>) -> Option<MaterialResourceBinding> {
        self.resource_bindings.remove(&name.into())
    }

    /// Sets new value of the property with given name to the property group with `properties` name.
    /// It is a standard property group, that could be used to store pretty much any values.
    ///
    /// # Type checking
    ///
    /// A new value must have the same type as in shader, otherwise an error will be generated at
    /// attempt to render something with this material.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fyrox_impl::material::{Material, MaterialProperty};
    /// # use fyrox_impl::core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// material.set_property("diffuseColor", Color::WHITE);
    ///
    /// // A full equivalent of the above:
    /// material.try_get_or_insert_property_group("properties")
    ///         .set_property("diffuseColor", Color::WHITE);
    /// ```
    pub fn set_property(
        &mut self,
        name: impl Into<ImmutableString>,
        new_value: impl Into<MaterialProperty>,
    ) {
        self.try_get_or_insert_property_group("properties")
            .set_property(name, new_value);
    }

    /// Returns a reference to current shader.
    pub fn shader(&self) -> &ShaderResource {
        &self.shader
    }

    /// Returns immutable reference to internal property storage.
    pub fn bindings(&self) -> &FxHashMap<ImmutableString, MaterialResourceBinding> {
        &self.resource_bindings
    }

    /// Tries to find a sampler with the given name and returns its texture (if any).
    pub fn texture(&self, name: impl Into<ImmutableString>) -> Option<TextureResource> {
        self.resource_bindings.get(&name.into()).and_then(|v| {
            if let MaterialResourceBinding::Texture(ref binding) = v {
                binding.value.clone()
            } else {
                None
            }
        })
    }
}

/// Material resource is a material instance that can be used across multiple objects. It is useful
/// when you need to have multiple objects that have the same material.
///
/// Material resource is also tells a renderer that this material can be used for efficient rendering -
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
        Self::new_ok(Uuid::new_v4(), ResourceKind::Embedded, material)
    }

    fn deep_copy(&self) -> MaterialResource {
        let material_state = self.header();
        let kind = material_state.kind;
        match material_state.state {
            ResourceState::Pending { ref path, .. } => {
                MaterialResource::new_pending(path.clone(), kind)
            }
            ResourceState::LoadError { ref error, .. } => {
                MaterialResource::new_load_error(kind, error.clone())
            }
            ResourceState::Ok { ref data, .. } => MaterialResource::new_ok(
                Uuid::new_v4(),
                kind,
                (&**data as &dyn Any)
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
                Uuid::new_v4(),
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
            material.bind("diffuseTexture", old_texture);
            return Some(MaterialResource::new_ok(
                Uuid::new_v4(),
                Default::default(),
                material,
            ));
        }
    }
    None
}
