//! A query object is used to fetch some data from rendering operations asynchronously. See
//! [`GpuQueryTrait`] docs for more info.

#![warn(missing_docs)]

use crate::define_shared_wrapper;
use fyrox_core::define_as_any_trait;
use std::fmt::Debug;

/// Kind of a GPU query.
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum QueryKind {
    /// Queries a number of rendered pixels; any pixel that passed all pipeline tests counts.
    SamplesPassed = glow::SAMPLES_PASSED,

    /// Queries a flag that defines whether the rendering operation produced any pixels or not.
    AnySamplesPassed = glow::ANY_SAMPLES_PASSED,
}

/// Result of a query.
#[derive(Debug)]
pub enum QueryResult {
    /// Number of rendered pixels; any pixel that passed all pipeline tests counts.
    SamplesPassed(u32),

    /// A flag that defines whether the rendering operation produced any pixels or not.
    AnySamplesPassed(bool),
}

define_as_any_trait!(GpuQueryAsAny => GpuQueryTrait);

/// A query object is used to fetch some data from rendering operations asynchronously. Usually it
/// is used to perform occlusion queries.
///
/// ## Examples
///
/// The following examples shows how to create a new GPU query, run it and fetch the result.
///
/// ```rust
/// use fyrox_graphics::{
///     error::FrameworkError,
///     query::{QueryKind, QueryResult},
///     server::GraphicsServer,
/// };
///
/// fn query(server: &dyn GraphicsServer) -> Result<(), FrameworkError> {
///     // Initialization.
///     let query = server.create_query()?;
///
///     // Somewhere in the rendering loop.
///     if !query.is_started() {
///         query.begin(QueryKind::AnySamplesPassed);
///
///         // Draw something.
///
///         query.end();
///     } else if let Some(QueryResult::AnySamplesPassed(any_samples_passed)) =
///         query.try_get_result()
///     {
///         println!("{any_samples_passed}");
///     }
///
///     Ok(())
/// }
/// ```
///
/// Keep in mind that you should always re-use the queries instead of creating them on the fly!
/// This is much more efficient, because it removes all redundant memory allocations and calls
/// to the GPU driver.
pub trait GpuQueryTrait: GpuQueryAsAny + Debug {
    /// Begins a query of the given kind. All drawing commands must be enclosed withing a pair of
    /// this method and [`Self::end`] calls. See [`QueryKind`] for more info.
    fn begin(&self, kind: QueryKind);

    /// Ends the query. Must be called after and in pair with [`Self::begin`].
    fn end(&self);

    /// Returns `true` if the query is started ([`Self::begin`] was called).
    fn is_started(&self) -> bool;

    /// Tries to fetch the query result without blocking. The query object guarantees that the
    /// result will be stored until the next call of [`Self::begin`], so consecutive calls of this
    /// method are allowed.
    fn try_get_result(&self) -> Option<QueryResult>;
}

define_shared_wrapper!(GpuQuery<dyn GpuQueryTrait>);
