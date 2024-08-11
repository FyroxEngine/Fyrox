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

//! Shader loader.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    },
    core::{uuid::Uuid, TypeUuidProvider},
    material::shader::Shader,
};
use fyrox_resource::state::LoadError;
use std::{path::PathBuf, sync::Arc};

/// Default implementation for shader loading.
pub struct ShaderLoader;

impl ResourceLoader for ShaderLoader {
    fn extensions(&self) -> &[&str] {
        &["shader"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Shader::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let shader_state = Shader::from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(shader_state))
        })
    }
}
