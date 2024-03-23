//! Shader is a script for graphics card. This module contains everything related to shaders.
//!
//! Shader is a script for graphics adapter, it defines how to draw an object.
//!
//! # Structure
//!
//! Shader has rigid structure that could be described in this code snipped:
//!
//! ```ron
//! (
//!     // A set of properties, there could be any amount of properties.
//!     properties: [
//!         (
//!             // Each property must have a name. This name must match with respective
//!             // uniforms! That's is the whole point of having properties.
//!             name: "diffuseTexture",
//!
//!             // Value has limited set of possible variants.
//!             value: Sampler(default: None, fallback: White)
//!         )
//!     ],
//!
//!     // A set of render passes (see a section `Render pass` for more info)
//!     passes: [
//!         (
//!             // Name must match with the name of either standard render pass (see below) or
//!             // one of your passes.
//!             name: "Forward",
//!
//!             // A set of parameters that regulate renderer pipeline state.
//!             // This is mandatory field of each render pass.
//!             draw_parameters: DrawParameters(
//!                 // A face to cull. Either Front or Back.
//!                 cull_face: Some(Back),
//!
//!                 // Color mask. Defines which colors should be written to render target.
//!                 color_write: ColorMask(
//!                     red: true,
//!                     green: true,
//!                     blue: true,
//!                     alpha: true,
//!                 ),
//!
//!                 // Whether to modify depth buffer or not.
//!                 depth_write: true,
//!
//!                 // Whether to use stencil test or not.
//!                 stencil_test: None,
//!
//!                 // Whether to perform depth test when drawing.
//!                 depth_test: true,
//!
//!                 // Blending options.
//!                 blend: Some(BlendParameters(
//!                     func: BlendFunc(
//!                         sfactor: SrcAlpha,
//!                         dfactor: OneMinusSrcAlpha,
//!                         alpha_sfactor: SrcAlpha,
//!                         alpha_dfactor: OneMinusSrcAlpha,
//!                     ),
//!                     equation: BlendEquation(
//!                         rgb: Add,
//!                         alpha: Add
//!                     )
//!                 )),
//!
//!                 // Stencil options.
//!                 stencil_op: StencilOp(
//!                     fail: Keep,
//!                     zfail: Keep,
//!                     zpass: Keep,
//!                     write_mask: 0xFFFF_FFFF,
//!                 ),
//!             ),
//!
//!             // Vertex shader code.
//!             vertex_shader:
//!                 r#"
//!                 #version 330 core
//!
//!                 layout(location = 0) in vec3 vertexPosition;
//!                 layout(location = 1) in vec2 vertexTexCoord;
//!
//!                 uniform mat4 fyrox_worldViewProjection;
//!
//!                 out vec2 texCoord;
//!
//!                 void main()
//!                 {
//!                     texCoord = vertexTexCoord;
//!                     gl_Position = fyrox_worldViewProjection * vertexPosition;
//!                 }
//!                 "#;
//!
//!             // Pixel shader code.
//!             pixel_shader:
//!                 r#"
//!                 #version 330 core
//!
//!                 // Note that the name of this uniform match the name of the property up above.
//!                 uniform sampler2D diffuseTexture;
//!
//!                 out vec4 FragColor;
//!
//!                 in vec2 texCoord;
//!
//!                 void main()
//!                 {
//!                     FragColor = diffuseColor * texture(diffuseTexture, texCoord);
//!                 }
//!                 "#;
//!         )
//!     ],
//! )
//! ```
//!
//! Shader should contain at least one render pass to actually do some job. A shader could not
//! have properties at all. Currently only vertex and fragment programs are supported. Each
//! program mush be written in GLSL. Comprehensive GLSL documentation can be found
//! [here](https://www.khronos.org/opengl/wiki/Core_Language_(GLSL))
//!
//! # Render pass
//!
//! Modern rendering is a very complex thing that requires drawing an object multiple times
//! with different "scripts". For example to draw an object with shadows you need to draw an
//! object twice: one directly in a render target, and one in a shadow map. Such stages called
//! render passes.
//!
//! Binding of shaders to render passes is done via names, each render pass has unique name.
//!
//! ## Predefined passes
//!
//! There are number of predefined render passes:
//!
//! - GBuffer - A pass that fills a set of render target sized textures with various data
//! about each rendered object. These textures then are used for physically-based lighting.
//! Use this pass when you want the standard lighting to work with your objects.
//!
//! - Forward - A pass that draws an object directly in render target. This pass is very
//! limiting, it does not support lighting, shadows, etc. It should be only used to render
//! translucent objects.
//!
//! - SpotShadow - A pass that emits depth values for an object, later this depth map will be
//! used to render shadows.
//!
//! - PointShadow - A pass that emits distance from a fragment to a point light, later this depth
//! map will be used to render shadows.
//!
//! ## Built-in properties
//!
//! There are number of built-in properties, that Fyrox will try to assign automatically if they're defined
//! in your shader:
//!
//! | Name                       | Type         | Description                                                                                                       |
//! |----------------------------|--------------|-------------------------------------------------------------------------------------------------------------------|
//! | fyrox_worldMatrix          | `mat4`       | Local-to-world transformation.                                                                                    |
//! | fyrox_worldViewProjection  | `mat4`       | Local-to-clip-space transform.                                                                                    |
//! | fyrox_boneMatrices         | `sampler2D`  | Array of bone matrices packed into a texture. Use `S_FetchMatrix` built-in method to fetch a matrix by its index. |
//! | fyrox_useSkeletalAnimation | `bool`       | Whether skinned meshes is rendering or not.                                                                       |
//! | fyrox_cameraPosition       | `vec3`       | Position of the camera.                                                                                           |
//! | fyrox_usePOM               | `bool`       | Whether to use parallax mapping or not.                                                                           |
//! | fyrox_lightPosition        | `vec3`       | Light position.                                                                                                   |
//! | fyrox_blendShapesStorage   | `sampler3D`  | 3D texture of layered blend shape storage. Use `S_FetchBlendShapeOffsets` built-in method to fetch info.          |
//! | fyrox_blendShapesWeights   | `float[128]` | Weights of all available blend shapes.                                                                            |
//! | fyrox_blendShapesCount     | `int`        | Total amount of blend shapes.                                                                                     |
//!
//! To use any of the properties, just define a uniform with an appropriate name:
//!
//! ```glsl
//! uniform mat4 fyrox_worldMatrix;
//! uniform vec3 fyrox_cameraPosition;
//! ```
//!
//! This list will be extended in future releases.
//!
//! # Drawing parameters
//!
//! Drawing parameters defines which GPU functions to use and at which state. For example, to render
//! transparent objects you need to enable blending with specific blending rules. Or you need to disable
//! culling to draw objects from both sides. This is when draw parameters comes in handy.
//!
//! There are relatively large list of drawing parameters and it could confuse a person who didn't get
//! used to work with graphics. The following list should help you to use drawing parameters correctly.
//!
//! - cull_face
//!     - Defines which side of polygon should be culled.
//!     - **Possible values:** `None`, [Some(CullFace::XXX)](crate::renderer::framework::framebuffer::CullFace)
//!
//! - color_write:
//!     - Defines which components of color should be written to a render target
//!     - **Possible values:** [ColorMask](crate::renderer::framework::state::ColorMask)(...)
//!
//!  - depth_write:
//!     - Whether to modify depth buffer or not.
//!     - **Possible values:** `true/false`
//!
//!  - stencil_test:
//!     - Whether to use stencil test or not.
//!     - **Possible values:**
//!         - `None`
//!         - Some([StencilFunc](crate::renderer::framework::state::StencilFunc))
//!
//!  - depth_test:
//!      - Whether to perform depth test when drawing.
//!      - **Possible values:** `true/false`
//!
//!   - blend:
//!      - Blending options.
//!      - **Possible values:**
//!         - `None`
//!         - Some([BlendFunc](crate::renderer::framework::state::BlendFunc))
//!
//!   - stencil_op:
//!      - Stencil options.
//!      - **Possible values:** [StencilOp](crate::renderer::framework::state::StencilOp)
//!
//! # Standard shader
//!
//! By default Fyrox uses standard material for rendering, it covers 95% of uses cases and it is very
//! flexible. To get standard shader instance, use [`ShaderResource::standard`]
//!
//! ```no_run
//! # use fyrox_impl::material::shader::{ShaderResource, ShaderResourceExtension};
//!
//! let standard_shader = ShaderResource::standard();
//! ```
//!
//! Usually you don't need to get this shader manually, using of [Material::standard](super::Material::standard)
//! is enough.

