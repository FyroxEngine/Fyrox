//! Animation loader.

use crate::resource::animation::{
    AnimationImportOptions, AnimationResource, AnimationResourceState,
};
use crate::{
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
    },
    utils::log::Log,
};

/// Default implementation for animation loading.
pub struct AnimationLoader;

impl ResourceLoader<AnimationResource, AnimationImportOptions> for AnimationLoader {
    fn load(
        &self,
        animation: AnimationResource,
        _default_import_options: AnimationImportOptions,
        event_broadcaster: ResourceEventBroadcaster<AnimationResource>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = animation.state().path().to_path_buf();

            match AnimationResourceState::from_file(&path).await {
                Ok(animation_state) => {
                    Log::info(format!("Animation {:?} is loaded!", path));

                    animation.state().commit_ok(animation_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(animation, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load animation from {:?}! Reason {:?}",
                        path, error
                    ));

                    animation.state().commit_error(path, error);
                }
            }
        })
    }
}
