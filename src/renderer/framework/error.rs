//! Contains all possible errors that may occur during rendering, initialization of
//! renderer structures, or GAPI.

use std::ffi::NulError;

/// Set of possible renderer errors.
#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    #[error(
        "Compilation of \"{}\" shader has failed: {}",
        shader_name,
        error_message
    )]
    /// Compilation of a shader has failed.
    ShaderCompilationFailed {
        /// Name of shader.
        shader_name: String,
        /// Compilation error message.
        error_message: String,
    },
    /// Means that shader link stage failed, exact reason is inside `error_message`
    #[error("Linking shader \"{}\" failed: {}", shader_name, error_message)]
    ShaderLinkingFailed {
        /// Name of shader.
        shader_name: String,
        /// Linking error message.
        error_message: String,
    },
    /// Shader source contains invalid characters.
    #[error("Shader source contains invalid characters")]
    FaultyShaderSource,
    /// There is no such shader uniform (could be optimized out).
    #[error("There is no such shader uniform: {0}")]
    UnableToFindShaderUniform(String),
    /// Texture has invalid data - insufficient size.
    #[error(
        "Texture has invalid data (insufficent size): expected {}, actual: {}",
        expected_data_size,
        actual_data_size
    )]
    InvalidTextureData {
        /// Expected data size in bytes.
        expected_data_size: usize,
        /// Actual data size in bytes.
        actual_data_size: usize,
    },
    /// None variant was passed as texture data, but engine does not support it.
    #[error("None variant was passed as texture data, but engine does not support it.")]
    EmptyTextureData,
    /// Means that you tried to draw element range from GeometryBuffer that
    /// does not have enough elements.
    #[error(
        "Tried to draw element from GeometryBuffer that does not have enough elements:
        start: {},
        end: {},
        total: {}
        ",
        start,
        end,
        total
    )]
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
    #[error("An attribute descriptor tried to define an attribute that does not exist in vertex or doesn't match size.")]
    InvalidAttributeDescriptor,
    /// Framebuffer is invalid.
    #[error("Framebuffer is invalid")]
    InvalidFrameBuffer,
    /// OpenGL failed to construct framebuffer.
    #[error("OpenGL failed to construct framebuffer.")]
    FailedToConstructFBO,
    /// Custom error. Usually used for internal errors.
    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<NulError> for FrameworkError {
    fn from(_: NulError) -> Self {
        Self::FaultyShaderSource
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<glutin::ContextError> for FrameworkError {
    fn from(err: glutin::ContextError) -> Self {
        Self::Custom(format!("{:?}", err))
    }
}

impl From<String> for FrameworkError {
    fn from(v: String) -> Self {
        Self::Custom(v)
    }
}
