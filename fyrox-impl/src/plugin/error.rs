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

use crate::scene::graph::GraphError;
use fyrox_core::pool::PoolError;
use fyrox_resource::state::LoadError;
use std::fmt::{Debug, Display, Formatter};

pub enum GameError {
    GraphError(GraphError),
    PoolError(PoolError),
    AnyError(AnyScriptError),
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

impl std::error::Error for GameError {}

impl Display for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::GraphError(err) => {
                write!(f, "{}", err)
            }
            GameError::PoolError(err) => {
                write!(f, "{}", err)
            }
            GameError::AnyError(err) => {
                write!(f, "{}", err)
            }
            GameError::ResourceLoadError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}

impl Debug for GameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

pub type GameResult = Result<(), GameError>;
pub type AnyScriptError = Box<dyn std::error::Error>;
