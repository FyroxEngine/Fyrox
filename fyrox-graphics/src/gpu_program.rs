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

//! A GPU program is a collection of shaders that are linked together so that they
//! can run on the GPU to control how rendering is performed.

use crate::{
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        reflect::prelude::*,
        sstorage::ImmutableString,
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_shared_wrapper,
};
use fyrox_core::define_as_any_trait;
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

define_as_any_trait!(GpuProgramAsAny => GpuProgramTrait);
/// A trait for whatever objects a graphics server is using to represent programs.
/// There are no methods because all interactions with programs are done through the server,
/// such as with [`crate::server::GraphicsServer::create_program`]
/// and [`crate::framebuffer::GpuFrameBufferTrait::draw`].
pub trait GpuProgramTrait: GpuProgramAsAny {}
define_shared_wrapper!(GpuProgram<dyn GpuProgramTrait>);

/// A shader can be either a fragment shader that produces pixels or
/// a vertex shader that controls where triangles are drawn on the screen.
pub enum ShaderKind {
    /// A vertex shader takes vertices in world coordinates and mathematically
    /// transforms them into screen coordinates for rendering. The vertex shader
    /// runs before the fragment shader and creates the input to the fragment shader.
    Vertex,
    /// The fragment shader determines whether a pixel should be drawn and what color it should be.
    /// It uses the data produced by the vertex shader after it has been interpolated by the GPU
    /// for the particular pixel under consideration.
    Fragment,
}

define_as_any_trait!(GpuShaderAsAny => GpuShaderTrait);
/// A trait for whatever objects a graphics server is using to represent shaders.
/// There are no methods because all interactions with programs are done through the server,
/// such as with [`crate::server::GraphicsServer::create_shader`].
pub trait GpuShaderTrait: GpuShaderAsAny {}
define_shared_wrapper!(GpuShader<dyn GpuShaderTrait>);

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
#[derive(
    Serialize,
    Deserialize,
    Default,
    Debug,
    PartialEq,
    Clone,
    Copy,
    Visit,
    Eq,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
    TypeUuidProvider,
)]
#[type_uuid(id = "791b333c-eb3f-4279-97fe-cf2ba45c6d78")]
pub enum SamplerFallback {
    /// A 1x1px white texture.
    #[default]
    White,
    /// A 1x1px texture with (0, 1, 0) vector.
    Normal,
    /// A 1x1px black texture.
    Black,
    /// A 1x1x1 volume texture with 1 black pixel.
    Volume,
}

/// A sampler represents how the data of a texture is accessed, and different kinds of samplers
/// are intended for different kinds of textures.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone, Copy, Visit, Eq, Reflect)]
pub enum SamplerKind {
    /// A sampler for a 1D linear texture, a series of values that are indexed by a single coordinate
    /// and where each component of the value is a float.
    Sampler1D,
    /// A sampler for the usual 2D image texture, a flat area of values that are indexed by a pair of coordinates (x, y)
    /// and where each component of the value is a float.
    #[default]
    Sampler2D,
    /// A sampler for a 3D texture, a volume of values that are indexed by three coordinates (x, y, z)
    /// and where each component of the value is a float.
    Sampler3D,
    /// A sampler for six square 2D images where each image represents one face of a cube.
    /// It is indexed by a three coordinate direction from the center of the cube (x, y, z) where the magnitude of the coordinates do not matter.
    /// The sampler follows the direction of the coordinates until it finds a place on one of the six faces of the cube.
    /// Each component of the resulting value is a float.
    SamplerCube,
    /// A sampler for a 1D linear texture, a series of values that are indexed by a single coordinate
    /// and where each component of the value is an unsigned integer.
    USampler1D,
    /// A sampler for the usual 2D image texture, a flat area of values that are indexed by a pair of coordinates (x, y)
    /// and where each component of the value is an unsigned integer.
    USampler2D,
    /// A sampler for a 3D texture, a volume of values that are indexed by three coordinates (x, y, z)
    /// and where each component of the value is an unsigned integer.
    USampler3D,
    /// A sampler for six square 2D images where each image represents one face of a cube.
    /// It is indexed by a three coordinate direction from the center of the cube (x, y, z) where the magnitude of the coordinates do not matter.
    /// The sampler follows the direction of the coordinates until it finds a place on one of the six faces of the cube.
    /// Each component of the resulting value is an unsigned integer.
    USamplerCube,
}

