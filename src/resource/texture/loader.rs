//! Texture loader.

use std::sync::Arc;

use fyrox_resource::io::ResourceIo;

use crate::{
    asset::{
        event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::try_get_import_settings,
        untyped::UntypedResource,
    },
    core::{instant, log::Log, uuid::Uuid, TypeUuidProvider},
    resource::texture::{Texture, TextureImportOptions},
};

/// Default implementation for texture loading.
pub struct TextureLoader {
    /// Default import options for textures.
    pub default_import_options: TextureImportOptions,
}

impl ResourceLoader for TextureLoader {
    fn extensions(&self) -> &[&str] {
        &[
            "jpg", "jpeg", "tga", "gif", "bmp", "png", "tiff", "tif", "dds",
        ]
    }

    fn data_type_uuid(&self) -> Uuid {
        Texture::type_uuid()
    }

    fn load(
        &self,
        texture: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();
        Box::pin(async move {
            let io = io.as_ref();

            let path = texture.path().to_path_buf();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let time = instant::Instant::now();
            match Texture::load_from_file(&path, io, import_options).await {
                Ok(raw_texture) => {
                    Log::info(format!(
                        "Texture {:?} is loaded in {:?}!",
                        path,
                        time.elapsed()
                    ));

                    texture.commit_ok(raw_texture);

                    event_broadcaster.broadcast_loaded_or_reloaded(texture, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load texture {:?}! Reason {:?}",
                        &path, &error
                    ));

                    texture.commit_error(path, error);
                }
            }
        })
    }
}
