//! Texture loader.

use crate::{
    asset::{
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::try_get_import_settings,
    },
    core::instant,
    resource::texture::{Texture, TextureImportOptions},
    utils::log::Log,
};
use fyrox_resource::event::ResourceEventBroadcaster;
use fyrox_resource::untyped::UntypedResource;
use std::any::Any;

/// Default implementation for texture loading.
pub struct TextureLoader {
    /// Default import options for textures.
    pub default_import_options: TextureImportOptions,
}

impl ResourceLoader for TextureLoader {
    fn extensions(&self) -> &[&str] {
        &["jpg", "jpeg", "tga", "gif", "bmp", "png", "tiff", "dds"]
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn load(
        &self,
        texture: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let path = texture.path().to_path_buf();

            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            let gen_mip_maps = import_options.minification_filter.is_using_mip_mapping();

            let time = instant::Instant::now();
            match Texture::load_from_file(&path, import_options.compression, gen_mip_maps).await {
                Ok(mut raw_texture) => {
                    Log::info(format!(
                        "Texture {:?} is loaded in {:?}!",
                        path,
                        time.elapsed()
                    ));

                    raw_texture.set_magnification_filter(import_options.magnification_filter);
                    raw_texture.set_minification_filter(import_options.minification_filter);
                    raw_texture.set_anisotropy_level(import_options.anisotropy);
                    raw_texture.set_s_wrap_mode(import_options.s_wrap_mode);
                    raw_texture.set_t_wrap_mode(import_options.t_wrap_mode);

                    texture.0.lock().commit_ok(raw_texture);

                    event_broadcaster.broadcast_loaded_or_reloaded(texture, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load texture {:?}! Reason {:?}",
                        &path, &error
                    ));

                    texture.0.lock().commit_error(path, error);
                }
            }
        })
    }
}
