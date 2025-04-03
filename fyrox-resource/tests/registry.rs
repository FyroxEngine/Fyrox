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

use fyrox_core::{
    futures::executor::block_on, io::FileError, parking_lot::Mutex, reflect::prelude::*, uuid,
    visitor::prelude::*, TypeUuidProvider, Uuid,
};
use fyrox_resource::{
    io::{FsResourceIo, ResourceIo},
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader, ResourceLoadersContainer},
    metadata::ResourceMetadata,
    registry::ResourceRegistry,
    state::LoadError,
    ResourceData,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Serialize, Deserialize, Debug, Visit, Reflect, TypeUuidProvider)]
#[type_uuid(id = "241d14c7-079e-4395-a63c-364f0fc3e6ea")]
struct MyData {
    data: u32,
}

impl MyData {
    pub async fn load_from_file(
        path: &Path,
        resource_io: &dyn ResourceIo,
    ) -> Result<Self, FileError> {
        resource_io.load_file(path).await.and_then(|metadata| {
            ron::de::from_bytes::<Self>(&metadata).map_err(|err| {
                FileError::Custom(format!(
                    "Unable to deserialize the resource metadata. Reason: {:?}",
                    err
                ))
            })
        })
    }
}

impl ResourceData for MyData {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let string = ron::ser::to_string_pretty(self, PrettyConfig::default())
            .map_err(|err| {
                FileError::Custom(format!(
                    "Unable to serialize resource metadata for {} resource! Reason: {}",
                    path.display(),
                    err
                ))
            })
            .map_err(|_| "error".to_string())?;
        let mut file = File::create(path)?;
        file.write_all(string.as_bytes())?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

struct MyDataLoader {}

impl MyDataLoader {
    const EXT: &'static str = "my_data";
}

impl ResourceLoader for MyDataLoader {
    fn extensions(&self) -> &[&str] {
        &[Self::EXT]
    }

    fn data_type_uuid(&self) -> Uuid {
        <MyData as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let my_data = MyData::load_from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(my_data))
        })
    }
}

const TEST_FOLDER: &'static str = "./test_output";

fn make_file_path(n: usize) -> PathBuf {
    Path::new(TEST_FOLDER).join(format!("test{n}.{}", MyDataLoader::EXT))
}

fn make_metadata_file_path(n: usize) -> PathBuf {
    Path::new(TEST_FOLDER).join(format!(
        "test{n}.{}.{}",
        MyDataLoader::EXT,
        ResourceMetadata::EXTENSION
    ))
}

fn write_test_resources() {
    let path = Path::new(TEST_FOLDER);
    if !std::fs::exists(path).unwrap() {
        std::fs::create_dir_all(path).unwrap();
    }

    MyData { data: 123 }.save(&make_file_path(1)).unwrap();
    MyData { data: 321 }.save(&make_file_path(2)).unwrap();
}

#[test]
fn test_registry_scan() {
    write_test_resources();

    assert!(std::fs::exists(make_file_path(1)).unwrap());
    assert!(std::fs::exists(make_file_path(2)).unwrap());

    let io = Arc::new(FsResourceIo);

    let mut loaders = ResourceLoadersContainer::new();
    loaders.set(MyDataLoader {});
    let loaders = Arc::new(Mutex::new(loaders));

    let registry = block_on(ResourceRegistry::scan(io, loaders, TEST_FOLDER));

    assert!(std::fs::exists(make_metadata_file_path(1)).unwrap());
    assert!(std::fs::exists(make_metadata_file_path(2)).unwrap());

    assert_eq!(registry.len(), 2);
}
