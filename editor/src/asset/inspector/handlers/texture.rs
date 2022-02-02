use crate::asset::inspector::handlers::ImportOptionsHandler;
use fyrox::{
    core::{append_extension, futures::executor::block_on, inspect::Inspect},
    engine::resource_manager::{
        options::{try_get_import_settings, ImportOptions},
        ResourceManager,
    },
    gui::inspector::{FieldKind, PropertyChanged},
    resource::texture::TextureImportOptions,
};
use std::path::{Path, PathBuf};

pub struct TextureImportOptionsHandler {
    resource_path: PathBuf,
    options: TextureImportOptions,
}

impl TextureImportOptionsHandler {
    pub fn new(resource_path: &Path) -> Self {
        Self {
            resource_path: resource_path.to_owned(),
            options: block_on(try_get_import_settings(resource_path)).unwrap_or_default(),
        }
    }
}

impl ImportOptionsHandler for TextureImportOptionsHandler {
    fn apply(&self, resource_manager: ResourceManager) {
        self.options
            .save(&append_extension(&self.resource_path, "options"));

        let texture = resource_manager.request_texture(&self.resource_path);
        resource_manager
            .state()
            .containers_mut()
            .textures
            .reload_resource(texture);
    }

    fn revert(&mut self) {
        self.options = block_on(try_get_import_settings(&self.resource_path)).unwrap_or_default();
    }

    fn value(&self) -> &dyn Inspect {
        &self.options
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) {
        if let FieldKind::Object(ref args) = property_changed.value {
            match property_changed.name.as_ref() {
                TextureImportOptions::MINIFICATION_FILTER => self
                    .options
                    .set_minification_filter(args.cast_clone().unwrap()),
                TextureImportOptions::MAGNIFICATION_FILTER => self
                    .options
                    .set_magnification_filter(args.cast_clone().unwrap()),
                TextureImportOptions::S_WRAP_MODE => {
                    self.options.set_s_wrap_mode(args.cast_clone().unwrap())
                }
                TextureImportOptions::T_WRAP_MODE => {
                    self.options.set_t_wrap_mode(args.cast_clone().unwrap())
                }
                TextureImportOptions::ANISOTROPY => {
                    self.options.set_anisotropy(args.cast_clone().unwrap())
                }
                TextureImportOptions::COMPRESSION => {
                    self.options.set_compression(args.cast_clone().unwrap())
                }
                _ => (),
            }
        }
    }
}
