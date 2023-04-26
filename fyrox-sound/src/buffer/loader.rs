//! Sound buffer loader.

use crate::buffer::{DataSource, SoundBuffer, SoundBufferResourceLoadError};
use fyrox_core::log::Log;
use fyrox_core::reflect::prelude::*;
use fyrox_resource::{
    event::ResourceEventBroadcaster,
    loader::{BoxedLoaderFuture, ResourceLoader},
    options::{try_get_import_settings, ImportOptions},
    untyped::UntypedResource,
};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// Defines sound buffer resource import options.
#[derive(Clone, Deserialize, Serialize, Default, Debug, Reflect)]
pub struct SoundBufferImportOptions {
    /// Whether the buffer is streaming or not.
    pub stream: bool,
}

impl ImportOptions for SoundBufferImportOptions {}

/// Default implementation for sound buffer loading.
pub struct SoundBufferLoader {
    /// Default import options for sound buffer resources.
    pub default_import_options: SoundBufferImportOptions,
}

impl ResourceLoader for SoundBufferLoader {
    fn extensions(&self) -> &[&str] {
        &["wav", "ogg"]
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

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
                            resource.commit_ok(sound_buffer);

                            event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);

                            Log::info(format!("Sound buffer {:?} is loaded!", path));
                        }
                        Err(_) => {
                            resource.commit_error(
                                path.clone(),
                                SoundBufferResourceLoadError::UnsupportedFormat,
                            );

                            Log::info(format!("Unable to load sound buffer from {:?}!", path));
                        }
                    }
                }
                Err(e) => {
                    Log::err(format!("Invalid data source for sound buffer: {:?}", e));

                    resource.commit_error(path.clone(), SoundBufferResourceLoadError::Io(e));
                }
            }
        })
    }
}
