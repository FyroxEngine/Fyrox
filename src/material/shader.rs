//! Shader is a script for graphics card. This module contains everything related to shaders.
//!
//! For more info see [`Shader`] struct docs.

use crate::{
    asset::{define_new_resource, Resource, ResourceData, ResourceState},
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        io::{self, FileLoadError},
        sparse::AtomicIndex,
        visitor::prelude::*,
    },
    lazy_static::lazy_static,
    renderer::{
        cache::{shader::ShaderSet, CacheEntry},
        framework::framebuffer::DrawParameters,
    },
};
use ron::Error;
use serde::Deserialize;
use std::{
    borrow::Cow,
    io::Cursor,
    path::{Path, PathBuf},
};

/// A source code of the standard shader.
pub const STANDARD_SHADER_SRC: &str = include_str!("standard/standard.shader");

/// A source code of the standard terrain shader.
pub const STANDARD_TERRAIN_SHADER_SRC: &str = include_str!("standard/terrain.shader");

/// Internal state of the shader.
///
/// # Notes
///
/// Usually you don't need to access internals of the shader, but there sometimes could be a need to
/// read shader definition, to get supported passes and properties.
#[derive(Default, Debug)]
pub struct ShaderState {
    path: PathBuf,

    /// Shader definition contains description of properties and render passes.
    pub definition: ShaderDefinition,

    pub(in crate) cache_index: AtomicIndex<CacheEntry<ShaderSet>>,
}

impl Visit for ShaderState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.path.visit("Path", visitor)?;

        if visitor.is_reading() {
            if self.path == Path::new("Standard") {
                self.definition = ShaderDefinition::from_str(STANDARD_SHADER_SRC).unwrap();
            } else if self.path == Path::new("StandardTerrain") {
                self.definition = ShaderDefinition::from_str(STANDARD_TERRAIN_SHADER_SRC).unwrap();
            }
        }

        visitor.leave_region()
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
#[derive(Deserialize, Debug, PartialEq, Clone, Copy, Visit)]
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
#[derive(Deserialize, Debug, PartialEq)]
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
#[derive(Default, Deserialize, Debug, PartialEq)]
pub struct PropertyDefinition {
    /// A name of the property.
    pub name: String,
    /// A kind of property with default value.
    pub kind: PropertyKind,
}

/// A render pass definition. See [`Shader`] docs for more info about render passes.
#[derive(Default, Deserialize, Debug, PartialEq)]
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
#[derive(Default, Deserialize, Debug, PartialEq)]
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

impl ShaderState {
    pub(in crate) async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ShaderError> {
        let content = io::load_file(path.as_ref()).await?;
        Ok(Self {
            path: path.as_ref().to_owned(),
            definition: ShaderDefinition::from_buf(content)?,
            cache_index: Default::default(),
        })
    }

    pub(in crate) fn from_str<P: AsRef<Path>>(str: &str, path: P) -> Result<Self, ShaderError> {
        Ok(Self {
            path: path.as_ref().to_owned(),
            definition: ShaderDefinition::from_str(str)?,
            cache_index: Default::default(),
        })
    }
}

