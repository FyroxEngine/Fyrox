//! Curve loader.

use crate::{
    asset::loader::{BoxedLoaderFuture, ResourceLoader},
    resource::curve::CurveResourceState,
    utils::log::Log,
};
use fyrox_resource::event::ResourceEventBroadcaster;
use fyrox_resource::untyped::UntypedResource;
use std::any::Any;

/// Default implementation for curve loading.
pub struct CurveLoader;

impl ResourceLoader for CurveLoader {
    fn extensions(&self) -> &[&str] {
        &["curve"]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn load(
        &self,
        curve: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = curve.0.lock().path().to_path_buf();

            match CurveResourceState::from_file(&path).await {
                Ok(curve_state) => {
                    Log::info(format!("Curve {:?} is loaded!", path));

                    curve.0.lock().commit_ok(curve_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(curve, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load curve from {:?}! Reason {:?}",
                        path, error
                    ));

                    curve.0.lock().commit_error(path, error);
                }
            }
        })
    }
}
