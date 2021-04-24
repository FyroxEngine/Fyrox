//! Contains all possible errors that may occur during rendering, initialization of
//! renderer structures, or GAPI.

use std::ffi::NulError;

/// Set of possible renderer errors.
#[derive(Debug)]
pub enum RendererError {
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
}

impl From<NulError> for RendererError {
    fn from(_: NulError) -> Self {
        Self::FaultyShaderSource
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<glutin::ContextError> for RendererError {
    fn from(err: glutin::ContextError) -> Self {
        Self::Custom(format!("{:?}", err))
    }
}

impl From<String> for RendererError {
    fn from(v: String) -> Self {
        Self::Custom(v)
    }
}
