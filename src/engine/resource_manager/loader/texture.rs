use crate::{
    core::instant,
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::try_get_import_settings,
    },
    resource::texture::{Texture, TextureData, TextureImportOptions},
    utils::log::Log,
};

pub struct TextureLoader;

impl ResourceLoader<Texture, TextureImportOptions> for TextureLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        texture: Texture,
        default_import_options: TextureImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Texture>,
        reload: bool,
    ) -> Self::Output {
        Box::pin(async move {
            let path = texture.state().path().to_path_buf();

            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            let gen_mip_maps = import_options.minification_filter.is_using_mip_mapping();

            let time = instant::Instant::now();
            match TextureData::load_from_file(&path, import_options.compression, gen_mip_maps).await
            {
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

                    texture.state().commit_ok(raw_texture);

                    event_broadcaster.broadcast_loaded_or_reloaded(texture, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load texture {:?}! Reason {:?}",
                        &path, &error
                    ));

                    texture.state().commit_error(path, error);
                }
            }
        })
    }
}
