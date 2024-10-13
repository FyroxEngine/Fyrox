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

//! Contains all possible errors that may occur during rendering, initialization of
//! renderer structures, or GAPI.

use std::{
    error::Error,
    ffi::NulError,
    fmt::{Display, Formatter},
};

/// Set of possible renderer errors.
#[derive(Debug)]
pub enum FrameworkError {
    /// Compilation of a shader has failed.
    ShaderCompilationFailed {
        /// Name of shader.
        shader_name: String,
        /// Compilation error message.
        error_message: String,
    },
    /// Means that shader link stage failed, exact reason is inside `error_message`
    ShaderLinkingFailed {
        /// Name of shader.
        shader_name: String,
        /// Linking error message.
        error_message: String,
    },
    /// Shader source contains invalid characters.
    FaultyShaderSource,
    /// There is no such shader uniform (could be optimized out).
    UnableToFindShaderUniform(String),
    /// There is no such shader uniform block.
    UnableToFindShaderUniformBlock(String),
    /// Texture has invalid data - insufficient size.
    InvalidTextureData {
        /// Expected data size in bytes.
        expected_data_size: usize,
        /// Actual data size in bytes.
        actual_data_size: usize,
    },
    /// None variant was passed as texture data, but engine does not support it.
    EmptyTextureData,
    /// Means that you tried to draw element range from GeometryBuffer that
    /// does not have enough elements.
    InvalidElementRange {
        /// First index.
        start: usize,
        /// Last index.
        end: usize,
        /// Total amount of triangles.
        total: usize,
    },
    /// Means that attribute descriptor tries to define an attribute that does
    /// not exists in vertex, or it does not match size. For example you have vertex:
    ///   pos: float2,
    ///   normal: float3
    /// But you described second attribute as Float4, then you'll get this error.
    InvalidAttributeDescriptor,
    /// Framebuffer is invalid.
    InvalidFrameBuffer,
    /// OpenGL failed to construct framebuffer.
    FailedToConstructFBO,
    /// Custom error. Usually used for internal errors.
    Custom(String),
    /// Graphics server disconnected.
    GraphicsServerUnavailable,
}

impl Display for FrameworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameworkError::ShaderCompilationFailed {
                shader_name,
                error_message,
            } => {
                write!(
                    f,
                    "Compilation of \"{shader_name}\" shader has failed: {error_message}",
                )
            }
            FrameworkError::ShaderLinkingFailed {
                shader_name,
                error_message,
            } => {
                write!(
                    f,
                    "Linking shader \"{shader_name}\" failed: {error_message}",
                )
            }
            FrameworkError::FaultyShaderSource => {
                write!(f, "Shader source contains invalid characters")
            }
            FrameworkError::UnableToFindShaderUniform(v) => {
                write!(f, "There is no such shader uniform: {v}")
            }
            FrameworkError::UnableToFindShaderUniformBlock(v) => {
                write!(f, "There is no such shader uniform block: {v}")
            }
            FrameworkError::InvalidTextureData {
                expected_data_size,
                actual_data_size,
            } => {
                write!(
                    f,
                    "Texture has invalid data (insufficent size): \
                expected {expected_data_size}, actual: {actual_data_size}",
                )
            }
            FrameworkError::EmptyTextureData => {
                write!(
                    f,
                    "None variant was passed as texture data, but engine does not support it."
                )
            }
            FrameworkError::InvalidElementRange { start, end, total } => {
                write!(
                    f,
                    "Tried to draw element from GeometryBuffer that does not have enough \
                    elements: start: {start}, end: {end}, total: {total}",
                )
            }
            FrameworkError::InvalidAttributeDescriptor => {
                write!(
                    f,
                    "An attribute descriptor tried to define an attribute that \
                does not exist in vertex or doesn't match size."
                )
            }
            FrameworkError::InvalidFrameBuffer => {
                write!(f, "Framebuffer is invalid")
            }
            FrameworkError::FailedToConstructFBO => {
                write!(f, "OpenGL failed to construct framebuffer.")
            }
            FrameworkError::Custom(v) => {
                write!(f, "Custom error: {v}")
            }
            FrameworkError::GraphicsServerUnavailable => {
                write!(f, "Graphics server disconnected.")
            }
        }
    }
}

impl From<NulError> for FrameworkError {
    fn from(_: NulError) -> Self {
        Self::FaultyShaderSource
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<glutin::error::Error> for FrameworkError {
    fn from(err: glutin::error::Error) -> Self {
        Self::Custom(format!("{err:?}"))
    }
}

impl From<String> for FrameworkError {
    fn from(v: String) -> Self {
        Self::Custom(v)
    }
}

impl From<Box<dyn Error>> for FrameworkError {
    fn from(e: Box<dyn Error>) -> Self {
        Self::Custom(format!("{e:?}"))
    }
}
