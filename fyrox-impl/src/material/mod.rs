//! Material is a set of parameters for a shader. This module contains everything related to materials.
//!
//! See [Material struct docs](self::Material) for more info.

#![warn(missing_docs)]

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
    material::shader::{PropertyKind, SamplerFallback, ShaderResource, ShaderResourceExtension},
    resource::texture::{Texture, TextureResource},
};
use fxhash::FxHashMap;
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

pub mod loader;
pub mod shader;

/// A value of a property that will be used for rendering with a shader.
///
/// # Limitations
///
/// There is a limited set of possible types that can be passed to a shader, most of them are
/// just simple data types.
#[derive(Debug, Visit, Clone, Reflect)]
pub enum PropertyValue {
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
}

macro_rules! define_as {
    ($(#[$meta:meta])* $name:ident = $variant:ident -> $ty:ty) => {
        $(#[$meta])*
        pub fn $name(&self) -> Option<$ty> {
            if let PropertyValue::$variant(v) = self {
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
            if let PropertyValue::$variant(v) = self {
                Some(v)
            } else {
                None
            }
        }
    };
}

impl PropertyValue {
    /// Creates property value from its shader's representation.
    pub fn from_property_kind(
        kind: &PropertyKind,
        resource_manager: Option<&ResourceManager>,
    ) -> Self {
        match kind {
            PropertyKind::Float(value) => PropertyValue::Float(*value),
            PropertyKind::Int(value) => PropertyValue::Int(*value),
            PropertyKind::UInt(value) => PropertyValue::UInt(*value),
            PropertyKind::Vector2(value) => PropertyValue::Vector2(*value),
            PropertyKind::Vector3(value) => PropertyValue::Vector3(*value),
            PropertyKind::Vector4(value) => PropertyValue::Vector4(*value),
            PropertyKind::Color { r, g, b, a } => {
                PropertyValue::Color(Color::from_rgba(*r, *g, *b, *a))
            }
            PropertyKind::Matrix2(value) => PropertyValue::Matrix2(*value),
            PropertyKind::Matrix3(value) => PropertyValue::Matrix3(*value),
            PropertyKind::Matrix4(value) => PropertyValue::Matrix4(*value),
            PropertyKind::Bool(value) => PropertyValue::Bool(*value),
            PropertyKind::Sampler {
                default,
                fallback: usage,
            } => PropertyValue::Sampler {
                value: default
                    .as_ref()
                    .and_then(|path| resource_manager.map(|rm| rm.request::<Texture>(path))),
                fallback: *usage,
            },
            PropertyKind::FloatArray(value) => PropertyValue::FloatArray(value.clone()),
            PropertyKind::IntArray(value) => PropertyValue::IntArray(value.clone()),
            PropertyKind::UIntArray(value) => PropertyValue::UIntArray(value.clone()),
            PropertyKind::Vector2Array(value) => PropertyValue::Vector2Array(value.clone()),
            PropertyKind::Vector3Array(value) => PropertyValue::Vector3Array(value.clone()),
            PropertyKind::Vector4Array(value) => PropertyValue::Vector4Array(value.clone()),
            PropertyKind::Matrix2Array(value) => PropertyValue::Matrix2Array(value.clone()),
            PropertyKind::Matrix3Array(value) => PropertyValue::Matrix3Array(value.clone()),
            PropertyKind::Matrix4Array(value) => PropertyValue::Matrix4Array(value.clone()),
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

    /// Tries to unwrap property value as texture.
    pub fn as_sampler(&self) -> Option<TextureResource> {
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
/// Usually standard shader is enough for most cases, [`Material`] even has a [`Material::standard()`]
/// method to create a material with standard shader:
///
/// ```no_run
/// # use fyrox_impl::{
/// #     material::shader::{ShaderResource, SamplerFallback},
/// #     asset::manager::ResourceManager,
/// #     material::{Material, PropertyValue},
/// #     core::sstorage::ImmutableString,
/// # };
/// # use fyrox_impl::resource::texture::Texture;
///
/// fn create_brick_material(resource_manager: ResourceManager) -> Material {
///     let mut material = Material::standard();
///
///     material.set_property(
///         &ImmutableString::new("diffuseTexture"),
///         PropertyValue::Sampler {
///             value: Some(resource_manager.request::<Texture>("Brick_DiffuseTexture.jpg")),
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
/// #     material::{Material, PropertyValue},
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
///     material.set_property(
///         &ImmutableString::new("windDirection"),
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
#[derive(Debug, Clone, Reflect)]
pub struct Material {
    shader: ShaderResource,
    properties: FxHashMap<ImmutableString, PropertyValue>,
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
        self.properties.visit("Properties", &mut region)?;

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
    /// A property is missing.
    NoSuchProperty {
        /// Name of the property.
        property_name: String,
    },
    /// Attempt to set a value of wrong type to a property.
    TypeMismatch {
        /// Name of the property.
        property_name: String,
        /// Expected property value.
        expected: PropertyValue,
        /// Given property value.
        given: PropertyValue,
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
            MaterialError::NoSuchProperty { property_name } => {
                write!(f, "Unable to find material property {property_name}")
            }
            MaterialError::TypeMismatch {
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
            MaterialError::Visit(e) => {
                write!(f, "Failed to visit data source. Reason: {:?}", e)
            }
        }
    }
}

lazy_static! {
    /// Standard PBR material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD: MaterialResource = MaterialResource::new_ok(
        "__StandardMaterial".into(),
    Material::from_shader(ShaderResource::standard(), None),
    );
}

lazy_static! {
    /// Standard 2D material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_2D: MaterialResource = MaterialResource::new_ok(
        "__Standard2DMaterial".into(),
        Material::from_shader(ShaderResource::standard_2d(), None),
    );
}

lazy_static! {
    /// Standard particle system material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_PARTICLE_SYSTEM: MaterialResource = MaterialResource::new_ok(
        "__StandardParticleSystemMaterial".into(),
        Material::from_shader(ShaderResource::standard_particle_system(), None),
    );
}

lazy_static! {
    /// Standard sprite material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_SPRITE: MaterialResource = MaterialResource::new_ok(
        "__StandardSpriteMaterial".into(),
        Material::from_shader(ShaderResource::standard_sprite(), None),
    );
}

lazy_static! {
    /// Standard terrain material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_TERRAIN: MaterialResource = MaterialResource::new_ok(
        "__StandardTerrainMaterial".into(),
       Material::from_shader(ShaderResource::standard_terrain(), None),
    );
}

lazy_static! {
    /// Standard two-sided material. Keep in mind that this material is global, any modification
    /// of it will reflect on every other usage of it.
    pub static ref STANDARD_TWOSIDES: MaterialResource = MaterialResource::new_ok(
        "__StandardTwoSidesMaterial".into(),
      Material::from_shader(ShaderResource::standard_twosides(), None),
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
    /// #     material::{Material, PropertyValue},
    /// #     core::sstorage::ImmutableString
    /// # };
    /// # use fyrox_impl::resource::texture::Texture;
    ///
    /// fn create_brick_material(resource_manager: ResourceManager) -> Material {
    ///     let mut material = Material::standard();
    ///
    ///     material.set_property(
    ///         &ImmutableString::new("diffuseTexture"),
    ///         PropertyValue::Sampler {
    ///             value: Some(resource_manager.request::<Texture>("Brick_DiffuseTexture.jpg")),
    ///             fallback: SamplerFallback::White
    ///         })
    ///         .unwrap();
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
    /// #     material::{Material, PropertyValue},
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
    ///     material.set_property(
    ///         &ImmutableString::new("windDirection"),
    ///         PropertyValue::Vector3(Vector3::new(1.0, 0.0, 0.5))
    ///         )
    ///         .unwrap();
    ///
    ///     material
    /// }
    /// ```
    pub fn from_shader(shader: ShaderResource, resource_manager: Option<ResourceManager>) -> Self {
        let data = shader.data_ref();

        let mut property_values = FxHashMap::default();
        for property_definition in data.definition.properties.iter() {
            let value = PropertyValue::from_property_kind(
                &property_definition.kind,
                resource_manager.as_ref(),
            );
            property_values.insert(ImmutableString::new(&property_definition.name), value);
        }

        drop(data);

        Self {
            shader,
            properties: property_values,
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
            properties: Default::default(),
        };
        let mut visitor = Visitor::load_from_memory(&content)?;
        visitor.blackboard.register(Arc::new(resource_manager));
        material.visit("Material", &mut visitor)?;
        Ok(material)
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
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    /// # use fyrox_impl::material::Material;
    ///
    /// let mut material = Material::standard();
    ///
    /// let color = material.property_ref(&ImmutableString::new("diffuseColor")).unwrap().as_color();
    /// ```
    pub fn property_ref(&self, name: &ImmutableString) -> Option<&PropertyValue> {
        self.properties.get(name)
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
    /// # use fyrox_impl::material::{Material, PropertyValue};
    /// # use fyrox_impl::core::color::Color;
    /// # use fyrox_impl::core::sstorage::ImmutableString;
    ///
    /// let mut material = Material::standard();
    ///
    /// assert!(material.set_property(&ImmutableString::new("diffuseColor"), PropertyValue::Color(Color::WHITE)).is_ok());
    /// ```
    pub fn set_property(
        &mut self,
        name: &ImmutableString,
        new_value: PropertyValue,
    ) -> Result<(), MaterialError> {
        if let Some(value) = self.properties.get_mut(name) {
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
                (PropertyValue::FloatArray(old_value), PropertyValue::FloatArray(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Int(old_value), PropertyValue::Int(value)) => {
                    *old_value = value;
                }
                (PropertyValue::IntArray(old_value), PropertyValue::IntArray(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Bool(old_value), PropertyValue::Bool(value)) => {
                    *old_value = value;
                }
                (PropertyValue::UInt(old_value), PropertyValue::UInt(value)) => {
                    *old_value = value;
                }
                (PropertyValue::UIntArray(old_value), PropertyValue::UIntArray(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector2(old_value), PropertyValue::Vector2(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector2Array(old_value), PropertyValue::Vector2Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector3(old_value), PropertyValue::Vector3(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector3Array(old_value), PropertyValue::Vector3Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector4(old_value), PropertyValue::Vector4(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Vector4Array(old_value), PropertyValue::Vector4Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix2(old_value), PropertyValue::Matrix2(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix2Array(old_value), PropertyValue::Matrix2Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix3(old_value), PropertyValue::Matrix3(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix3Array(old_value), PropertyValue::Matrix3Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix4(old_value), PropertyValue::Matrix4(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Matrix4Array(old_value), PropertyValue::Matrix4Array(value)) => {
                    *old_value = value;
                }
                (PropertyValue::Color(old_value), PropertyValue::Color(value)) => {
                    *old_value = value;
                }
                (value, new_value) => {
                    return Err(MaterialError::TypeMismatch {
                        property_name: name.deref().to_owned(),
                        expected: value.clone(),
                        given: new_value,
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

    /// Sets a value for sampler at the given name. It is a shortcut for [`Self::set_property`]
    /// method with [`PropertyValue::Sampler`] and [`SamplerFallback::White`].
    pub fn set_texture(
        &mut self,
        name: &ImmutableString,
        texture: Option<TextureResource>,
    ) -> Result<(), MaterialError> {
        self.set_property(
            name,
            PropertyValue::Sampler {
                value: texture,
                fallback: SamplerFallback::White,
            },
        )
    }

    /// Adds missing properties with default values, removes non-existent properties. Does not modify any existing
    /// properties. This method has limited usage, that is mostly related to shader hot reloading. Returns `true`
    /// if the syncing was successful, `false` - if the shader resource is not loaded.
    pub fn sync_to_shader(&mut self, resource_manager: &ResourceManager) -> bool {
        let shader_kind = self.shader.kind().clone();
        if let Some(shader) = self.shader.state().data() {
            if shader.definition.properties.len() > self.properties.len() {
                // Some property was added to the shader, but missing in the material.
                for property_definition in shader.definition.properties.iter() {
                    let name = ImmutableString::new(&property_definition.name);
                    if !self.properties.contains_key(&name) {
                        // Add the property with default values.
                        self.properties.insert(
                            name.clone(),
                            PropertyValue::from_property_kind(
                                &property_definition.kind,
                                Some(resource_manager),
                            ),
                        );

                        Log::info(format!(
                            "Added {} property to the material instance, since it exists in the \
                            shader {}, but not in the material instance.",
                            name, shader_kind
                        ));
                    }
                }
            } else {
                // Some property was removed from the shader, but still exists in the material.
                for property_name in self.properties.keys().cloned().collect::<Vec<_>>() {
                    if shader
                        .definition
                        .properties
                        .iter()
                        .all(|p| p.name != property_name.as_ref())
                    {
                        self.properties.remove(&property_name);

                        Log::info(format!(
                            "Removing {} property from the material instance, since it does \
                        not exists in the shader {}.",
                            property_name, shader_kind
                        ));
                    }
                }
            }

            return true;
        }

        false
    }

    /// Returns a reference to current shader.
    pub fn shader(&self) -> &ShaderResource {
        &self.shader
    }

    /// Returns immutable reference to internal property storage.
    pub fn properties(&self) -> &FxHashMap<ImmutableString, PropertyValue> {
        &self.properties
    }

    /// Tries to find a sampler with the given name and returns its texture (if any).
    pub fn texture(&self, name: &str) -> Option<TextureResource> {
        self.properties.iter().find_map(|(property_name, value)| {
            if property_name.as_str() == name {
                if let PropertyValue::Sampler { value, .. } = value {
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
            Log::verify(material.set_property(
                &ImmutableString::new("diffuseTexture"),
                PropertyValue::Sampler {
                    value: old_texture,
                    fallback: SamplerFallback::White,
                },
            ));
            return Some(MaterialResource::new_ok(Default::default(), material));
        }
    }
    None
}
