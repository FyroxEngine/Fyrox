//! Animation blending state machine loader.

use crate::{
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
    },
    resource::absm::{AbsmImportOptions, AbsmResource, AbsmResourceState},
    utils::log::Log,
};

/// Default implementation for ABSM loading.
pub struct AbsmLoader;

impl ResourceLoader<AbsmResource, AbsmImportOptions> for AbsmLoader {
    fn load(
        &self,
        absm: AbsmResource,
        _default_import_options: AbsmImportOptions,
        event_broadcaster: ResourceEventBroadcaster<AbsmResource>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = absm.state().path().to_path_buf();

            match AbsmResourceState::from_file(&path).await {
                Ok(absm_state) => {
                    Log::info(format!("ABSM {:?} is loaded!", path));

                    absm.state().commit_ok(absm_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(absm, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load ABSM from {:?}! Reason {:?}",
                        path, error
                    ));

                    absm.state().commit_error(path, error);
                }
            }
        })
    }
}
