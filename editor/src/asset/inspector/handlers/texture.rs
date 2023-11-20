use crate::asset::inspector::handlers::ImportOptionsHandler;
use fyrox::{
    asset::{
        io::FsResourceIo,
        manager::ResourceManager,
        options::{try_get_import_settings, ImportOptions},
    },
    core::{append_extension, futures::executor::block_on, log::Log, reflect::prelude::*},
    gui::inspector::{PropertyAction, PropertyChanged},
    resource::texture::{Texture, TextureImportOptions},
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
            options: block_on(try_get_import_settings(resource_path, &FsResourceIo))
                .unwrap_or_default(),
        }
    }
}

impl ImportOptionsHandler for TextureImportOptionsHandler {
    fn apply(&self, resource_manager: ResourceManager) {
        self.options
            .save(&append_extension(&self.resource_path, "options"));

        let texture = resource_manager.request::<Texture>(&self.resource_path);
        resource_manager
            .state()
            .reload_resource(texture.into_untyped());
    }

    fn revert(&mut self) {
        self.options = block_on(try_get_import_settings(&self.resource_path, &FsResourceIo))
            .unwrap_or_default();
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
