//! All possible errors that can happen in the engine.

use crate::{renderer::framework::error::FrameworkError, scene::sound::SoundError};

/// See module docs.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Sound system error.
    #[error(transparent)]
    Sound(SoundError),
    /// Rendering system error.
    #[error(transparent)]
    Renderer(FrameworkError),
    /// Internal error.
    #[error("Custom error: {0}")]
    Custom(String),
}

impl From<SoundError> for EngineError {
    fn from(sound: SoundError) -> Self {
        Self::Sound(sound)
    }
}

impl From<FrameworkError> for EngineError {
    fn from(renderer: FrameworkError) -> Self {
        Self::Renderer(renderer)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<glutin::CreationError> for EngineError {
    fn from(e: glutin::CreationError) -> Self {
        Self::Custom(format!("{:?}", e))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<glutin::ContextError> for EngineError {
    fn from(e: glutin::ContextError) -> Self {
        Self::Custom(format!("{:?}", e))
    }
}
