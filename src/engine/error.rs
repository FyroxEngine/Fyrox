//! All possible errors that can happen in the engine.

use crate::{renderer::framework::error::FrameworkError, scene::sound::SoundError};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

/// See module docs.
#[derive(Debug)]
pub enum EngineError {
    /// Sound system error.
    Sound(SoundError),
    /// Rendering system error.
    Renderer(FrameworkError),
    /// Internal error.
    Custom(String),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Sound(v) => Display::fmt(v, f),
            EngineError::Renderer(v) => Display::fmt(v, f),
            EngineError::Custom(v) => {
                write!(f, "Custom error: {v}")
            }
        }
    }
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
impl From<glutin::error::Error> for EngineError {
    fn from(e: glutin::error::Error) -> Self {
        Self::Custom(format!("{:?}", e))
    }
}

impl From<Box<dyn Error>> for EngineError {
    fn from(e: Box<dyn Error>) -> Self {
        Self::Custom(format!("{:?}", e))
    }
}
