//! All possible errors that can happen in the engine.

use crate::{renderer::error::RendererError, sound::error::SoundError};

/// See module docs.
#[derive(Debug)]
pub enum EngineError {
    /// Sound system error.
    Sound(SoundError),
    /// Rendering system error.
    Renderer(RendererError),
    /// Internal error.
    Custom(String),
}

impl From<SoundError> for EngineError {
    fn from(sound: SoundError) -> Self {
        Self::Sound(sound)
    }
}

impl From<RendererError> for EngineError {
    fn from(renderer: RendererError) -> Self {
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
