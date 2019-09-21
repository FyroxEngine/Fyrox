use std::ffi::NulError;
use glutin::{CreationError, ContextError};

#[derive(Debug)]
pub enum RendererError {
    ShaderCompilationFailed {
        shader_name: String,
        error_message: String,
    },
    ShaderLinkingFailed {
        shader_name: String,
        error_message: String,
    },
    FaultyShaderSource,
    UnableToFindShaderUniform(String),
    InternalError(String),
    ContextError(String),
    InvalidTextureData,
    InvalidTriangleRange {
        start: usize,
        end: usize,
        total: usize,
    },

    /// Means that attrubute descriptor tries to define an attribute that does not exists in vertex,
    /// or it does not match size. For example you have vertex:
    ///   pos: float2,
    ///   normal: float3
    /// But you described second attribute as Float4, then you'll get this error.
    InvalidAttributeDescriptor,
}

impl From<NulError> for RendererError {
    fn from(_: NulError) -> Self {
        RendererError::FaultyShaderSource
    }
}

impl From<CreationError> for RendererError {
    fn from(e: CreationError) -> Self {
        RendererError::InternalError(format!("{:?}", e))
    }
}

impl From<ContextError> for RendererError {
    fn from(e: ContextError) -> Self {
        RendererError::ContextError(format!("{:?}", e))
    }
}