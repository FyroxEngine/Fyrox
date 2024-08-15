use crate::renderer::framework::{error::FrameworkError, state::PipelineState};
use glow::HasContext;
use std::{cell::Cell, rc::Weak};

#[repr(u32)]
#[derive(Copy, Clone)]
pub enum QueryKind {
    SamplesPassed = glow::SAMPLES_PASSED,
    AnySamplesPassed = glow::ANY_SAMPLES_PASSED,
}

pub enum QueryResult {
    SamplesPassed(u32),
    AnySamplesPassed(bool),
}

pub struct Query {
    id: glow::Query,
    pipeline_state: Weak<PipelineState>,
    active_query: Cell<Option<QueryKind>>,
}

impl Query {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            id: unsafe { state.gl.create_query()? },
            pipeline_state: state.weak(),
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
        unsafe {
            if let Some(pipeline_state) = self.pipeline_state.upgrade() {
                pipeline_state.gl.delete_query(self.id);
            }
        }
    }
}
