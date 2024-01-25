//! User Interface loader.

use crate::{
    constructor::WidgetConstructorContainer,
    core::{uuid::Uuid, TypeUuidProvider},
    UserInterface,
};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    manager::ResourceManager,
    state::LoadError,
};
use std::{path::PathBuf, sync::Arc};

/// Default implementation for UI loading.
pub struct UserInterfaceLoader {
    pub resource_manager: ResourceManager,
}

impl ResourceLoader for UserInterfaceLoader {
    fn extensions(&self) -> &[&str] {
        &["ui"]
    }

    fn data_type_uuid(&self) -> Uuid {
        UserInterface::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let io = io.as_ref();
            let ui = UserInterface::load_from_file_ex(
                &path,
                Arc::new(WidgetConstructorContainer::new()),
                resource_manager,
                io,
            )
            .await
            .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(ui))
        })
    }
}
