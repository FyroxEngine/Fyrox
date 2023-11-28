//! Sound buffer loader.

use crate::buffer::{DataSource, SoundBuffer, SoundBufferResourceLoadError};
use fyrox_core::{log::Log, reflect::prelude::*, uuid::Uuid, TypeUuidProvider};
use fyrox_resource::options::BaseImportOptions;
use fyrox_resource::{
    event::ResourceEventBroadcaster,
    io::ResourceIo,
    loader::{BoxedImportOptionsLoaderFuture, BoxedLoaderFuture, ResourceLoader},
    options::{try_get_import_settings, try_get_import_settings_opaque, ImportOptions},
    untyped::UntypedResource,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

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

    fn data_type_uuid(&self) -> Uuid {
        SoundBuffer::type_uuid()
    }

    fn load(
        &self,
        resource: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let io = io.as_ref();

            let path = resource.path().to_path_buf();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            match DataSource::from_file(&path, io).await {
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
                            resource.commit_error(SoundBufferResourceLoadError::UnsupportedFormat);

                            Log::info(format!("Unable to load sound buffer from {:?}!", path));
                        }
                    }
                }
                Err(e) => {
                    Log::err(format!("Invalid data source for sound buffer: {:?}", e));

                    resource.commit_error(SoundBufferResourceLoadError::Io(e));
                }
            }
        })
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            try_get_import_settings_opaque::<SoundBufferImportOptions>(&resource_path, &*io).await
        })
    }

    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        Some(Box::<SoundBufferImportOptions>::default())
    }
}
