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
