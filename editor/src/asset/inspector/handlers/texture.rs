use crate::asset::inspector::handlers::ImportOptionsHandler;
use fyrox::{
    core::{append_extension, futures::executor::block_on, reflect::prelude::*},
    engine::resource_manager::{
        options::{try_get_import_settings, ImportOptions},
        ResourceManager,
    },
    gui::inspector::{PropertyAction, PropertyChanged},
    resource::texture::TextureImportOptions,
    utils::log::Log,
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

    fn value(&self) -> &dyn Reflect {
        &self.options
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) {
        PropertyAction::from_field_kind(&property_changed.value).apply(
            &property_changed.path(),
            &mut self.options,
            &mut |result| {
                Log::verify(result);
            },
        );
    }
}