use crate::{
    asset::{io::ResourceIo, untyped::ResourceKind, Resource, ResourceData, SHADER_RESOURCE_UUID},
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        io::FileLoadError,
        reflect::prelude::*,
        sparse::AtomicIndex,
        uuid::Uuid,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    lazy_static::lazy_static,
    renderer::framework::framebuffer::DrawParameters,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    any::Any,
    error::Error,
    fmt::{Display, Formatter},
    fs::File,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

pub mod loader;

/// A name of the standard shader.
pub const STANDARD_SHADER_NAME: &str = "Standard";

/// A source code of the standard shader.
pub const STANDARD_SHADER_SRC: &str = include_str!("standard/standard.shader");

/// A name of the standard 2D shader.
pub const STANDARD_2D_SHADER_NAME: &str = "Standard2D";

/// A source code of the standard 2D shader.
pub const STANDARD_2D_SHADER_SRC: &str = include_str!("standard/standard2d.shader");

/// A name of the standard particle system shader.
pub const STANDARD_PARTICLE_SYSTEM_SHADER_NAME: &str = "StandardParticleSystem";

/// A source code of the standard particle system shader.
pub const STANDARD_PARTICLE_SYSTEM_SHADER_SRC: &str =
    include_str!("standard/standard_particle_system.shader");

/// A source code of the standard sprite shader.
pub const STANDARD_SPRITE_SHADER_SRC: &str = include_str!("standard/standard_sprite.shader");

/// A name of the standard two-sides shader.
pub const STANDARD_TWOSIDES_SHADER_NAME: &str = "StandardTwoSides";

/// A source code of the standard two-sides shader.
pub const STANDARD_TWOSIDES_SHADER_SRC: &str = include_str!("standard/standard-two-sides.shader");

/// A name of the standard terrain shader.
pub const STANDARD_TERRAIN_SHADER_NAME: &str = "StandardTerrain";

/// A name of the standard sprite shader.
pub const STANDARD_SPRITE_SHADER_NAME: &str = "StandardSprite";

/// A source code of the standard terrain shader.
pub const STANDARD_TERRAIN_SHADER_SRC: &str = include_str!("standard/terrain.shader");

/// A list of names of standard shaders.
pub const STANDARD_SHADER_NAMES: [&str; 6] = [
    STANDARD_SHADER_NAME,
    STANDARD_2D_SHADER_NAME,
    STANDARD_PARTICLE_SYSTEM_SHADER_NAME,
    STANDARD_SPRITE_SHADER_NAME,
    STANDARD_TWOSIDES_SHADER_NAME,
    STANDARD_TERRAIN_SHADER_NAME,
];

/// A list of source code of standard shaders.
pub const STANDARD_SHADER_SOURCES: [&str; 6] = [
    STANDARD_SHADER_SRC,
    STANDARD_2D_SHADER_SRC,
    STANDARD_PARTICLE_SYSTEM_SHADER_SRC,
    STANDARD_SPRITE_SHADER_SRC,
    STANDARD_TWOSIDES_SHADER_SRC,
    STANDARD_TERRAIN_SHADER_SRC,
];

/// Internal state of the shader.
///
/// # Notes
///
/// Usually you don't need to access internals of the shader, but there sometimes could be a need to
/// read shader definition, to get supported passes and properties.
#[derive(Default, Debug, Reflect, Visit)]
pub struct Shader {
    /// Shader definition contains description of properties and render passes.
    #[visit(optional)]
    pub definition: ShaderDefinition,

    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) cache_index: Arc<AtomicIndex>,
}

