use crate::{
    asset::ResourceState,
    core::instant,
    engine::resource_manager::{
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::try_get_import_settings,
    },
    renderer::TextureUploadSender,
    resource::texture::{Texture, TextureData, TextureImportOptions},
    utils::log::{Log, MessageKind},
};
use std::{path::PathBuf, sync::Arc};

pub struct TextureLoader {
    pub upload_sender: TextureUploadSender,
}

impl ResourceLoader<Texture, TextureImportOptions> for TextureLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        texture: Texture,
        path: PathBuf,
        default_import_options: TextureImportOptions,
    ) -> Self::Output {
        let upload_sender = self.upload_sender.clone();

        let fut = async move {
            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            let gen_mip_maps = import_options.minification_filter.is_using_mip_mapping();

            let time = instant::Instant::now();
            match TextureData::load_from_file(&path, import_options.compression, gen_mip_maps).await
            {
                Ok(mut raw_texture) => {
                    Log::writeln(
                        MessageKind::Information,
                        format!("Texture {:?} is loaded in {:?}!", path, time.elapsed()),
                    );

                    raw_texture.set_magnification_filter(import_options.magnification_filter);
                    raw_texture.set_minification_filter(import_options.minification_filter);
                    raw_texture.set_anisotropy_level(import_options.anisotropy);
                    raw_texture.set_s_wrap_mode(import_options.s_wrap_mode);
                    raw_texture.set_t_wrap_mode(import_options.t_wrap_mode);

                    texture.state().commit(ResourceState::Ok(raw_texture));

                    // Ask renderer to upload texture to GPU.
                    upload_sender.request_upload(texture);
                }
                Err(error) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!("Unable to load texture {:?}! Reason {:?}", &path, &error),
                    );

                    texture.state().commit(ResourceState::LoadError {
                        path,
                        error: Some(Arc::new(error)),
                    });
                }
            }
        };
        Box::pin(fut)
    }
}
