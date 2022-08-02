use crate::asset::inspector::handlers::ImportOptionsHandler;
use fyrox::{
    core::{append_extension, futures::executor::block_on, inspect::Inspect},
    engine::resource_manager::{
        loader::sound::SoundBufferImportOptions,
        options::{try_get_import_settings, ImportOptions},
        ResourceManager,
    },
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

        let texture = resource_manager.request_sound_buffer(&self.resource_path);
        resource_manager
            .state()
            .containers_mut()
            .sound_buffers
            .reload_resource(texture);
    }

    fn revert(&mut self) {
        self.options = block_on(try_get_import_settings(&self.resource_path)).unwrap_or_default();
    }

    fn value(&self) -> &dyn Inspect {
        &self.options
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) {
        Log::verify(
            PropertyAction::from_field_kind(&property_changed.value)
                .apply(&property_changed.path(), &mut self.options),
        )
    }
}
