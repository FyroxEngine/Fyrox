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