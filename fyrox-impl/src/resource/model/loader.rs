//! Model loader.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{
            BoxedImportOptionsLoaderFuture, BoxedLoaderFuture, LoaderPayload, ResourceLoader,
        },
        manager::ResourceManager,
        options::{try_get_import_settings, try_get_import_settings_opaque, BaseImportOptions},
    },
    core::{uuid::Uuid, TypeUuidProvider},
    engine::SerializationContext,
    resource::model::{Model, ModelImportOptions},
};
use fyrox_resource::state::LoadError;
use std::{path::PathBuf, sync::Arc};

/// Default implementation for model loading.
pub struct ModelLoader {
    /// Resource manager to allow complex model loading.
    pub resource_manager: ResourceManager,
    /// Node constructors contains a set of constructors that allows to build a node using its
    /// type UUID.
    pub serialization_context: Arc<SerializationContext>,
    /// Default import options for model resources.
    pub default_import_options: ModelImportOptions,
}

impl ResourceLoader for ModelLoader {
    fn extensions(&self) -> &[&str] {
        &["rgs", "fbx"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Model::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        let node_constructors = self.serialization_context.clone();
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let io = io.as_ref();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let model = Model::load(
                path,
                io,
                node_constructors,
                resource_manager,
                import_options,
            )
            .await
            .map_err(LoadError::new)?;

            Ok(LoaderPayload::new(model))
        })
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            try_get_import_settings_opaque::<ModelImportOptions>(&resource_path, &*io).await
        })
    }

    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        Some(Box::<ModelImportOptions>::default())
    }
}