/// Shader property with default value.
#[derive(Serialize, Deserialize, Debug, PartialEq, Reflect, Visit, Clone)]
pub enum ShaderResourceKind {
    /// A texture.
    Texture {
        /// Kind of the texture.
        kind: SamplerKind,

        /// Fallback value.
        ///
        /// Sometimes you don't want to set a value to a texture binding, or you even don't have the appropriate
        /// one. There is fallback value that helps you with such situations, it defines a set of values that
        /// will be fetched from a texture binding point when there is no actual texture.
        ///
        /// For example, standard shader has a lot of samplers defined: diffuse, normal, height, emission,
        /// mask, metallic, roughness, etc. In some situations you may not have all the textures, you have
        /// only diffuse texture, to keep rendering correct, each other property has appropriate fallback
        /// value. Normal sampler - a normal vector pointing up (+Y), height - zero, emission - zero, etc.
        ///
        /// Fallback value is also helpful to catch missing textures, you'll definitely know the texture is
        /// missing by very specific value in the fallback texture.
        fallback: SamplerFallback,
    },
    /// A list of properties with names and values that represents a uniform struct within the shader
    /// and the default values for each field of the struct.
    PropertyGroup(Vec<ShaderProperty>),
}

/// A data type and default value for a uniform within a shader.
/// When a material supplies an actual value, it is done using a `MaterialProperty` value
/// from the `fyrox-material` crate.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Reflect, Visit)]
pub enum ShaderPropertyKind {
    /// Real number.
    Float {
        /// Default value
        #[serde(default)]
        value: f32,
    },

