//! Curve resource holds a [`Curve`]

use crate::{
    asset::{define_new_resource, Resource, ResourceData},
    core::reflect::prelude::*,
    core::{curve::Curve, io::FileLoadError, visitor::prelude::*},
    engine::resource_manager::options::ImportOptions,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// An error that may occur during curve resource loading.
#[derive(Debug, thiserror::Error)]
pub enum CurveResourceError {
    /// An i/o error has occurred.
    #[error("A file load error has occurred {0:?}")]
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    #[error("An error that may occur due to version incompatibilities. {0:?}")]
    Visit(VisitError),
}

impl From<FileLoadError> for CurveResourceError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for CurveResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

/// State of the [`CurveResource`]
#[derive(Debug, Visit, Default)]
pub struct CurveResourceState {
    pub(crate) path: PathBuf,
    /// Actual curve.
    pub curve: Curve,
}

impl ResourceData for CurveResourceState {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl CurveResourceState {
    /// Load a curve resource from the specific file path.
    pub async fn from_file(path: &Path) -> Result<Self, CurveResourceError> {
        let mut visitor = Visitor::load_binary(path).await?;
        let mut curve = Curve::default();
        curve.visit("Curve", &mut visitor)?;
        Ok(Self {
            curve,
            path: path.to_path_buf(),
        })
    }
}

define_new_resource!(
    /// See module docs.
    #[derive(Reflect)]
    #[reflect(hide_all)]
    CurveResource<CurveResourceState, CurveResourceError>
);

/// Import options for curve resource.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CurveImportOptions {}

impl ImportOptions for CurveImportOptions {}
