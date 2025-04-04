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

use crate::{
    io::ResourceIo, loader::ResourceLoadersContainer, metadata::ResourceMetadata, state::WakersList,
};
use fyrox_core::{
    append_extension, info, io::FileError, ok_or_return, parking_lot::Mutex, replace_slashes, warn,
    Uuid,
};
use ron::ser::PrettyConfig;
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

pub type RegistryContainer = BTreeMap<Uuid, PathBuf>;

#[allow(async_fn_in_trait)]
pub trait RegistryContainerExt: Sized {
    async fn load_from_file(path: &Path, resource_io: &dyn ResourceIo) -> Result<Self, FileError>;
    async fn save(&self, path: &Path, resource_io: &dyn ResourceIo) -> Result<(), FileError>;
}

impl RegistryContainerExt for RegistryContainer {
    async fn load_from_file(path: &Path, resource_io: &dyn ResourceIo) -> Result<Self, FileError> {
        resource_io.load_file(path).await.and_then(|metadata| {
            ron::de::from_bytes::<Self>(&metadata).map_err(|err| {
                FileError::Custom(format!(
                    "Unable to deserialize the resource registry. Reason: {:?}",
                    err
                ))
            })
        })
    }

    async fn save(&self, path: &Path, resource_io: &dyn ResourceIo) -> Result<(), FileError> {
        let string = ron::ser::to_string_pretty(self, PrettyConfig::default()).map_err(|err| {
            FileError::Custom(format!(
                "Unable to serialize resource registry! Reason: {}",
                err
            ))
        })?;
        resource_io.write_file(path, string.into_bytes()).await
    }
}

#[derive(Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ResourceRegistryStatus {
    #[default]
    Unknown,
    Loaded,
    Loading,
}

#[derive(Clone, Default)]
pub struct RegistryReadyFlagData {
    status: ResourceRegistryStatus,
    wakers: WakersList,
}

#[derive(Default, Clone)]
pub struct ResourceRegistryStatusFlag(Arc<Mutex<RegistryReadyFlagData>>);

impl ResourceRegistryStatusFlag {
    pub fn mark_as_loaded(&self) {
        let mut lock = self.0.lock();

        lock.status = ResourceRegistryStatus::Loaded;

        for waker in lock.wakers.drain(..) {
            waker.wake();
        }
    }

    pub fn mark_as_loading(&self) {
        self.0.lock().status = ResourceRegistryStatus::Loading;
    }
}

impl Future for ResourceRegistryStatusFlag {
    type Output = ResourceRegistryStatus;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut lock = self.0.lock();

        match lock.status {
            ResourceRegistryStatus::Unknown => Poll::Ready(ResourceRegistryStatus::Unknown),
            ResourceRegistryStatus::Loaded => Poll::Ready(ResourceRegistryStatus::Loaded),
            ResourceRegistryStatus::Loading => {
                lock.wakers.add_waker(cx.waker());
                Poll::Pending
            }
        }
    }
}

async fn make_relative_path_async<P: AsRef<Path>>(
    path: P,
    io: &dyn ResourceIo,
) -> Result<PathBuf, FileError> {
    let path = path.as_ref();
    // Canonicalization requires the full path to exist, so remove the file name before
    // calling canonicalize.
    let file_name = path.file_name().ok_or(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("Invalid path: {}", path.display()),
    ))?;
    let dir = path.parent();
    let dir = if let Some(dir) = dir {
        if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        }
    } else {
        Path::new(".")
    };
    let canon_path = io
        .canonicalize_path(dir)
        .await
        .map_err(|err| {
            FileError::Custom(format!(
                "Unable to canonicalize '{}'. Reason: {err:?}",
                dir.display()
            ))
        })?
        .join(file_name);
    let current_dir = io.canonicalize_path(&std::env::current_dir()?).await?;
    match canon_path.strip_prefix(current_dir) {
        Ok(relative_path) => Ok(replace_slashes(relative_path)),
        Err(err) => Err(FileError::Custom(format!(
            "unable to strip prefix from '{}'! Reason: {err}",
            canon_path.display()
        ))),
    }
}