    /// Real number array.
    FloatArray {
        /// Default value
        value: Vec<f32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Integer number.
    Int {
        /// Default value
        #[serde(default)]
        value: i32,
    },

    /// Integer number array.
    IntArray {
        /// Default value
        value: Vec<i32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Natural number.
    UInt {
        /// Default value
        #[serde(default)]
        value: u32,
    },

    /// Natural number array.
    UIntArray {
        /// Default value
        value: Vec<u32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Boolean value.
    Bool {
        /// Default value
        #[serde(default)]
        value: bool,
    },

    /// Two-dimensional vector.
    Vector2 {
        /// Default value
        #[serde(default)]
        value: Vector2<f32>,
    },

    /// Two-dimensional vector array.
    Vector2Array {
        /// Default value
        value: Vec<Vector2<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Three-dimensional vector.
    Vector3 {
        /// Default value
        #[serde(default)]
        value: Vector3<f32>,
    },

    /// Three-dimensional vector array.
    Vector3Array {
        /// Default value
        value: Vec<Vector3<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Four-dimensional vector.
    Vector4 {
        /// Default value
        #[serde(default)]
        value: Vector4<f32>,
    },

    /// Four-dimensional vector array.
    Vector4Array {
        /// Default value
        value: Vec<Vector4<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 2x2 Matrix.
    Matrix2 {
        /// Default value
        #[serde(default)]
        value: Matrix2<f32>,
    },

    /// 2x2 Matrix array.
    Matrix2Array {
        /// Default value
        value: Vec<Matrix2<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 3x3 Matrix.
    Matrix3 {
        /// Default value
        #[serde(default)]
        value: Matrix3<f32>,
    },

    /// 3x3 Matrix array.
    Matrix3Array {
        /// Default value
        value: Vec<Matrix3<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 4x4 Matrix.
    Matrix4 {
        /// Default value
        #[serde(default)]
        value: Matrix4<f32>,
    },

    /// 4x4 Matrix array.
    Matrix4Array {
        /// Default value
        value: Vec<Matrix4<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// An sRGB color.
    Color {
        /// Default Red.
        #[serde(default = "default_color_component")]
        r: u8,

        /// Default Green.
        #[serde(default = "default_color_component")]
        g: u8,

        /// Default Blue.
        #[serde(default = "default_color_component")]
        b: u8,

        /// Default Alpha.
        #[serde(default = "default_color_component")]
        a: u8,
    },
}

fn default_color_component() -> u8 {
    255
}

/// A uniform value that is supplied to a shader by a material.
#[derive(Serialize, Deserialize, Debug, PartialEq, Reflect, Visit, Clone, Default)]
pub struct ShaderProperty {
    /// The name of the value in the shader and when editing the value in the material.
    pub name: ImmutableString,
    /// The property's data type and default value.
    pub kind: ShaderPropertyKind,
}

impl ShaderProperty {
    /// Create a property with the given name and value.
    pub fn new(name: impl Into<ImmutableString>, kind: ShaderPropertyKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }

    /// Create a property with the 2x2 identity matrix as its value.
    pub fn new_matrix2(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Matrix2 {
                value: Matrix2::identity(),
            },
        )
    }

    /// Create a property with the 3x3 identity matrix as its value.
    pub fn new_matrix3(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Matrix3 {
                value: Matrix3::identity(),
            },
        )
    }

    /// Create a property with the 4x4 identity matrix as its value.
    pub fn new_matrix4(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Matrix4 {
                value: Matrix4::identity(),
            },
        )
    }

    /// Create a property with the vector (0,0) as its value.
    pub fn new_vector2(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Vector2 {
                value: Default::default(),
            },
        )
    }

    /// Create a property with the vector (0,0,0) as its value.
    pub fn new_vector3(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Vector3 {
                value: Default::default(),
            },
        )
    }

    /// Create a property with the vector (0,0,0,0) as its value.
    pub fn new_vector4(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Vector4 {
                value: Default::default(),
            },
        )
    }

    /// Create a property with the float 0.0 as its value.
    pub fn new_float(name: impl Into<ImmutableString>) -> Self {
        Self::new(name, ShaderPropertyKind::Float { value: 0.0 })
    }

    /// Create a property with false as its value.
    pub fn new_bool(name: impl Into<ImmutableString>) -> Self {
        Self::new(name, ShaderPropertyKind::Bool { value: false })
    }

    /// Create a property with the integer 0 as its value.
    pub fn new_int(name: impl Into<ImmutableString>) -> Self {
        Self::new(name, ShaderPropertyKind::Int { value: 0 })
    }

    /// Create a property with white as its value.
    pub fn new_color(name: impl Into<ImmutableString>) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        )
    }

    /// Create a property with an empty list of 4x4 matrices and the given maximum length for the list.
    pub fn new_mat4_f32_array(name: impl Into<ImmutableString>, max_len: usize) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Matrix4Array {
                value: Default::default(),
                max_len,
            },
        )
    }

    /// Create a property with an empty list of floats and the given maximum length for the list.
    pub fn new_f32_array(name: impl Into<ImmutableString>, max_len: usize) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::FloatArray {
                value: Default::default(),
                max_len,
            },
        )
    }

    /// Create a property with an empty list of vectors and the given maximum length for the list.
    pub fn new_vec4_f32_array(name: impl Into<ImmutableString>, max_len: usize) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Vector4Array {
                value: Default::default(),
                max_len,
            },
        )
    }

    /// Create a property with an empty list of vectors and the given maximum length for the list.
    pub fn new_vec3_f32_array(name: impl Into<ImmutableString>, max_len: usize) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Vector3Array {
                value: Default::default(),
                max_len,
            },
        )
    }

    /// Create a property with an empty list of vectors and the given maximum length for the list.
    pub fn new_vec2_f32_array(name: impl Into<ImmutableString>, max_len: usize) -> Self {
        Self::new(
            name,
            ShaderPropertyKind::Vector2Array {
                value: Default::default(),
                max_len,
            },
        )
    }
}

impl Default for ShaderPropertyKind {
    fn default() -> Self {
        Self::Float { value: 0.0 }
    }
}

impl Default for ShaderResourceKind {
    fn default() -> Self {
        Self::PropertyGroup(Default::default())
    }
}

/// Shader resource definition.
#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, Visit)]
pub struct ShaderResourceDefinition {
    /// The name of the uniform as it appears in the source code, ready to be passed to `glGetUniformLocation`.
    /// If the name begins with "fyrox_" then Fyrox will treat it specially and try to automatically generate
    /// the uniform's value based on its name, such as "fyrox_sceneDepth".
    pub name: ImmutableString,
    /// A kind of resource.
    pub kind: ShaderResourceKind,
    /// Each of a program's active uniform blocks has a corresponding uniform buffer binding point.
    /// Binding points for active uniform blocks are assigned using `glUniformBlockBinding`.
    /// For textures, `glUniform1i` is used to assign the texture's binding point to the texture uniform.
    pub binding: usize,
}

impl ShaderResourceDefinition {
    /// Fyrox provides certain resources to shaders automatically, without the resources needing
    /// to be part of the material. This method is used by the `fyrox-impl` crate to decide whether
    /// it should look for the resource in the material (if `false`) or whether it should assume
    /// that this resource will be among the automatically provided resources.
    pub fn is_built_in(&self) -> bool {
        self.name.starts_with("fyrox_")
    }
}
