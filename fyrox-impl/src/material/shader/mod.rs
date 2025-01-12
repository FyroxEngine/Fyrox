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

//! Shader is a script for graphics card, it defines how to draw an object. It also defines a set
//! of external resources needed for the rendering.
//!
//! # Structure
//!
//! Shader has rigid structure that could be described in this code snipped:
//!
//! ```ron
//! (
//!     // A set of resources, the maximum amount of resources is limited by your GPU. The engine
//!     // guarantees, that there could at least 16 textures and 16 resource groups per shader.
//!     resources: [
//!         (
//!             // Each resource binding must have a name.
//!             name: "diffuseTexture",
//!
//!             // Value has limited set of possible variants.
//!             value: Texture(kind: Sampler2D, fallback: White),
//!
//!             binding: 0
//!         ),
//!
//!         // The following property groups are built-in and provides useful data for each shader.
//!         (
//!             name: "fyrox_instanceData",
//!             kind: PropertyGroup([
//!                 // Autogenerated
//!             ]),
//!             binding: 1
//!         ),
//!         (
//!             name: "fyrox_boneMatrices",
//!             kind: PropertyGroup([
//!                 // Autogenerated
//!             ]),
//!             binding: 2
//!         ),
//!         (
//!             name: "fyrox_graphicsSettings",
//!             kind: PropertyGroup([
//!                 // Autogenerated
//!             ]),
//!             binding: 3
//!         ),
//!         (
//!             name: "fyrox_cameraData",
//!             kind: PropertyGroup([
//!                 // Autogenerated
//!             ]),
//!             binding: 4
//!         ),
//!         (
//!             name: "fyrox_lightData",
//!             kind: PropertyGroup([
//!                 // Autogenerated
//!             ]),
//!             binding: 5
//!         ),
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
//!                 depth_test: Some(Less),
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
//!
//!                 // Scissor box
//!                 scissor_box: Some(ScissorBox(
//!                     x: 10,
//!                     y: 20,
//!                     width: 100,
//!                     height: 30
//!                 ))
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
//!                 out vec2 texCoord;
//!
//!                 void main()
//!                 {
//!                     texCoord = vertexTexCoord;
//!                     gl_Position = fyrox_instanceData.worldViewProjection * vertexPosition;
//!                 }
//!                 "#;
//!
//!             // Pixel shader code.
//!             pixel_shader:
//!                 r#"
//!                 #version 330 core
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
//! - `GBuffer` - A pass that fills a set of textures (render targets) with various data about each
//! rendered object (depth, normal, albedo, etc.). These textures then are used for physically-based
//! lighting. Use this pass when you want the standard lighting to work with your objects.
//!
//! - `Forward` - A pass that draws an object directly in render target. This pass is very
//! limiting, it does not support lighting, shadows, etc. It should be only used to render
//! translucent objects.
//!
//! - `SpotShadow` - A pass that emits depth values for an object, later this depth map will be
//! used to render shadows.
//!
//! - `PointShadow` - A pass that emits distance from a fragment to a point light, later this depth
//! map will be used to render shadows.
//!
//! - `DirectionalShadow` - A pass that emits depth values for an object, later this depth map will be
//! used to render shadows for directional cascaded shadows.
//!
//! # Resources
//!
//! Each shader requires a specific set of external resources that will be used during the rendering.
//! This set is defined in `resources` section of the shader and could contain the following resources:
//!
//! - `Texture` - a texture of arbitrary type
//! - `PropertyGroup` - a group of numeric properties.
//!
//! ## Binding points
//!
//! Shader resource must define a unique (over its type) binding index. The engine will use these
//! points to prepare appropriate resource descriptor sets for GPU. Keep in mind, that binding point
//! indices are **unique** per each type of resource. This means that a set of texture resource could
//! use the same indices as property groups. The binding points must be unique in its group. If
//! there are more than one resource of a certain type, that shares the same binding point, the
//! engine will refuse to use such shader.
//!
//! ## Built-in resources
//!
//! There are number of built-in resources, that Fyrox will try to assign automatically if they're
//! defined in your shader, something like this:
//!
//! ```ron
//! (
//!     name: "fyrox_instanceData",
//!     kind: PropertyGroup([
//!         // Autogenerated
//!     ]),
//!     binding: 1
//! ),
//! ```
//!
//! The full list of built-in resources is defined below.
//!
//! ### `fyrox_instanceData`
//!
//! Property group. Provided for each rendered surface instance.
//!
//! | Name                 | Type       | Description                                 |
//! |----------------------|------------|---------------------------------------------|
//! | worldMatrix          | `mat4`     | Local-to-world transformation.              |
//! | worldViewProjection  | `mat4`     | Local-to-clip-space transform.              |
//! | blendShapesCount     | `int`      | Total amount of blend shapes.               |
//! | useSkeletalAnimation | `bool`     | Whether skinned meshes is rendering or not. |
//! | blendShapesWeights   | `vec4[32]` | Blend shape weights.                        |
//!
//! ### `fyrox_boneMatrices`
//!
//! Property group. Provided for each rendered surface, that has skeletal animation.
//!
//! | Name     | Type        | Description   |
//! |----------|-------------|---------------|
//! | matrices | `mat4[256]` | Bone matrices |
//!
//!
//! ### `fyrox_cameraData`
//!
//! Property group. Contains camera properties. It contains info not only about scene camera,
//! but also observer info when rendering shadow maps. In other words - it is generic observer
//! properties.
//!
//! | Name                 | Type       | Description                                      |
//! |----------------------|------------|--------------------------------------------------|
//! | viewProjectionMatrix | `mat4`     | World-to-clip-space transformation.              |
//! | position             | `vec3`     | World-space position of the camera.              |
//! | upVector             | `vec3`     | World-space up-vector of the camera.             |
//! | sideVector           | `vec3`     | World-space side-vector of the camera.           |
//! | zNear                | `float`    | Near clipping plane location.                    |
//! | zFar                 | `float`    | Far clipping plane location.                     |
//! | zRange               | `float`    | `zFar - zNear`                                   |
//!
//! ### `fyrox_lightData`
//!
//! Property group. Available only in shadow passes.
//!
//! | Name              | Type   | Description                                                |
//! |-------------------|--------|------------------------------------------------------------|
//! | lightPosition     | `vec3` | World-space light source position. Only for shadow passes. |
//! | ambientLightColor | `vec4` | Ambient lighting color of the scene.                       |
//!
//! ### `fyrox_lightsBlock`
//!
//! Property group. Information about visible light sources
//!
//! | Name              | Type       | Description                                             |
//! |------------------ |------------|---------------------------------------------------------|
//! | lightCount        | `int`      | Total amount of light sources visible on screen.        |
//! | lightsColorRadius | `vec4[16]` | Color (xyz) and radius (w) of light source              |
//! | lightsParameters  | `vec2[16]` | Hot-spot cone angle cos (x) and half cone angle cos (y) |
//! | lightsPosition    | `vec3[16]` | World-space light position.                             |
//! | lightsDirection   | `vec3[16]` | World-space light direction                             |
//!
//! ### `fyrox_graphicsSettings`
//!
//! Property group. Contains graphics options of the renderer.
//!
//! | Name   | Type       | Description                                       |
//! |--------|------------|---------------------------------------------------|
//! | usePom | `bool`     | Whether to use parallax occlusion mapping or not. |
//!
//! ### `fyrox_sceneDepth`
//!
//! Texture. Contains depth values of scene. Available **only** after opaque geometry is
//! rendered (read - G-Buffer is filled). Typical usage is something like this:
//!
//! ```ron
//! (
//!     name: "fyrox_sceneDepth",
//!     kind: Texture(kind: Sampler2D, fallback: White),
//!     binding: 1
//! ),
//! ```
//!
//! # Code generation
//!
//! Fyrox automatically generates code for resource bindings. This is made specifically to prevent
//! subtle mistakes. For example when you define this set of resources:
//!
//! ```ron
//! (
//!     name: "MyShader",
//!
//!     resources: [
//!         (
//!             name: "diffuseTexture",
//!             kind: Texture(kind: Sampler2D, fallback: White),
//!             binding: 0
//!         ),
//!         (
//!             name: "normalTexture",
//!             kind: Texture(kind: Sampler2D, fallback: Normal),
//!             binding: 1
//!         ),
//!         (
//!             name: "properties",
//!             kind: PropertyGroup([
//!                 (
//!                     name: "texCoordScale",
//!                     kind: Vector2((1.0, 1.0)),
//!                 ),
//!                 (
//!                     name: "diffuseColor",
//!                     kind: Color(r: 255, g: 255, b: 255, a: 255),
//!                 ),
//!             ]),
//!             binding: 0
//!         ),
//!     ]
//! )
//! ```
//!
//! The engine generates the following code and adds it to source code of every shader of every pass
//! automatically:
//!
//! ```glsl
//! uniform sampler2D diffuseTexture;
//! uniform sampler2D normalTexture;
//! struct Tproperties {
//!     vec2 texCoordScale;
//!     vec4 diffuseColor;
//! };
//! layout(std140) uniform Uproperties { Tproperties properties; }
//! ```
//!
//! The most important thing is that the engine keeps properties in the `struct Tproperties` in
//! correct order and forces `std140` layout on the generated uniform block. Since the engine knows
//! the layout of the properties from their definition section, it could easily form a memory block
//! with all required alignments and paddings that could be uploaded to GPU. The next important thing
//! is that the engine batches all the data needed into a large chunks of data and uploads them
//! all at once, which is much faster.
//!
//! # Drawing parameters
//!
//! Drawing parameters defines which GPU functions to use and at which state. For example, to render
//! transparent objects you need to enable blending with specific blending rules. Or you need to disable
//! culling to draw objects from both sides. This is when draw parameters comes in handy.
//!
//! There are relatively large list of drawing parameters, and it could confuse a person who didn't get
//! used to work with graphics. The following list should help you to use drawing parameters correctly.
//!
//! - cull_face
//!     - Defines which side of polygon should be culled.
//!     - **Possible values:** `None`, [Some(CullFace::XXX)](crate::renderer::framework::CullFace)
//!
//! - color_write:
//!     - Defines which components of color should be written to a render target
//!     - **Possible values:** [ColorMask](crate::renderer::framework::ColorMask)(...)
//!
//!  - depth_write:
//!     - Whether to modify depth buffer or not.
//!     - **Possible values:** `true/false`
//!
//!  - stencil_test:
//!     - Whether to use stencil test or not.
//!     - **Possible values:**
//!         - `None`
//!         - Some([StencilFunc](crate::renderer::framework::StencilFunc))
//!
//!  - depth_test:
//!      - Whether to perform depth test when drawing.
//!      - **Possible values:** `true/false`
//!
//!   - blend:
//!      - Blending options.
//!      - **Possible values:**
//!         - `None`
//!         - Some([BlendFunc](crate::renderer::framework::BlendFunc))
//!
//!   - stencil_op:
//!      - Stencil options.
//!      - **Possible values:** [StencilOp](crate::renderer::framework::StencilOp)
//!
//! # Standard shader
//!
//! By default, Fyrox uses standard material for rendering, it covers 95% of uses cases and it is very
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
    asset::{
        embedded_data_source, io::ResourceIo, manager::BuiltInResource, untyped::ResourceKind,
        Resource, ResourceData, SHADER_RESOURCE_UUID,
    },
    core::{
        io::FileLoadError, reflect::prelude::*, sparse::AtomicIndex, uuid::Uuid,
        visitor::prelude::*, TypeUuidProvider,
    },
    lazy_static::lazy_static,
    renderer::framework::{
        gpu_program::{ShaderProperty, ShaderPropertyKind},
        DrawParameters,
    },
};
use fyrox_core::algebra;
pub use fyrox_graphics::gpu_program::{
    SamplerFallback, ShaderResourceDefinition, ShaderResourceKind,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Display, Formatter},
    fs::File,
    io::{Cursor, Write},
    path::Path,
    sync::Arc,
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

