//! Curve resource holds a [`Curve`]

use crate::{
    asset::{io::ResourceIo, Resource, ResourceData, CURVE_RESOURCE_UUID},
    core::{
        io::FileLoadError, math::curve::Curve, reflect::prelude::*, uuid::Uuid,
        visitor::prelude::*, TypeUuidProvider,
    },
};
use std::error::Error;
use std::{
    any::Any,
    fmt::{Display, Formatter},
    path::Path,
};

pub mod loader;

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
#[derive(Debug, Visit, Default, Reflect)]
pub struct CurveResourceState {
    /// Actual curve.
    pub curve: Curve,
}

impl ResourceData for CurveResourceState {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
        // TODO: Add saving.
        Err("Saving is not supported!".to_string().into())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}

impl TypeUuidProvider for CurveResourceState {
    fn type_uuid() -> Uuid {
        CURVE_RESOURCE_UUID
    }
}

impl CurveResourceState {
    /// Load a curve resource from the specific file path.
    pub async fn from_file(path: &Path, io: &dyn ResourceIo) -> Result<Self, CurveResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        let mut curve = Curve::default();
        curve.visit("Curve", &mut visitor)?;
        Ok(Self { curve })
    }
}

/// Type alias for curve resources.
pub type CurveResource = Resource<CurveResourceState>;
