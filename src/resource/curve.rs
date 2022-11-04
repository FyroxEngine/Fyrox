//! Curve resource holds a [`Curve`]

use crate::{
    asset::{define_new_resource, Resource, ResourceData},
    core::reflect::prelude::*,
    core::{curve::Curve, io::FileLoadError, visitor::prelude::*},
    engine::resource_manager::options::ImportOptions,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// An error that may occur during curve resource loading.
#[derive(Debug)]
pub enum CurveResourceError {
    /// An i/o error has occurred.
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),
}

impl Display for CurveResourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CurveResourceError::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            CurveResourceError::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
        }
    }
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