/// A name of the standard tile shader.
pub const STANDARD_TILE_SHADER_NAME: &str = "StandardTile";

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
    /// A set of resource definitions.
    pub resources: Vec<ShaderResourceDefinition>,
}

impl ShaderDefinition {
    /// Maximum amount of simultaneous light sources that can be passed into a standard lights data
    /// block.
    pub const MAX_LIGHTS: usize = 16;

    /// Maximum amount of bone matrices per shader.
    pub const MAX_BONE_MATRICES: usize = 256;

    /// Maximum amount of blend shape weight groups (packed weights of blend shapes into vec4).
    pub const MAX_BLEND_SHAPE_WEIGHT_GROUPS: usize = 32;

    fn from_buf(buf: Vec<u8>) -> Result<Self, ShaderError> {
        let mut definition: ShaderDefinition = ron::de::from_reader(Cursor::new(buf))?;
        definition.generate_built_in_resources();
        Ok(definition)
    }

    fn from_str(str: &str) -> Result<Self, ShaderError> {
        let mut definition: ShaderDefinition = ron::de::from_str(str)?;
        definition.generate_built_in_resources();
        Ok(definition)
    }

    fn generate_built_in_resources(&mut self) {
        for resource in self.resources.iter_mut() {
            let ShaderResourceKind::PropertyGroup(ref mut properties) = resource.kind else {
                continue;
            };

            use ShaderPropertyKind::*;
            match resource.name.as_str() {
                "fyrox_cameraData" => {
                    properties.clear();
                    properties.extend([
                        ShaderProperty::new(
                            "viewProjectionMatrix",
                            Matrix4(algebra::Matrix4::identity()),
                        ),
                        ShaderProperty::new("position", Vector3(Default::default())),
                        ShaderProperty::new("upVector", Vector3(Default::default())),
                        ShaderProperty::new("sideVector", Vector3(Default::default())),
                        ShaderProperty::new("zNear", Float(0.0)),
                        ShaderProperty::new("zFar", Float(0.0)),
                        ShaderProperty::new("zRange", Float(0.0)),
                    ]);
                }
                "fyrox_lightData" => {
                    properties.clear();
                    properties.extend([
                        ShaderProperty::new("lightPosition", Vector3(Default::default())),
                        ShaderProperty::new("ambientLightColor", Vector4(Default::default())),
                    ]);
                }
                "fyrox_graphicsSettings" => {
                    properties.clear();
                    properties.extend([ShaderProperty::new("usePOM", Bool(false))]);
                }
                "fyrox_lightsBlock" => {
                    properties.clear();
                    properties.extend([
                        ShaderProperty::new("lightCount", Int(0)),
                        ShaderProperty::new(
                            "lightsColorRadius",
                            Vector4Array {
                                value: Default::default(),
                                max_len: Self::MAX_LIGHTS,
                            },
                        ),
                        ShaderProperty::new(
                            "lightsParameters",
                            Vector2Array {
                                value: Default::default(),
                                max_len: Self::MAX_LIGHTS,
                            },
                        ),
                        ShaderProperty::new(
                            "lightsPosition",
                            Vector3Array {
                                value: Default::default(),
                                max_len: Self::MAX_LIGHTS,
                            },
                        ),
                        ShaderProperty::new(
                            "lightsDirection",
                            Vector3Array {
                                value: Default::default(),
                                max_len: Self::MAX_LIGHTS,
                            },
                        ),
                    ])
                }
                "fyrox_instanceData" => {
                    properties.clear();
                    properties.extend([
                        ShaderProperty::new("worldMatrix", Matrix4(algebra::Matrix4::identity())),
                        ShaderProperty::new(
                            "worldViewProjection",
                            Matrix4(algebra::Matrix4::identity()),
                        ),
                        ShaderProperty::new("blendShapesCount", Int(0)),
                        ShaderProperty::new("useSkeletalAnimation", Bool(false)),
                        ShaderProperty::new(
                            "blendShapesWeights",
                            Vector4Array {
                                value: Default::default(),
                                max_len: Self::MAX_BLEND_SHAPE_WEIGHT_GROUPS,
                            },
                        ),
                    ]);
                }
                "fyrox_boneMatrices" => {
                    properties.clear();
                    properties.extend([ShaderProperty::new(
                        "matrices",
                        Matrix4Array {
                            value: Default::default(),
                            max_len: Self::MAX_BONE_MATRICES,
                        },
                    )])
                }
                _ => (),
            }
        }
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

    /// Creates a shader from string represented as raw bytes. This function will fail if the `bytes`
    /// does not contain Utf8-encoded string.
    pub fn from_string_bytes(bytes: &[u8]) -> Result<Self, ShaderError> {
        Ok(Self {
            definition: ShaderDefinition::from_str(
                std::str::from_utf8(bytes).map_err(|_| ShaderError::NotUtf8Source)?,
            )?,
            cache_index: Default::default(),
        })
    }
}

impl ResourceData for Shader {
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

