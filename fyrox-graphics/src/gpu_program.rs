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
pub trait GpuProgramTrait: GpuProgramAsAny {}
define_shared_wrapper!(GpuProgram<dyn GpuProgramTrait>);

pub enum ShaderKind {
    Vertex,
    Fragment,
}

define_as_any_trait!(GpuShaderAsAny => GpuShaderTrait);
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
    PropertyGroup(Vec<ShaderProperty>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Reflect, Visit)]
pub enum ShaderPropertyKind {
    /// Real number.
    Float {
        #[serde(default)]
        value: f32,
    },

    /// Real number array.
    FloatArray {
        value: Vec<f32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Integer number.
    Int {
        #[serde(default)]
        value: i32,
    },

    /// Integer number array.
    IntArray {
        value: Vec<i32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Natural number.
    UInt {
        #[serde(default)]
        value: u32,
    },

    /// Natural number array.
    UIntArray {
        value: Vec<u32>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Boolean value.
    Bool {
        #[serde(default)]
        value: bool,
    },

    /// Two-dimensional vector.
    Vector2 {
        #[serde(default)]
        value: Vector2<f32>,
    },

    /// Two-dimensional vector array.
    Vector2Array {
        value: Vec<Vector2<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Three-dimensional vector.
    Vector3 {
        #[serde(default)]
        value: Vector3<f32>,
    },

    /// Three-dimensional vector array.
    Vector3Array {
        value: Vec<Vector3<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// Four-dimensional vector.
    Vector4 {
        #[serde(default)]
        value: Vector4<f32>,
    },

    /// Four-dimensional vector array.
    Vector4Array {
        value: Vec<Vector4<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 2x2 Matrix.
    Matrix2 {
        #[serde(default)]
        value: Matrix2<f32>,
    },

    /// 2x2 Matrix array.
    Matrix2Array {
        value: Vec<Matrix2<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 3x3 Matrix.
    Matrix3 {
        #[serde(default)]
        value: Matrix3<f32>,
    },

    /// 3x3 Matrix array.
    Matrix3Array {
        value: Vec<Matrix3<f32>>,
        /// `max_len` defines the maximum number of elements in the shader.
        max_len: usize,
    },

    /// 4x4 Matrix.
    Matrix4 {
        #[serde(default)]
        value: Matrix4<f32>,
    },

    /// 4x4 Matrix array.
    Matrix4Array {
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Reflect, Visit, Clone, Default)]
pub struct ShaderProperty {
    pub name: ImmutableString,
    pub kind: ShaderPropertyKind,
}

impl ShaderProperty {
    pub fn new(name: impl Into<ImmutableString>, kind: ShaderPropertyKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
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
    /// A name of the resource.
    pub name: ImmutableString,
    /// A kind of resource.
    pub kind: ShaderResourceKind,
    pub binding: usize,
}

impl ShaderResourceDefinition {
    pub fn is_built_in(&self) -> bool {
        self.name.starts_with("fyrox_")
    }
}
