use crate::{error::FrameworkError, state::GlGraphicsServer};
use glow::HasContext;
use std::{cell::Cell, rc::Weak};

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum QueryKind {
    SamplesPassed = glow::SAMPLES_PASSED,
    AnySamplesPassed = glow::ANY_SAMPLES_PASSED,
}

#[derive(Debug)]
pub enum QueryResult {
    SamplesPassed(u32),
    AnySamplesPassed(bool),
}

#[derive(Debug)]
pub struct Query {
    id: glow::Query,
    pipeline_state: Weak<GlGraphicsServer>,
    active_query: Cell<Option<QueryKind>>,
}

impl Query {
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

    pub fn begin(&self, kind: QueryKind) {
        if let Some(pipeline_state) = self.pipeline_state.upgrade() {
            unsafe {
                pipeline_state.gl.begin_query(kind as u32, self.id);
            }

            self.active_query.set(Some(kind));
        }
    }

    pub fn end(&self) {
        if let Some(active_query) = self.active_query.get() {
            if let Some(pipeline_state) = self.pipeline_state.upgrade() {
                unsafe {
                    pipeline_state.gl.end_query(active_query as u32);
                }
            }
        }
    }

    pub fn try_get_result(&self) -> Option<QueryResult> {
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

impl Drop for Query {
    fn drop(&mut self) {
        if let Some(pipeline_state) = self.pipeline_state.upgrade() {
            pipeline_state.state.borrow_mut().queries.push(self.id);
        }
    }
}