impl TypeUuidProvider for Shader {
    fn type_uuid() -> Uuid {
        SHADER_RESOURCE_UUID
    }
}

/// A fallback value for the sampler.
///
/// # Notes
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Visit, Eq, Reflect)]
pub enum SamplerFallback {
    /// A 1x1px white texture.
    White,
    /// A 1x1px texture with (0, 1, 0) vector.
    Normal,
    /// A 1x1px black texture.
    Black,
}

impl Default for SamplerFallback {
    fn default() -> Self {
        Self::White
    }
}

/// Shader property with default value.
#[derive(Serialize, Deserialize, Debug, PartialEq, Reflect, Visit)]
pub enum PropertyKind {
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

    /// Boolean value.
    Bool(bool),

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

    /// An sRGB color.
    ///
    /// # Conversion
    ///
    /// The colors you see on your monitor are in sRGB color space, this is fine for simple cases
    /// of rendering, but not for complex things like lighting. Such things require color to be
    /// linear. Value of this variant will be automatically **converted to linear color space**
    /// before it passed to shader.
    Color {
        /// Default Red.
        r: u8,

        /// Default Green.
        g: u8,

        /// Default Blue.
        b: u8,

        /// Default Alpha.
        a: u8,
    },

    /// A texture.
    Sampler {
        /// Optional path to default texture.
        default: Option<PathBuf>,

        /// Default fallback value. See [`SamplerFallback`] for more info.
        fallback: SamplerFallback,
    },
}

