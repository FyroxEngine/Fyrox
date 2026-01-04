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

//! Contains all possible errors that may occur during script or plugin methods execution.

use crate::{
    asset::state::LoadError,
    core::{dyntype::DynTypeError, pool::PoolError, visitor::error::VisitError},
    scene::graph::GraphError,
};
use std::{
    backtrace::Backtrace,
    fmt::{Debug, Display, Formatter},
    sync::atomic::{AtomicBool, Ordering},
};

static CAPTURE_BACKTRACE: AtomicBool = AtomicBool::new(false);

/// Enables or disables backtrace capture when an error occurs. Backtrace capture is an expensive
/// operation, so it is disabled by default.
pub fn enable_backtrace_capture(capture: bool) {
    CAPTURE_BACKTRACE.store(capture, Ordering::Relaxed);
}

/// Returns `true` when the backtrace capture is one, `false` - otherwise.
pub fn is_capturing_backtrace() -> bool {
    CAPTURE_BACKTRACE.load(Ordering::Relaxed)
}

/// All possible errors that may occur during script or plugin methods execution.
pub enum GameErrorKind {
    /// A [`GraphError`] has occurred.
    GraphError(GraphError),
    /// A [`PoolError`] has occurred.
    PoolError(PoolError),
    /// An arbitrary, user-defined error has occurred.
    UserError(UserError),
    /// A [`VisitError`] has occurred.
    VisitError(VisitError),
    /// A [`LoadError`] has occurred.
    ResourceLoadError(LoadError),
    /// A [`DynTypeError`] has occurred.
    DynTypeError(DynTypeError),
    /// Arbitrary error message.
    StringError(String),
}

/// An error that may occur during game code execution.
pub struct GameError {
    kind: GameErrorKind,
    trace: Option<Backtrace>,
}

impl GameError {
    /// Creates a new error from the specified kind.
    pub fn new(kind: GameErrorKind) -> Self {
        Self {
            kind,
            trace: if is_capturing_backtrace() {
                Some(Backtrace::force_capture())
            } else {
                None
            },
        }
    }

    /// A shortcut `GameError::user(value)` for `GameError::UserError(Box::new(value))`.
    pub fn user(value: impl std::error::Error + 'static) -> Self {
        Self::new(GameErrorKind::UserError(Box::new(value)))
    }

    /// A shortcut for [`Self::StringError`]
    pub fn str(value: impl AsRef<str>) -> Self {
        Self::new(GameErrorKind::StringError(value.as_ref().to_string()))
    }
}

impl From<GraphError> for GameError {
    fn from(value: GraphError) -> Self {
        Self::new(GameErrorKind::GraphError(value))
    }
}

impl From<PoolError> for GameError {
    fn from(value: PoolError) -> Self {
        Self::new(GameErrorKind::PoolError(value))
    }
}

impl From<UserError> for GameError {
    fn from(value: UserError) -> Self {
        Self::new(GameErrorKind::UserError(value))
    }
}

impl From<LoadError> for GameError {
    fn from(value: LoadError) -> Self {
        Self::new(GameErrorKind::ResourceLoadError(value))
    }
}

impl From<VisitError> for GameError {
    fn from(value: VisitError) -> Self {
        Self::new(GameErrorKind::VisitError(value))
    }
}

impl From<DynTypeError> for GameError {
    fn from(value: DynTypeError) -> Self {
        Self::new(GameErrorKind::DynTypeError(value))
    }
}

impl std::error::Error for GameError {}

impl Display for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.trace.as_ref() {
            Some(trace) => {
                write!(f, "{}\nBacktrace:\n{}", self.kind, trace)
            }
            None => {
                write!(
                    f,
                    "{}\nBacktrace is unavailable, call `enable_backtrace_capture(true)` to \
                 enable backtrace capture. Keep in mind that it may be very slow!",
                    self.kind
                )
            }
        }
    }
}

impl Display for GameErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GraphError(err) => Display::fmt(&err, f),
            Self::PoolError(err) => Display::fmt(&err, f),
            Self::UserError(err) => Display::fmt(&err, f),
            Self::ResourceLoadError(err) => Display::fmt(&err, f),
            Self::VisitError(err) => Display::fmt(&err, f),
            Self::DynTypeError(err) => Display::fmt(&err, f),
            Self::StringError(msg) => {
                write!(f, "{msg}")
            }
        }
    }
}

impl Debug for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// An alias for [`Result`] that has `()` as `Ok` value, and [`GameError`] as error value.
pub type GameResult = Result<(), GameError>;

/// An arbitrary, user-defined, boxed error type.
pub type UserError = Box<dyn std::error::Error>;
