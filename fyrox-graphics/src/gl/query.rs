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

use crate::{
    error::FrameworkError,
    gl::server::GlGraphicsServer,
    query::{GpuQueryTrait, QueryKind, QueryResult},
};
use glow::HasContext;
use std::{cell::Cell, rc::Weak};

#[derive(Debug)]
pub struct GlQuery {
    id: glow::Query,
    pipeline_state: Weak<GlGraphicsServer>,
    active_query: Cell<Option<QueryKind>>,
}

impl GlQuery {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let mut inner = server.state.borrow_mut();
        let id = if let Some(existing) = inner.queries.pop() {
            existing
        } else {
            unsafe { server.gl.create_query()? }
        };
        Ok(Self {
            id,
            pipeline_state: server.weak(),
            active_query: Default::default(),
        })
    }
}

impl GpuQueryTrait for GlQuery {
    fn begin(&self, kind: QueryKind) {
        if let Some(pipeline_state) = self.pipeline_state.upgrade() {
            unsafe {
                pipeline_state.gl.begin_query(kind as u32, self.id);
            }

            self.active_query.set(Some(kind));
        }
    }

    fn end(&self) {
        if let Some(active_query) = self.active_query.get() {
            if let Some(pipeline_state) = self.pipeline_state.upgrade() {
                unsafe {
                    pipeline_state.gl.end_query(active_query as u32);
                }
            }
        }
    }

    fn is_started(&self) -> bool {
        self.active_query.get().is_some()
    }

    fn try_get_result(&self) -> Option<QueryResult> {
        let pipeline_state = self.pipeline_state.upgrade()?;
        let active_query = self.active_query.get()?;
        unsafe {
            let is_ready = pipeline_state
                .gl
                .get_query_parameter_u32(self.id, glow::QUERY_RESULT_AVAILABLE)
                > 0;
            if is_ready {
                let query_result = pipeline_state
                    .gl
                    .get_query_parameter_u32(self.id, glow::QUERY_RESULT);
                match active_query {
                    QueryKind::SamplesPassed => Some(QueryResult::SamplesPassed(query_result)),
                    QueryKind::AnySamplesPassed => {
                        Some(QueryResult::AnySamplesPassed(query_result > 0))
                    }
                }
            } else {
                None
            }
        }
    }
}

impl Drop for GlQuery {
    fn drop(&mut self) {
        if let Some(pipeline_state) = self.pipeline_state.upgrade() {
            pipeline_state.state.borrow_mut().queries.push(self.id);
        }
    }
}
