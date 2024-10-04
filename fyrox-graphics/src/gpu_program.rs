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

use crate::{
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        reflect::prelude::*,
        sstorage::ImmutableString,
        visitor::prelude::*,
    },
    error::FrameworkError,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::{marker::PhantomData, path::PathBuf};

pub trait GpuProgram: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn built_in_uniform_locations(&self) -> &[Option<UniformLocation>];
    fn built_in_uniform_blocks(&self) -> &[Option<usize>];
    fn uniform_location(&self, name: &ImmutableString) -> Result<UniformLocation, FrameworkError>;
    fn uniform_block_index(&self, name: &ImmutableString) -> Result<usize, FrameworkError>;
    fn set_bool(&self, location: &UniformLocation, value: bool);
    fn set_i32(&self, location: &UniformLocation, value: i32);
    fn set_u32(&self, location: &UniformLocation, value: u32);
    fn set_f32(&self, location: &UniformLocation, value: f32);
    fn set_vector2(&self, location: &UniformLocation, value: &Vector2<f32>);
    fn set_vector3(&self, location: &UniformLocation, value: &Vector3<f32>);
    fn set_vector4(&self, location: &UniformLocation, value: &Vector4<f32>);
    fn set_i32_slice(&self, location: &UniformLocation, value: &[i32]);
    fn set_u32_slice(&self, location: &UniformLocation, value: &[u32]);
    fn set_f32_slice(&self, location: &UniformLocation, value: &[f32]);
    fn set_vector2_slice(&self, location: &UniformLocation, value: &[Vector2<f32>]);
    fn set_vector3_slice(&self, location: &UniformLocation, value: &[Vector3<f32>]);
    fn set_vector4_slice(&self, location: &UniformLocation, value: &[Vector4<f32>]);
    fn set_matrix2(&self, location: &UniformLocation, value: &Matrix2<f32>);
    fn set_matrix2_array(&self, location: &UniformLocation, value: &[Matrix2<f32>]);
    fn set_matrix3(&self, location: &UniformLocation, value: &Matrix3<f32>);
    fn set_matrix3_array(&self, location: &UniformLocation, value: &[Matrix3<f32>]);
    fn set_matrix4(&self, location: &UniformLocation, value: &Matrix4<f32>);
    fn set_matrix4_array(&self, location: &UniformLocation, value: &[Matrix4<f32>]);
    fn set_linear_color(&self, location: &UniformLocation, value: &Color);
    fn set_srgb_color(&self, location: &UniformLocation, value: &Color);
}

#[repr(usize)]
pub enum BuiltInUniformBlock {
    BoneMatrices,
    InstanceData,
    CameraData,
    MaterialProperties,
    Count,
}

#[repr(usize)]
pub enum BuiltInUniform {
    SceneDepth,
    UsePOM,
    LightPosition,
    BlendShapesStorage,
    LightCount,
    LightsColorRadius,
    LightsPosition,
    LightsDirection,
    LightsParameters,
    AmbientLight,
    // Must be last.
    Count,
}

#[derive(Clone, Debug)]
pub struct UniformLocation {
    pub id: glow::UniformLocation,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    pub thread_mark: PhantomData<*const u8>,
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
#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone, Copy, Visit, Eq, Reflect)]
pub enum SamplerFallback {
    /// A 1x1px white texture.
    #[default]
    White,
    /// A 1x1px texture with (0, 1, 0) vector.
    Normal,
    /// A 1x1px black texture.
    Black,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Clone, Copy, Visit, Eq, Reflect)]
pub enum SamplerKind {
    Sampler1D,
    #[default]
    Sampler2D,
    Sampler3D,
    SamplerCube,
    USampler1D,
    USampler2D,
    USampler3D,
    USamplerCube,
}

impl SamplerKind {
    pub fn glsl_name(&self) -> &str {
        match self {
            SamplerKind::Sampler1D => "sampler1D",
            SamplerKind::Sampler2D => "sampler2D",
            SamplerKind::Sampler3D => "sampler3D",
            SamplerKind::SamplerCube => "samplerCube",
            SamplerKind::USampler1D => "usampler1D",
            SamplerKind::USampler2D => "usampler2D",
            SamplerKind::USampler3D => "usampler3D",
            SamplerKind::USamplerCube => "usamplerCube",
        }
    }
}

/// Shader property with default value.
#[derive(Serialize, Deserialize, Debug, PartialEq, Reflect, Visit)]
pub enum PropertyKind {
    /// Real number.
    Float(f32),

    /// Real number array.
    FloatArray {
        value: Vec<f32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Integer number.
    Int(i32),

    /// Integer number array.
    IntArray {
        value: Vec<i32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Natural number.
    UInt(u32),

    /// Natural number array.
    UIntArray {
        value: Vec<u32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Boolean value.
    Bool(bool),

    /// Two-dimensional vector.
    Vector2(Vector2<f32>),

    /// Two-dimensional vector array.
    Vector2Array {
        value: Vec<Vector2<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Three-dimensional vector.
    Vector3(Vector3<f32>),

    /// Three-dimensional vector array.
    Vector3Array {
        value: Vec<Vector3<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Four-dimensional vector.
    Vector4(Vector4<f32>),

    /// Four-dimensional vector array.
    Vector4Array {
        value: Vec<Vector4<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 2x2 Matrix.
    Matrix2(Matrix2<f32>),

    /// 2x2 Matrix array.
    Matrix2Array {
        value: Vec<Matrix2<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 3x3 Matrix.
    Matrix3(Matrix3<f32>),

    /// 3x3 Matrix array.
    Matrix3Array {
        value: Vec<Matrix3<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 4x4 Matrix.
    Matrix4(Matrix4<f32>),

    /// 4x4 Matrix array.
    Matrix4Array {
        value: Vec<Matrix4<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

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
        kind: SamplerKind,

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
    pub name: ImmutableString,
    /// A kind of property with default value.
    pub kind: PropertyKind,
}