    /// Bytes does not represent Utf8-encoded string.
    NotUtf8Source,
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
            ShaderError::NotUtf8Source => {
                write!(f, "Bytes does not represent Utf8-encoded string.")
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

    /// Returns an instance of standard tile shader.
    fn standard_tile() -> Self;

    /// Returns an instance of standard two-sides terrain shader.
    fn standard_twosides() -> Self;

    /// Returns a list of standard shader.
    fn standard_shaders() -> [&'static BuiltInResource<Shader>; 7];
}

impl ShaderResourceExtension for ShaderResource {
    fn from_str(str: &str, kind: ResourceKind) -> Result<Self, ShaderError> {
        Ok(Resource::new_ok(kind, Shader::from_string(str)?))
    }

    fn standard() -> Self {
        STANDARD.resource()
    }

    fn standard_2d() -> Self {
        STANDARD_2D.resource()
    }

    fn standard_particle_system() -> Self {
        STANDARD_PARTICLE_SYSTEM.resource()
    }

    fn standard_sprite() -> Self {
        STANDARD_SPRITE.resource()
    }

    fn standard_terrain() -> Self {
        STANDARD_TERRAIN.resource()
    }

    fn standard_tile() -> Self {
        STANDARD_TILE.resource()
    }

    fn standard_twosides() -> Self {
        STANDARD_TWOSIDES.resource()
    }

    fn standard_shaders() -> [&'static BuiltInResource<Shader>; 7] {
        [
            &STANDARD,
            &STANDARD_2D,
            &STANDARD_PARTICLE_SYSTEM,
            &STANDARD_SPRITE,
            &STANDARD_TERRAIN,
            &STANDARD_TWOSIDES,
            &STANDARD_TILE,
        ]
    }
}

lazy_static! {
    static ref STANDARD: BuiltInResource<Shader> =
        BuiltInResource::new(embedded_data_source!("standard/standard.shader"), |data| {
            ShaderResource::new_ok(
                STANDARD_SHADER_NAME.into(),
                Shader::from_string_bytes(data).unwrap(),
            )
        });
    static ref STANDARD_2D: BuiltInResource<Shader> = BuiltInResource::new(
        embedded_data_source!("standard/standard2d.shader"),
        |data| ShaderResource::new_ok(
            STANDARD_2D_SHADER_NAME.into(),
            Shader::from_string_bytes(data).unwrap(),
        )
    );
    static ref STANDARD_PARTICLE_SYSTEM: BuiltInResource<Shader> = BuiltInResource::new(
        embedded_data_source!("standard/standard_particle_system.shader"),
        |data| ShaderResource::new_ok(
            STANDARD_PARTICLE_SYSTEM_SHADER_NAME.into(),
            Shader::from_string_bytes(data).unwrap(),
        )
    );
    static ref STANDARD_SPRITE: BuiltInResource<Shader> = BuiltInResource::new(
        embedded_data_source!("standard/standard_sprite.shader"),
        |data| ShaderResource::new_ok(
            STANDARD_SPRITE_SHADER_NAME.into(),
            Shader::from_string_bytes(data).unwrap(),
        )
    );
    static ref STANDARD_TERRAIN: BuiltInResource<Shader> =
        BuiltInResource::new(embedded_data_source!("standard/terrain.shader"), |data| {
            ShaderResource::new_ok(
                STANDARD_TERRAIN_SHADER_NAME.into(),
                Shader::from_string_bytes(data).unwrap(),
            )
        });
    static ref STANDARD_TILE: BuiltInResource<Shader> =
        BuiltInResource::new(embedded_data_source!("standard/tile.shader"), |data| {
            ShaderResource::new_ok(
                STANDARD_TILE_SHADER_NAME.into(),
                Shader::from_string_bytes(data).unwrap(),
            )
        });
    static ref STANDARD_TWOSIDES: BuiltInResource<Shader> = BuiltInResource::new(
        embedded_data_source!("standard/standard-two-sides.shader"),
        |data| ShaderResource::new_ok(
            STANDARD_TWOSIDES_SHADER_NAME.into(),
            Shader::from_string_bytes(data).unwrap(),
        )
    );
}

#[cfg(test)]
mod test {
    use crate::material::shader::{
        RenderPassDefinition, SamplerFallback, ShaderDefinition, ShaderResource,
        ShaderResourceDefinition, ShaderResourceExtension, ShaderResourceKind,
    };
    use fyrox_graphics::gpu_program::SamplerKind;

    #[test]
    fn test_shader_load() {
        let code = r#"
            (
                name: "TestShader",

                resources: [
                    (
                        name: "diffuseTexture",
                        kind: Texture(kind: Sampler2D, fallback: White),
                        binding: 0
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
                            depth_test: Some(Less),
                            blend: None,
                            stencil_op: StencilOp(
                                fail: Keep,
                                zfail: Keep,
                                zpass: Keep,
                                write_mask: 0xFFFF_FFFF,
                            ),
                            scissor_box: None
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
            resources: vec![ShaderResourceDefinition {
                name: "diffuseTexture".into(),
                kind: ShaderResourceKind::Texture {
                    kind: SamplerKind::Sampler2D,
                    fallback: SamplerFallback::White,
                },
                binding: 0,
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
