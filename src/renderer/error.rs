use std::ffi::NulError;

#[derive(Debug)]
pub enum RendererError {
    ShaderCompilationFailed {
        shader_name: String,
        error_message: String,
    },

    /// Means that shader link stage failed, exact reason is inside `error_message`
    ShaderLinkingFailed {
        shader_name: String,
        error_message: String,
    },
    FaultyShaderSource,
    UnableToFindShaderUniform(String),
    InvalidTextureData,

    /// Means that you tried to draw element range from GeometryBuffer that
    /// does not have enough elements.
    InvalidElementRange {
        start: usize,
        end: usize,
        total: usize,
    },

    /// Means that attribute descriptor tries to define an attribute that does
    /// not exists in vertex, or it does not match size. For example you have vertex:
    ///   pos: float2,
    ///   normal: float3
    /// But you described second attribute as Float4, then you'll get this error.
    InvalidAttributeDescriptor,

    InvalidFrameBuffer,
}

impl From<NulError> for RendererError {
    fn from(_: NulError) -> Self {
        RendererError::FaultyShaderSource
    }
}

