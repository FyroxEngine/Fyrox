use std::{any::Any, fmt::Debug};

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

pub trait Query: Any + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn begin(&self, kind: QueryKind);
    fn end(&self);
    fn try_get_result(&self) -> Option<QueryResult>;
}
