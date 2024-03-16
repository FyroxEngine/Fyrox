//! Curve loader.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    },
    core::{uuid::Uuid, TypeUuidProvider},
    resource::curve::CurveResourceState,
};
use fyrox_resource::state::LoadError;
use std::{path::PathBuf, sync::Arc};

/// Default implementation for curve loading.
pub struct CurveLoader;

impl ResourceLoader for CurveLoader {
    fn extensions(&self) -> &[&str] {
        &["curve", "crv"]
    }

    fn data_type_uuid(&self) -> Uuid {
        CurveResourceState::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let curve_state = CurveResourceState::from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(curve_state))
        })
    }
}
