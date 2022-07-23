//! Sound buffer loader.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        reflect::Reflect,
    },
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::{try_get_import_settings, ImportOptions},
    },
    utils::log::Log,
};
use fyrox_sound::buffer::{
    DataSource, SoundBufferResource, SoundBufferResourceLoadError, SoundBufferState,
};
use serde::{Deserialize, Serialize};

/// Defines sound buffer resource import options.
#[derive(Clone, Deserialize, Serialize, Default, Inspect, Reflect)]
pub struct SoundBufferImportOptions {
    /// Whether the buffer is streaming or not.
    pub stream: bool,
}

impl ImportOptions for SoundBufferImportOptions {}

/// Default implementation for sound buffer loading.
pub struct SoundBufferLoader;

impl ResourceLoader<SoundBufferResource, SoundBufferImportOptions> for SoundBufferLoader {
    fn load(
        &self,
        resource: SoundBufferResource,
        default_import_options: SoundBufferImportOptions,
        event_broadcaster: ResourceEventBroadcaster<SoundBufferResource>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = resource.state().path().to_path_buf();

            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            match DataSource::from_file(&path).await {
                Ok(source) => {
                    let buffer = if import_options.stream {
                        SoundBufferState::raw_streaming(source)
                    } else {
                        SoundBufferState::raw_generic(source)
                    };
                    match buffer {
                        Ok(sound_buffer) => {
                            resource.state().commit_ok(sound_buffer);

                            event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);

                            Log::info(format!("Sound buffer {:?} is loaded!", path));
                        }
                        Err(_) => {
                            resource.state().commit_error(
                                path.clone(),
                                SoundBufferResourceLoadError::UnsupportedFormat,
                            );

                            Log::err(format!("Unable to load sound buffer from {:?}!", path));
                        }
                    }
                }
                Err(e) => {
                    Log::err(format!("Invalid data source for sound buffer: {:?}", e));

                    resource
                        .state()
                        .commit_error(path.clone(), SoundBufferResourceLoadError::Io(e));
                }
            }
        })
    }
}
