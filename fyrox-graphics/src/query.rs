use crate::core::Downcast;
use std::fmt::Debug;

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

pub trait Query: Downcast + Debug {
    fn begin(&self, kind: QueryKind);
    fn end(&self);
    fn try_get_result(&self) -> Option<QueryResult>;
}