/// Resource registry is responsible for UUID mapping of resource files. It maintains a map of
/// `UUID -> Resource Path`.
#[derive(Default, Clone)]
pub struct ResourceRegistry {
    pub paths: RegistryContainer,
    pub status: ResourceRegistryStatusFlag,
}

impl ResourceRegistry {
    pub const DEFAULT_PATH: &'static str = "./resources.registry";

    pub fn register(&mut self, uuid: Uuid, path: PathBuf) -> Option<PathBuf> {
        self.paths.insert(uuid, path)
    }

    pub fn unregister(&mut self, uuid: Uuid) -> Option<PathBuf> {
        self.paths.remove(&uuid)
    }

    pub fn unregister_path(&mut self, path: &Path) -> Option<Uuid> {
        let uuid = self.path_to_uuid(path)?;
        self.paths.remove(&uuid);
        Some(uuid)
    }

    pub fn set_container(&mut self, registry_container: RegistryContainer) {
        self.paths = registry_container;
    }

    pub fn uuid_to_path(&self, uuid: Uuid) -> Option<&Path> {
        self.paths.get(&uuid).map(|path| path.as_path())
    }

    pub fn uuid_to_path_buf(&self, uuid: Uuid) -> Option<PathBuf> {
        self.uuid_to_path(uuid).map(|path| path.to_path_buf())
    }

    pub fn path_to_uuid(&self, path: &Path) -> Option<Uuid> {
        self.paths
            .iter()
            .find_map(|(k, v)| if v == path { Some(*k) } else { None })
    }

    pub fn path_to_uuid_or_random(&self, path: &Path) -> Uuid {
        self.path_to_uuid(path).unwrap_or_else(|| {
            warn!(
                "There's no UUID for {} resource! Random UUID will be used, run \
                    ResourceRegistry::scan_and_update to generate resource ids!",
                path.display()
            );

            Uuid::new_v4()
        })
    }

    /// Searches for supported resources starting from the given path and builds a mapping `UUID -> Path`.
    /// If a supported resource does not have a metadata file besides it, this method will automatically
    /// add it with a new UUID and add the resource to the registry.
    ///
    /// This method does **not** load any resource, instead it checks extension of every file in the
    /// given directory, and if there's a loader for it, "remember" the resource.
    pub async fn scan(
        resource_io: Arc<dyn ResourceIo>,
        loaders: Arc<Mutex<ResourceLoadersContainer>>,
        root: impl AsRef<Path>,
    ) -> RegistryContainer {
        let registry_path = root.as_ref();
        let registry_folder = registry_path
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let mut container = RegistryContainer::default();

        let file_iterator = ok_or_return!(
            resource_io.walk_directory(&registry_folder).await,
            container
        );
        for fs_path in file_iterator {
            let path = match make_relative_path_async(&fs_path, &*resource_io).await {
                Ok(path) => path,
                Err(err) => {
                    warn!(
                        "Unable to make relative path from {} path! The resource won't be \
                    included in the registry! Reason: {:?}",
                        fs_path.display(),
                        err
                    );
                    continue;
                }
            };

            if !loaders.lock().is_supported_resource(&path) {
                continue;
            }

            let metadata_path = append_extension(&path, ResourceMetadata::EXTENSION);
            let metadata =
                match ResourceMetadata::load_from_file(&metadata_path, &*resource_io).await {
                    Ok(metadata) => metadata,
                    Err(err) => {
                        warn!(
                            "Unable to load metadata for {} resource. Reason: {:?}, The metadata \
                            file will be added/recreated, do **NOT** delete it! Add it to the \
                            version control!",
                            path.display(),
                            err
                        );
                        let new_metadata = ResourceMetadata::new_with_random_id();
                        if let Err(err) = new_metadata.save(&metadata_path, &*resource_io).await {
                            warn!(
                                "Unable to save resource {} metadata. Reason: {:?}",
                                path.display(),
                                err
                            );
                        }
                        new_metadata
                    }
                };

            if container
                .insert(metadata.resource_id, path.clone())
                .is_some()
            {
                warn!(
                    "Resource UUID collision occurred for {} resource!",
                    path.display()
                );
            }

            info!(
                "Resource {} was registered with {} UUID.",
                path.display(),
                metadata.resource_id
            );
        }

        container
    }
}
