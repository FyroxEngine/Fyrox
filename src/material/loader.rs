//! Material loader.

use crate::{
    asset::{
        event::ResourceEventBroadcaster,
        io::ResourceIo,
        loader::{BoxedLoaderFuture, ResourceLoader},
        untyped::UntypedResource,
    },
    core::{log::Log, uuid::Uuid, TypeUuidProvider},
    material::Material,
};
use std::sync::Arc;

/// Default implementation for material loading.
pub struct MaterialLoader;

impl ResourceLoader for MaterialLoader {
    fn extensions(&self) -> &[&str] {
        &["material"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Material::type_uuid()
    }

    fn load(
        &self,
        material: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = material.path().to_path_buf();

            match Material::from_file(&path, io.as_ref()).await {
                Ok(shader_state) => {
                    Log::info(format!("Material {:?} is loaded!", path));

                    material.commit_ok(shader_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(material, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load material from {:?}! Reason {:?}",
                        path, error
                    ));

                    material.commit_error(path, error);
                }
            }
        })
    }
}