impl Default for PropertyKind {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Shader property definition.
#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Reflect, Visit)]
pub struct PropertyDefinition {
    /// A name of the property.
    pub name: String,
    /// A kind of property with default value.
    pub kind: PropertyKind,
}

/// A render pass definition. See [`ShaderResource`] docs for more info about render passes.
#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq, Reflect, Visit)]
pub struct RenderPassDefinition {
    /// A name of render pass.
    pub name: String,
    /// A set of parameters that will be used in a render pass.
    pub draw_parameters: DrawParameters,
    /// A source code of vertex shader.
    pub vertex_shader: String,
    /// A source code of fragment shader.
    pub fragment_shader: String,
}

/// A definition of the shader.
#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Reflect, Visit)]
pub struct ShaderDefinition {
    /// A name of the shader.
    pub name: String,
    /// A set of render passes.
    pub passes: Vec<RenderPassDefinition>,
    /// A set of property definitions.
    pub properties: Vec<PropertyDefinition>,
}

impl ShaderDefinition {
    fn from_buf(buf: Vec<u8>) -> Result<Self, ShaderError> {
        Ok(ron::de::from_reader(Cursor::new(buf))?)
    }

    fn from_str(str: &str) -> Result<Self, ShaderError> {
        Ok(ron::de::from_str(str)?)
    }
}

impl Shader {
    /// Creates a shader from file.
    pub async fn from_file<P: AsRef<Path>>(
        path: P,
        io: &dyn ResourceIo,
    ) -> Result<Self, ShaderError> {
        let content = io.load_file(path.as_ref()).await?;
        Ok(Self {
            definition: ShaderDefinition::from_buf(content)?,
            cache_index: Default::default(),
        })
    }

    /// Creates a shader from string.
    pub fn from_string(str: &str) -> Result<Self, ShaderError> {
        Ok(Self {
            definition: ShaderDefinition::from_str(str)?,
            cache_index: Default::default(),
        })
    }
}

impl ResourceData for Shader {
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
        let mut file = File::create(path)?;
        file.write_all(
            ron::ser::to_string_pretty(&self.definition, PrettyConfig::default())?.as_bytes(),
        )?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

/// A set of possible error variants that can occur during shader loading.
#[derive(Debug)]
pub enum ShaderError {
    /// An i/o error has occurred.
    Io(FileLoadError),

    /// A parsing error has occurred.
    ParseError(ron::error::SpannedError),
}

impl Display for ShaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderError::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            ShaderError::ParseError(v) => {
                write!(f, "A parsing error has occurred {v:?}")
            }
        }
    }
}

impl From<ron::error::SpannedError> for ShaderError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::ParseError(e)
    }
}

impl From<FileLoadError> for ShaderError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

/// Type alias for shader resources.
pub type ShaderResource = Resource<Shader>;

/// Extension trait for shader resources.
pub trait ShaderResourceExtension: Sized {
    /// Creates new shader from given string. Input string must have the format defined in
    /// examples for [`ShaderResource`].
    fn from_str(str: &str, kind: ResourceKind) -> Result<Self, ShaderError>;

    /// Returns an instance of standard shader.
    fn standard() -> Self;

    /// Returns an instance of standard 2D shader.
    fn standard_2d() -> Self;

    /// Returns an instance of standard particle system shader.
    fn standard_particle_system() -> Self;

    /// Returns an instance of standard sprite shader.
    fn standard_sprite() -> Self;

    /// Returns an instance of standard terrain shader.
    fn standard_terrain() -> Self;

    /// Returns an instance of standard two-sides terrain shader.
    fn standard_twosides() -> Self;

    /// Returns a list of standard shader.
    fn standard_shaders() -> Vec<ShaderResource>;
}

impl ShaderResourceExtension for ShaderResource {
    fn from_str(str: &str, kind: ResourceKind) -> Result<Self, ShaderError> {
        Ok(Resource::new_ok(kind, Shader::from_string(str)?))
    }

