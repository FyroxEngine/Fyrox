//! Curve loader.

use crate::{
    asset::{
        event::ResourceEventBroadcaster,
        io::ResourceIo,
        loader::{BoxedLoaderFuture, ResourceLoader},
        untyped::UntypedResource,
    },
    core::{log::Log, uuid::Uuid, TypeUuidProvider},
    resource::curve::CurveResourceState,
};
use std::sync::Arc;

/// Default implementation for curve loading.
pub struct CurveLoader;

impl ResourceLoader for CurveLoader {
    fn extensions(&self) -> &[&str] {
        &["curve", "crv"]
    }

    fn data_type_uuid(&self) -> Uuid {
        CurveResourceState::type_uuid()
    }

    fn load(
        &self,
        curve: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = curve.path();
            match CurveResourceState::from_file(&path, io.as_ref()).await {
                Ok(curve_state) => {
                    Log::info(format!("Curve {:?} is loaded!", path));

                    curve.commit_ok(curve_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(curve, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load curve from {:?}! Reason {:?}",
                        path, error
                    ));

                    curve.commit_error(error);
                }
            }
        })
    }
}
