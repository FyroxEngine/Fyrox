use crate::asset::inspector::handlers::ImportOptionsHandler;
use fyrox::{
    core::{append_extension, futures::executor::block_on, inspect::Inspect},
    engine::resource_manager::{
        options::{try_get_import_settings, ImportOptions},
        ResourceManager,
    },
    gui::inspector::{FieldKind, PropertyChanged},
    resource::model::ModelImportOptions,
};
use std::path::{Path, PathBuf};

pub struct ModelImportOptionsHandler {
    resource_path: PathBuf,
    options: ModelImportOptions,
}

impl ModelImportOptionsHandler {
    pub fn new(resource_path: &Path) -> Self {
        Self {
            resource_path: resource_path.to_owned(),
            options: block_on(try_get_import_settings(resource_path)).unwrap_or_default(),
        }
    }
}

impl ImportOptionsHandler for ModelImportOptionsHandler {
    fn apply(&self, _resource_manager: ResourceManager) {
        // TODO: Reload model.

        self.options
            .save(&append_extension(&self.resource_path, "options"));
    }

    fn revert(&mut self) {
        self.options = block_on(try_get_import_settings(&self.resource_path)).unwrap_or_default();
    }

    fn value(&self) -> &dyn Inspect {
        &self.options
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) {
        if let FieldKind::Object(ref args) = property_changed.value {
            if let ModelImportOptions::MATERIAL_SEARCH_OPTIONS = property_changed.name.as_ref() {
                self.options.material_search_options = args.cast_clone().unwrap()
            }
        }
    }
}
