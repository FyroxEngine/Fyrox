use crate::{renderer::error::RendererError, sound::error::SoundError};
use glutin::{ContextError, CreationError};

#[derive(Debug)]
pub enum EngineError {
    Sound(SoundError),
    Renderer(RendererError),
    InternalError(String),
    ContextError(String),
}

impl From<SoundError> for EngineError {
    fn from(sound: SoundError) -> Self {
        EngineError::Sound(sound)
    }
}

impl From<RendererError> for EngineError {
    fn from(renderer: RendererError) -> Self {
        EngineError::Renderer(renderer)
    }
}

impl From<CreationError> for EngineError {
    fn from(e: CreationError) -> Self {
        EngineError::InternalError(format!("{:?}", e))
    }
}

impl From<ContextError> for EngineError {
    fn from(e: ContextError) -> Self {
        EngineError::ContextError(format!("{:?}", e))
    }
}
