//! Material loader.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
        manager::ResourceManager,
    },
    core::{uuid::Uuid, TypeUuidProvider},
    material::Material,
};
use fyrox_resource::state::LoadError;
use std::{path::PathBuf, sync::Arc};

/// Default implementation for material loading.
pub struct MaterialLoader {
    /// Resource manager that will be used to load internal shader resources of materials.
    pub resource_manager: ResourceManager,
}

impl ResourceLoader for MaterialLoader {
    fn extensions(&self) -> &[&str] {
        &["material"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Material::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let material = Material::from_file(&path, io.as_ref(), resource_manager)
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(material))
        })
    }
}