    fn standard() -> Self {
        STANDARD.clone()
    }

    fn standard_2d() -> Self {
        STANDARD_2D.clone()
    }

    fn standard_particle_system() -> Self {
        STANDARD_PARTICLE_SYSTEM.clone()
    }

    fn standard_sprite() -> Self {
        STANDARD_SPRITE.clone()
    }

    fn standard_terrain() -> Self {
        STANDARD_TERRAIN.clone()
    }

    fn standard_twosides() -> Self {
        STANDARD_TWOSIDES.clone()
    }

    fn standard_shaders() -> Vec<ShaderResource> {
        vec![
            Self::standard(),
            Self::standard_2d(),
            Self::standard_particle_system(),
            Self::standard_sprite(),
            Self::standard_terrain(),
            Self::standard_twosides(),
        ]
    }
}

lazy_static! {
    static ref STANDARD: ShaderResource = ShaderResource::new_ok(
        STANDARD_SHADER_NAME.into(),
        Shader::from_string(STANDARD_SHADER_SRC).unwrap(),
    );
}

lazy_static! {
    static ref STANDARD_2D: ShaderResource = ShaderResource::new_ok(
        STANDARD_2D_SHADER_NAME.into(),
        Shader::from_string(STANDARD_2D_SHADER_SRC).unwrap(),
    );
}

lazy_static! {
    static ref STANDARD_PARTICLE_SYSTEM: ShaderResource = ShaderResource::new_ok(
        STANDARD_PARTICLE_SYSTEM_SHADER_NAME.into(),
        Shader::from_string(STANDARD_PARTICLE_SYSTEM_SHADER_SRC).unwrap(),
    );
}

lazy_static! {
    static ref STANDARD_SPRITE: ShaderResource = ShaderResource::new_ok(
        STANDARD_SPRITE_SHADER_NAME.into(),
        Shader::from_string(STANDARD_SPRITE_SHADER_SRC).unwrap(),
    );
}

lazy_static! {
    static ref STANDARD_TERRAIN: ShaderResource = ShaderResource::new_ok(
        STANDARD_TERRAIN_SHADER_NAME.into(),
        Shader::from_string(STANDARD_TERRAIN_SHADER_SRC).unwrap(),
    );
}

lazy_static! {
    static ref STANDARD_TWOSIDES: ShaderResource = ShaderResource::new_ok(
        STANDARD_TWOSIDES_SHADER_NAME.into(),
        Shader::from_string(STANDARD_TWOSIDES_SHADER_SRC).unwrap(),
    );
}

#[cfg(test)]
mod test {
    use crate::material::shader::{
        PropertyDefinition, PropertyKind, RenderPassDefinition, SamplerFallback, ShaderDefinition,
        ShaderResource, ShaderResourceExtension,
    };

    #[test]
    fn test_shader_load() {
        let code = r#"
            (
                name: "TestShader",

                properties: [
                    (
                        name: "diffuseTexture",
                        kind: Sampler(value: None, fallback: White),
                    ),
                ],

                passes: [
                    (
                        name: "GBuffer",
                        draw_parameters: DrawParameters(
                            cull_face: Some(Back),
                            color_write: ColorMask(
                                red: true,
                                green: true,
                                blue: true,
                                alpha: true,
                            ),
                            depth_write: true,
                            stencil_test: None,
                            depth_test: true,
                            blend: None,
                            stencil_op: StencilOp(
                                fail: Keep,
                                zfail: Keep,
                                zpass: Keep,
                                write_mask: 0xFFFF_FFFF,
                            ),
                        ),
                        vertex_shader: "<CODE>",
                        fragment_shader: "<CODE>",
                    ),
                ],
            )
            "#;

        let shader = ShaderResource::from_str(code, "test".into()).unwrap();
        let data = shader.data_ref();

        let reference_definition = ShaderDefinition {
            name: "TestShader".to_owned(),
            properties: vec![PropertyDefinition {
                name: "diffuseTexture".to_string(),
                kind: PropertyKind::Sampler {
                    default: None,
                    fallback: SamplerFallback::White,
                },
            }],
            passes: vec![RenderPassDefinition {
                name: "GBuffer".to_string(),
                draw_parameters: Default::default(),
                vertex_shader: "<CODE>".to_string(),
                fragment_shader: "<CODE>".to_string(),
            }],
        };

        assert_eq!(data.definition, reference_definition);
    }
}
