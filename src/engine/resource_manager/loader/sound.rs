use crate::{
    asset::ResourceState,
    core::inspect::{Inspect, PropertyInfo},
    engine::resource_manager::{
        container::event::{ResourceEvent, ResourceEventBroadcaster},
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::{try_get_import_settings, ImportOptions},
    },
    utils::log::{Log, MessageKind},
};
use fyrox_sound::buffer::{
    DataSource, SoundBufferResource, SoundBufferResourceLoadError, SoundBufferState,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

/// Defines sound buffer resource import options.
#[derive(Clone, Deserialize, Serialize, Default, Inspect)]
pub struct SoundBufferImportOptions {
    /// Whether the buffer is streaming or not.
    pub stream: bool,
}

impl ImportOptions for SoundBufferImportOptions {}

pub struct SoundBufferLoader;

impl ResourceLoader<SoundBufferResource, SoundBufferImportOptions> for SoundBufferLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        resource: SoundBufferResource,
        path: PathBuf,
        default_import_options: SoundBufferImportOptions,
        event_broadcaster: ResourceEventBroadcaster<SoundBufferResource>,
    ) -> Self::Output {
        let fut = async move {
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
                            Log::writeln(
                                MessageKind::Information,
                                format!("Sound buffer {:?} is loaded!", path),
                            );

                            resource.state().commit(ResourceState::Ok(sound_buffer));

                            event_broadcaster.broadcast(ResourceEvent::Loaded(resource));
                        }
                        Err(_) => {
                            Log::writeln(
                                MessageKind::Error,
                                format!("Unable to load sound buffer from {:?}!", path),
                            );

                            resource.state().commit(ResourceState::LoadError {
                                path: path.clone(),
                                error: Some(Arc::new(
                                    SoundBufferResourceLoadError::UnsupportedFormat,
                                )),
                            })
                        }
                    }
                }
                Err(e) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!("Invalid data source for sound buffer: {:?}", e),
                    );

                    resource.state().commit(ResourceState::LoadError {
                        path: path.clone(),
                        error: Some(Arc::new(SoundBufferResourceLoadError::Io(e))),
                    })
                }
            }
        };
        Box::pin(fut)
    }
}
