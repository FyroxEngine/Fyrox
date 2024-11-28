// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! User Interface loader.

use crate::constructor::new_widget_constructor_container;
use crate::{
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
                Arc::new(new_widget_constructor_container()),
                resource_manager,
                io,
            )
            .await
            .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(ui))
        })
    }
}
