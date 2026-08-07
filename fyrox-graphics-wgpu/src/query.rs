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

use crate::server::WgpuGraphicsServer;
use fyrox_graphics::{
    error::FrameworkError,
    query::{GpuQueryTrait, QueryKind, QueryResult},
};
use std::cell::Cell;
use std::fmt::Debug;
use std::rc::Weak;

/// Wgpu implementation of [`GpuQueryTrait`](fyrox_graphics::query::GpuQueryTrait).
///
/// **Stub implementation**: wgpu does not currently expose occlusion queries,
/// so this type tracks the active state but always returns `u32::MAX` from
/// [`try_get_result`](Self::try_get_result). This is sufficient for the engine's
/// query-based culling to work (everything is considered visible).
pub struct WgpuQuery {
    _server: Weak<WgpuGraphicsServer>,
    active: Cell<bool>,
}

impl Debug for WgpuQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuQuery")
            .field("active", &self.active.get())
            .finish()
    }
}

impl WgpuQuery {
    /// Creates a new stub query.
    pub fn new(server: &WgpuGraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            _server: server.weak_ref(),
            active: Cell::new(false),
        })
    }
}

impl GpuQueryTrait for WgpuQuery {
    fn begin(&self, _kind: QueryKind) {
        self.active.set(true);
    }
    fn end(&self) {
        self.active.set(false);
    }
    fn is_started(&self) -> bool {
        self.active.get()
    }
    /// Always returns `Some(QueryResult::SamplesPassed(u32::MAX))` — see the
    /// struct-level documentation for details.
    fn try_get_result(&self) -> Option<QueryResult> {
        Some(QueryResult::SamplesPassed(u32::MAX))
    }
}
