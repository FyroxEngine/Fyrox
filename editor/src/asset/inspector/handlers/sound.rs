use crate::asset::inspector::handlers::ImportOptionsHandler;
use fyrox::scene::sound::SoundBuffer;
use fyrox::{
    asset::{
        manager::ResourceManager,
        options::{try_get_import_settings, ImportOptions},
    },
    core::{append_extension, futures::executor::block_on, reflect::prelude::*},
    engine::resource_loaders::sound::SoundBufferImportOptions,
    gui::inspector::{PropertyAction, PropertyChanged},
    utils::log::Log,
};
use std::path::{Path, PathBuf};

pub struct SoundBufferImportOptionsHandler {
    resource_path: PathBuf,
    options: SoundBufferImportOptions,
}

impl SoundBufferImportOptionsHandler {
    pub fn new(resource_path: &Path) -> Self {
        Self {
            resource_path: resource_path.to_owned(),
            options: block_on(try_get_import_settings(resource_path)).unwrap_or_default(),
        }
    }
}

impl ImportOptionsHandler for SoundBufferImportOptionsHandler {
    fn apply(&self, resource_manager: ResourceManager) {
        self.options
            .save(&append_extension(&self.resource_path, "options"));

        let texture = resource_manager.request::<SoundBuffer, _>(&self.resource_path);
        resource_manager
            .state()
            .reload_resource(texture.into_untyped());
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
