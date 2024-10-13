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

impl From<Box<dyn Error>> for EngineError {
    fn from(e: Box<dyn Error>) -> Self {
        Self::Custom(format!("{e:?}"))
    }
}
