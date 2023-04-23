//! Sound buffer loader.

use crate::{
    asset::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::{try_get_import_settings, ImportOptions},
    },
    core::reflect::prelude::*,
    utils::log::Log,
};
use fyrox_resource::untyped::UntypedResource;
use fyrox_sound::buffer::{DataSource, SoundBuffer, SoundBufferResourceLoadError};
use serde::{Deserialize, Serialize};

/// Defines sound buffer resource import options.
#[derive(Clone, Deserialize, Serialize, Default, Debug, Reflect)]
pub struct SoundBufferImportOptions {
    /// Whether the buffer is streaming or not.
    pub stream: bool,
}

impl ImportOptions for SoundBufferImportOptions {}

/// Default implementation for sound buffer loading.
pub struct SoundBufferLoader {
    pub default_import_options: SoundBufferImportOptions,
}

impl ResourceLoader for SoundBufferLoader {
    fn load(
        &self,
        resource: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let path = resource.path().to_path_buf();

            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            match DataSource::from_file(&path).await {
                Ok(source) => {
                    let buffer = if import_options.stream {
                        SoundBuffer::raw_streaming(source)
                    } else {
                        SoundBuffer::raw_generic(source)
                    };
                    match buffer {
                        Ok(sound_buffer) => {
                            resource.0.lock().commit_ok(sound_buffer);

                            event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);

                            Log::info(format!("Sound buffer {:?} is loaded!", path));
                        }
                        Err(_) => {
                            resource.0.lock().commit_error(
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
                        .0
                        .lock()
                        .commit_error(path.clone(), SoundBufferResourceLoadError::Io(e));
                }
            }
        })
    }
}
