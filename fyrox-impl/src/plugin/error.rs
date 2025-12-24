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
    core::{pool::PoolError, visitor::error::VisitError},
    scene::graph::GraphError,
};
use std::fmt::{Debug, Display, Formatter};

/// All possible errors that may occur during script or plugin methods execution.
pub enum GameError {
    /// A [`GraphError`] has occurred.
    GraphError(GraphError),
    /// A [`PoolError`] has occurred.
    PoolError(PoolError),
    /// An arbitrary, user-defined error has occurred.
    AnyError(AnyScriptError),
    /// A [`VisitError`] has occurred.
    VisitError(VisitError),
    /// A [`LoadError`] has occurred.
    ResourceLoadError(LoadError),
}

impl From<GraphError> for GameError {
    fn from(value: GraphError) -> Self {
        Self::GraphError(value)
    }
}

impl From<PoolError> for GameError {
    fn from(value: PoolError) -> Self {
        Self::PoolError(value)
    }
}

impl From<AnyScriptError> for GameError {
    fn from(value: AnyScriptError) -> Self {
        Self::AnyError(value)
    }
}

impl From<LoadError> for GameError {
    fn from(value: LoadError) -> Self {
        Self::ResourceLoadError(value)
    }
}

impl From<VisitError> for GameError {
    fn from(value: VisitError) -> Self {
        Self::VisitError(value)
    }
}

impl std::error::Error for GameError {}

impl Display for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::GraphError(err) => Display::fmt(&err, f),
            GameError::PoolError(err) => Display::fmt(&err, f),
            GameError::AnyError(err) => Display::fmt(&err, f),
            GameError::ResourceLoadError(err) => Display::fmt(&err, f),
            GameError::VisitError(err) => Display::fmt(&err, f),
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
pub type AnyScriptError = Box<dyn std::error::Error>;
