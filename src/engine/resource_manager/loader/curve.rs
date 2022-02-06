use crate::{
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        ResourceManager
    },
    resource::curve::{CurveImportOptions, CurveResource, CurveResourceState},
    utils::log::Log,
};

pub struct CurveLoader;

impl ResourceLoader<CurveResource, CurveImportOptions> for CurveLoader {
    fn load(
        &self,
        curve: CurveResource,
        _default_import_options: CurveImportOptions,
        _resource_manager: ResourceManager,
        event_broadcaster: ResourceEventBroadcaster<CurveResource>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = curve.state().path().to_path_buf();

            match CurveResourceState::from_file(&path).await {
                Ok(curve_state) => {
                    Log::info(format!("Curve {:?} is loaded!", path));

                    curve.state().commit_ok(curve_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(curve, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load curve from {:?}! Reason {:?}",
                        path, error
                    ));

                    curve.state().commit_error(path, error);
                }
            }
        })
    }
}
