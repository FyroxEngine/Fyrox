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

use crate::core::{parking_lot::Mutex, SafeLock};
use std::{
    fmt::{Debug, Formatter},
    path::Path,
    sync::Arc,
};

pub type FilterCallback = dyn FnMut(&Path) -> bool + Send;

#[derive(Clone, Default)]
pub enum PathFilter {
    #[default]
    AllPass,
    Callback(Arc<Mutex<FilterCallback>>),
}

impl PathFilter {
    #[inline]
    pub fn all_pass() -> Self {
        Self::AllPass
    }

    #[inline]
    pub fn new<F: FnMut(&Path) -> bool + 'static + Send>(filter: F) -> Self {
        Self::Callback(Arc::new(Mutex::new(filter)))
    }

    #[inline]
    pub fn folder() -> Self {
        Self::new(|p: &Path| p.is_dir())
    }

    #[inline]
    pub fn passes(&self, path: impl AsRef<Path>) -> bool {
        match self {
            PathFilter::AllPass => true,
            PathFilter::Callback(callback) => callback.safe_lock()(path.as_ref()),
        }
    }
}

impl PartialEq for PathFilter {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::AllPass, Self::AllPass) => true,
            (Self::Callback(a), Self::Callback(b)) => std::ptr::eq(&**a, &**b),
            _ => false,
        }
    }
}

impl Debug for PathFilter {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Filter")
    }
}