impl ResourceData for ShaderState {
    fn path(&self) -> Cow<Path> {
        Cow::from(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

/// A set of possible error variants that can occur during shader loading.
#[derive(Debug, thiserror::Error)]
pub enum ShaderError {
    /// An i/o error has occurred.
    #[error("A file load error has occurred {0:?}")]
    Io(FileLoadError),

    /// A parsing error has occurred.
    #[error("A parsing error has occurred {0:?}")]
    ParseError(ron::Error),
}

impl From<ron::Error> for ShaderError {
    fn from(e: Error) -> Self {
        Self::ParseError(e)
    }
}

impl From<FileLoadError> for ShaderError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

define_new_resource!(
    /// Shader is a script for graphics adapter, it defines how to draw an object.
    ///
    /// # Structure
    ///
    /// Shader has rigid structure that could be described in this code snipped:
    ///
    /// ```ron
    /// (
    ///     // A set of properties, there could be any amount of properties.
    ///     properties: [
    ///         (
    ///             // Each property must have a name. This name must match with respective
    ///             // uniforms! That's is the whole point of having properties.
    ///             name: "diffuseTexture",
    ///
    ///             // Value has limited set of possible variants.
    ///             value: Sampler(default: None, fallback: White)
    ///         )
    ///     ],
    ///
    ///     // A set of render passes (see a section `Render pass` for more info)
    ///     passes: [
    ///         (
    ///             // Name must match with the name of either standard render pass (see below) or
    ///             // one of your passes.
    ///             name: "Forward",
    ///
    ///             // A set of parameters that regulate renderer pipeline state.
    ///             // This is mandatory field of each render pass.
    ///             draw_parameters: DrawParameters(
    ///                 // A face to cull. Either Front or Back.
    ///                 cull_face: Some(Back),
    ///
    ///                 // Color mask. Defines which colors should be written to render target.
    ///                 color_write: ColorMask(
    ///                     red: true,
    ///                     green: true,
    ///                     blue: true,
    ///                     alpha: true,
    ///                 ),
    ///
    ///                 // Whether to modify depth buffer or not.
    ///                 depth_write: true,
    ///
    ///                 // Whether to use stencil test or not.
    ///                 stencil_test: None,
    ///
    ///                 // Whether to perform depth test when drawing.
    ///                 depth_test: true,
    ///
    ///                 // Blending options.
    ///                 blend: Some(BlendFunc(
    ///                     sfactor: SrcAlpha,
    ///                     dfactor: OneMinusSrcAlpha,
    ///                 )),
    ///
    ///                 // Stencil options.
    ///                 stencil_op: StencilOp(
    ///                     fail: Keep,
    ///                     zfail: Keep,
    ///                     zpass: Keep,
    ///                     write_mask: 0xFFFF_FFFF,
    ///                 ),
    ///             ),
    ///
    ///             // Vertex shader code.
    ///             vertex_shader:
    ///                 r#"
    ///                 #version 330 core
    ///
    ///                 layout(location = 0) in vec3 vertexPosition;
    ///                 layout(location = 1) in vec2 vertexTexCoord;
    ///
    ///                 uniform mat4 rg3d_worldViewProjection;
    ///
    ///                 out vec2 texCoord;
    ///
    ///                 void main()
    ///                 {
    ///                     texCoord = vertexTexCoord;
    ///                     gl_Position = rg3d_worldViewProjection * vertexPosition;
    ///                 }
    ///                 "#;
    ///
    ///             // Pixel shader code.
    ///             pixel_shader:
    ///                 r#"
    ///                 #version 330 core
    ///
    ///                 // Note that the name of this uniform match the name of the property up above.
    ///                 uniform sampler2D diffuseTexture;
    ///
    ///                 out vec4 FragColor;
    ///
    ///                 in vec2 texCoord;
    ///
    ///                 void main()
    ///                 {
    ///                     FragColor = diffuseColor * texture(diffuseTexture, texCoord);
    ///                 }
    ///                 "#;
    ///         )
    ///     ],
    /// )
    /// ```
    ///
    /// Shader should contain at least one render pass to actually do some job. A shader could not
    /// have properties at all. Currently only vertex and fragment programs are supported. Each
    /// program mush be written in GLSL. Comprehensive GLSL documentation can be found
    /// [here](https://www.khronos.org/opengl/wiki/Core_Language_(GLSL))
    ///
    /// # Render pass
    ///
    /// Modern rendering is a very complex thing that requires drawing an object multiple times
    /// with different "scripts". For example to draw an object with shadows you need to draw an
    /// object twice: one directly in a render target, and one in a shadow map. Such stages called
    /// render passes.
    ///
    /// Binding of shaders to render passes is done via names, each render pass has unique name.
    ///
    /// ## Predefined passes
    ///
    /// There are number of predefined render passes:
    ///
    /// - GBuffer - A pass that fills a set of render target sized textures with various data
    /// about each rendered object. These textures then are used for physically-based lighting.
    /// Use this pass when you want the standard lighting to work with your objects.
    ///
    /// - Forward - A pass that draws an object directly in render target. This pass is very
    /// limiting, it does not support lighting, shadows, etc. It should be only used to render
    /// translucent objects.
    ///
    /// - SpotShadow - A pass that emits depth values for an object, later this depth map will be
    /// used to render shadows.
    ///
    /// - PointShadow - A pass that emits distance from a fragment to a point light, later this depth
    /// map will be used to render shadows.
    ///
    /// # Built-in variables
    ///
    /// There are number of build-in variables that rg3d pass to each shader automatically:
    ///
    /// | Name                      | Type            | Description
    /// |---------------------------|-----------------|--------------------------------------------
    /// | rg3d_worldMatrix          | `Matrix4`       | Local-to-world transformation.
    /// | rg3d_worldViewProjection  | `Matrix4`       | Local-to-clip-space transform.
    /// | rg3d_boneMatrices         | `[Matrix4; 60]` | Array of bone matrices.
    /// | rg3d_useSkeletalAnimation | `Vector3`       | Whether skinned meshes is rendering or not.
    /// | rg3d_cameraPosition       | `Vector3`       | Position of the camera.
    /// | rg3d_usePOM               | `bool`          | Whether to use parallax mapping or not.
    /// | rg3d_lightPosition        | `Vector3`       | Light position.
    ///
    /// To use any of the variables, just define a uniform with appropriate name:
    ///
    /// ```glsl
    /// uniform mat4 rg3d_worldMatrix;
    /// uniform vec3 rg3d_cameraPosition;
    /// ```
    ///
    /// This list will be extended in future releases.
    ///
    /// # Drawing parameters
    ///
    /// Drawing parameters defines which GPU functions to use and at which state. For example, to render
    /// transparent objects you need to enable blending with specific blending rules. Or you need to disable
    /// culling to draw objects from both sides. This is when draw parameters comes in handy.
    ///
    /// There are relatively large list of drawing parameters and it could confuse a person who didn't get
    /// used to work with graphics. The following list should help you to use drawing parameters correctly.
    ///
    /// - cull_face
    ///     - Defines which side of polygon should be culled.
    ///     - **Possible values:** `None`, [Some(CullFace::XXX)](crate::renderer::framework::state::CullFace)
    ///
    /// - color_write:
    ///     - Defines which components of color should be written to a render target
    ///     - **Possible values:** [ColorMask](crate::renderer::framework::state::ColorMask)(...)
    ///
    ///  - depth_write:
    ///     - Whether to modify depth buffer or not.
    ///     - **Possible values:** `true/false`
    ///
    ///  - stencil_test:
    ///     - Whether to use stencil test or not.
    ///     - **Possible values:**
    ///         - `None`
    ///         - Some([StencilFunc](crate::renderer::framework::state::StencilFunc))
    ///
    ///  - depth_test:
    ///      - Whether to perform depth test when drawing.
    ///      - **Possible values:** `true/false`
    ///
    ///   - blend:
    ///      - Blending options.
    ///      - **Possible values:**
    ///         - `None`
    ///         - Some([BlendFunc](crate::renderer::framework::state::BlendFunc))
    ///
    ///   - stencil_op:
    ///      - Stencil options.
    ///      - **Possible values:** [StencilOp](crate::renderer::framework::state::StencilOp)
    ///
    /// # Standard shader
    ///
    /// By default rg3d uses standard material for rendering, it covers 95% of uses cases and it is very
    /// flexible. To get standard shader instance, use [`Shader::standard`]
    ///
    /// ```no_run
    /// # use rg3d::material::shader::Shader;
    ///
    /// let standard_shader = Shader::standard();
    /// ```
    ///
    /// Usually you don't need to get this shader manually, using of [Material::standard](super::Material::standard)
    /// is enough.
    Shader<ShaderState, ShaderError>
);

impl Shader {
    /// Creates new shader from given string. Input string must have the format defined in
    /// examples for [`Shader`].
    pub fn from_str<P: AsRef<Path>>(str: &str, path: P) -> Result<Self, ShaderError> {
        Ok(Self(Resource::new(ResourceState::Ok(
            ShaderState::from_str(str, path.as_ref())?,
        ))))
    }

    /// Returns an instance of standard shader.
    pub fn standard() -> Self {
        STANDARD.clone()
    }

    /// Returns an instance of standard terrain shader.
    pub fn standard_terrain() -> Self {
        STANDARD_TERRAIN.clone()
    }

    /// Returns a list of standard shader.
    pub fn standard_shaders() -> Vec<Shader> {
        vec![Self::standard(), Self::standard_terrain()]
    }
}

lazy_static! {
    static ref STANDARD: Shader = Shader(Resource::new(ResourceState::Ok(
        ShaderState::from_str(STANDARD_SHADER_SRC, "Standard").unwrap(),
    )));
}

lazy_static! {
    static ref STANDARD_TERRAIN: Shader = Shader(Resource::new(ResourceState::Ok(
        ShaderState::from_str(STANDARD_TERRAIN_SHADER_SRC, "StandardTerrain").unwrap(),
    )));
}

#[cfg(test)]
mod test {
    use crate::material::shader::{
        PropertyDefinition, PropertyKind, RenderPassDefinition, SamplerFallback, Shader,
        ShaderDefinition,
    };

    #[test]
    fn test_shader_load() {
        let code = r##"
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
            "##;

        let shader = Shader::from_str(code, "test").unwrap();
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
