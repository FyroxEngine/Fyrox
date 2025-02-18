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

//! Contains all possible errors that can occur in the engine.

use std::fmt::{Display, Error, Formatter};

/// Decoder specific error.
#[derive(Debug)]
pub enum DecoderError {
    /// Error coming from Symphonia
    SymphoniaError(symphonia::core::errors::Error),
}

/// Generic error enumeration for each error in this engine.
#[derive(Debug)]
pub enum SoundError {
    /// Generic input error.
    Io(std::io::Error),

    /// No backend is provided for current OS.
    NoBackend,

    /// Unable to initialize device, exact reason stored in inner value.
    FailedToInitializeDevice(String),

    /// Invalid header of sound file.
    InvalidHeader,

    /// Unsupported format of sound file.
    UnsupportedFormat,

    /// It means that some thread panicked while holding a MutexGuard, the data mutex
    /// protected can be corrupted.
    PoisonedMutex,

    /// An error occurred during math calculations, i.e. there was an attempt to
    /// normalize a vector with length `|v| == 0.0`.
    MathError(String),

    /// You tried to create a source with streaming buffer that is currently being
    /// used by some other source. This is wrong because only one source can play
    /// sound from streaming buffer.
    StreamingBufferAlreadyInUse,

    /// Decoder specific error, can occur in the decoder by any reason (invalid format,
    /// insufficient data, etc.). Exact reason stored in inner value.
    DecoderError(DecoderError),

    /// A buffer is invalid (for example it is LoadError state)
    BufferFailedToLoad,

    /// A buffer is not loaded yet, consider to `await` it before use.
    BufferIsNotLoaded,
}

impl From<std::io::Error> for SoundError {
    fn from(e: std::io::Error) -> Self {
        SoundError::Io(e)
    }
}

impl<'a, T> From<std::sync::PoisonError<std::sync::MutexGuard<'a, T>>> for SoundError {
    fn from(_: std::sync::PoisonError<std::sync::MutexGuard<'a, T>>) -> Self {
        SoundError::PoisonedMutex
    }
}

impl From<symphonia::core::errors::Error> for SoundError {
    fn from(e: symphonia::core::errors::Error) -> Self {
        SoundError::DecoderError(DecoderError::SymphoniaError(e))
    }
}

impl Display for SoundError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            SoundError::Io(io) => write!(f, "io error: {io}"),
            SoundError::NoBackend => write!(f, "no backend implemented for current platform"),
            SoundError::FailedToInitializeDevice(reason) => {
                write!(f, "failed to initialize device. reason: {reason}")
            }
            SoundError::InvalidHeader => write!(f, "invalid header of sound file"),
            SoundError::UnsupportedFormat => write!(f, "unsupported format of sound file"),
            SoundError::PoisonedMutex => write!(f, "attempt to use poisoned mutex"),
            SoundError::MathError(reason) => {
                write!(f, "math error has occurred. reason: {reason}")
            }
            SoundError::StreamingBufferAlreadyInUse => {
                write!(f, "streaming buffer in already in use")
            }
            SoundError::DecoderError(de) => write!(f, "internal decoder error: {de:?}"),
            SoundError::BufferFailedToLoad => write!(f, "a buffer failed to load"),
            SoundError::BufferIsNotLoaded => write!(f, "a buffer is not loaded yet"),
        }
    }
}

impl std::error::Error for SoundError {}
