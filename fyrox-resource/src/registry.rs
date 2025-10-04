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

//! Resource registry is a database, that contains `UUID -> Path` mappings for every **external**
//! resource used in your game. See [`ResourceRegistry`] docs for more info.

use crate::{
    core::{
        append_extension, err, info, io::FileError, ok_or_return, parking_lot::Mutex,
        replace_slashes, warn, Uuid,
    },
    io::ResourceIo,
    loader::ResourceLoadersContainer,
    metadata::ResourceMetadata,
    state::WakersList,
};
use fxhash::FxHashSet;
use fyrox_core::SafeLock;
use ron::ser::PrettyConfig;
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Component, Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// A type alias for the actual registry data container.
pub type RegistryContainer = BTreeMap<Uuid, PathBuf>;

/// An extension trait with save/load methods.
#[allow(async_fn_in_trait)]
pub trait RegistryContainerExt: Sized {
    /// Serializes the registry into a formatted string.
    fn serialize_to_string(&self) -> Result<String, FileError>;

    /// Tries to load a registry from a file using the specified resource IO. This method is intended
    /// to be used only in async contexts.
    async fn load_from_file(path: &Path, resource_io: &dyn ResourceIo) -> Result<Self, FileError>;

    /// Tries to save the registry into a file using the specified resource IO. This method is
    /// intended to be used only in async contexts.
    async fn save(&self, path: &Path, resource_io: &dyn ResourceIo) -> Result<(), FileError>;

    /// Same as [`Self::save`], but synchronous.
    fn save_sync(&self, path: &Path, resource_io: &dyn ResourceIo) -> Result<(), FileError>;
}

impl RegistryContainerExt for RegistryContainer {
    fn serialize_to_string(&self) -> Result<String, FileError> {
        ron::ser::to_string_pretty(self, PrettyConfig::default()).map_err(|err| {
            FileError::Custom(format!(
                "Unable to serialize resource registry! Reason: {err}"
            ))
        })
    }

    async fn load_from_file(path: &Path, resource_io: &dyn ResourceIo) -> Result<Self, FileError> {
        resource_io.load_file(path).await.and_then(|metadata| {
            ron::de::from_bytes::<Self>(&metadata).map_err(|err| {
                FileError::Custom(format!(
                    "Unable to deserialize the resource registry. Reason: {err:?}"
                ))
            })
        })
    }

    async fn save(&self, path: &Path, resource_io: &dyn ResourceIo) -> Result<(), FileError> {
        let string = self.serialize_to_string()?;
        resource_io.write_file(path, string.into_bytes()).await
    }

    fn save_sync(&self, path: &Path, resource_io: &dyn ResourceIo) -> Result<(), FileError> {
        let string = self.serialize_to_string()?;
        resource_io.write_file_sync(path, string.as_bytes())
    }
}

/// Actual status of a resource registry.
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ResourceRegistryStatus {
    /// The status is unknown. It means that the registry wasn't even attempted to be loaded from
    /// a file. This status will prevent any access to resources through a resource manager - all
    /// requested resources will immediately fail to load.
    #[default]
    Unknown,

    /// Fully loaded registry and ready to use.
    Loaded,

    /// The registry is still loading and has to be waited for. See [`ResourceRegistryStatusFlag`]
    /// for more info.
    Loading,
}

/// Internal data of the registry status flag.
#[derive(Clone, Default)]
pub struct RegistryReadyFlagData {
    status: ResourceRegistryStatus,
    wakers: WakersList,
}

/// A shared flag that can be used to fetch the current status of a resource registry. This struct
/// supports [`Future`] trait, which means that you can `.await` it in an async context to wait
/// until the registry is fully loaded (or failed to load). Any access to the registry in an async
/// context must be guarded with such `.await` call.
#[derive(Default, Clone)]
pub struct ResourceRegistryStatusFlag(Arc<Mutex<RegistryReadyFlagData>>);

impl ResourceRegistryStatusFlag {
    /// Returns current status of the registry.
    pub fn status(&self) -> ResourceRegistryStatus {
        self.0.safe_lock().status
    }

    fn mark_as(&self, status: ResourceRegistryStatus) {
        let mut lock = self.0.safe_lock();

        lock.status = status;

        for waker in lock.wakers.drain(..) {
            waker.wake();
        }
    }

    /// Marks the registry as loaded.
    pub fn mark_as_loaded(&self) {
        self.mark_as(ResourceRegistryStatus::Loaded);
    }

    /// Marks the registry as unknown, due to an error.
    pub fn mark_as_unknown(&self) {
        self.mark_as(ResourceRegistryStatus::Unknown);
    }

    /// Marks the registry as loading. This method should be used before trying to load a registry
    /// from an external source.
    pub fn mark_as_loading(&self) {
        self.0.safe_lock().status = ResourceRegistryStatus::Loading;
    }
}

impl Future for ResourceRegistryStatusFlag {
    type Output = ResourceRegistryStatus;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut lock = self.0.safe_lock();

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

/// A mutable reference to a resource registry. Automatically saves the registry back to a source
/// from which it was loaded when an instance of this object is dropped. To prevent saving use
/// [`std::mem::forget`] after you've finished working with the mutable reference.
pub struct ResourceRegistryRefMut<'a> {
    registry: &'a mut ResourceRegistry,
}

impl ResourceRegistryRefMut<'_> {
    /// Writes the new metadata file for a resource at the given path and registers the resource
    /// in the registry.
    pub fn write_metadata(
        &mut self,
        uuid: Uuid,
        path: impl AsRef<Path>,
    ) -> Result<Option<PathBuf>, FileError> {
        ResourceMetadata::new_with_random_id().save_sync(
            &append_extension(path.as_ref(), ResourceMetadata::EXTENSION),
            &*self.registry.io,
        )?;

        Ok(self.register(uuid, path.as_ref().to_path_buf()))
    }

    /// Unregisters the resource at the given path (if any) from the registry and deletes its
    /// associated metadata file.
    pub fn remove_metadata(&mut self, path: impl AsRef<Path>) -> Result<(), FileError> {
        if let Some(uuid) = self.registry.path_to_uuid(path.as_ref()) {
            self.unregister(uuid);

            let metadata_path = append_extension(path.as_ref(), ResourceMetadata::EXTENSION);

            self.registry.io.delete_file_sync(&metadata_path)?;

            Ok(())
        } else {
            Err(FileError::Custom(format!(
                "The {} resource is not registered in the registry!",
                path.as_ref().display()
            )))
        }
    }

    /// Registers a new pair `UUID -> Path`.
    pub fn register(&mut self, uuid: Uuid, path: PathBuf) -> Option<PathBuf> {
        self.registry.paths.insert(uuid, path)
    }

    /// Unregisters a resource path with the given UUID.
    pub fn unregister(&mut self, uuid: Uuid) -> Option<PathBuf> {
        self.registry.paths.remove(&uuid)
    }

    /// Unregisters a resource path.
    pub fn unregister_path(&mut self, path: &Path) -> Option<Uuid> {
        let uuid = self.registry.path_to_uuid(path)?;
        self.registry.paths.remove(&uuid);
        Some(uuid)
    }

    /// Completely replaces the internal storage.
    pub fn set_container(&mut self, registry_container: RegistryContainer) {
        self.registry.paths = registry_container;
    }
}

impl Drop for ResourceRegistryRefMut<'_> {
    fn drop(&mut self) {
        self.registry.save_sync();
    }
}

/// Resource registry is responsible for UUID mapping of resource files. It maintains a map of
/// `UUID -> Resource Path`.
#[derive(Clone)]
pub struct ResourceRegistry {
    path: PathBuf,
    paths: RegistryContainer,
    status: ResourceRegistryStatusFlag,
    io: Arc<dyn ResourceIo>,
    /// A list of folder that should be excluded when scanning the project folder for supported
    /// resources. By default, it contains `./target` (a folder with build artifacts) and `./build`
    /// (a folder with production builds) folders.
    pub excluded_folders: FxHashSet<PathBuf>,
}

impl ResourceRegistry {
    /// Default path of the registry. It can be overridden on a registry instance using
    /// [`Self::set_path`] method.
    pub const DEFAULT_PATH: &'static str = "data/resources.registry";

    /// Creates a new resource registry with the given resource IO.
    pub fn new(io: Arc<dyn ResourceIo>) -> Self {
        let mut excluded_folders = FxHashSet::default();

        // Exclude build artifacts folder by default.
        excluded_folders.insert(PathBuf::from("target"));
        // Exclude the standard production build folder as well.
        excluded_folders.insert(PathBuf::from("build"));

        Self {
            path: PathBuf::from(Self::DEFAULT_PATH),
            paths: Default::default(),
            status: Default::default(),
            io,
            excluded_folders,
        }
    }

    /// Returns a shared reference to the status flag. See [`ResourceRegistryStatusFlag`] docs for
    /// more info.
    pub fn status_flag(&self) -> ResourceRegistryStatusFlag {
        self.status.clone()
    }

    /// Normalizes the path by resolving all `.` and `..` and removing any prefixes. Also replaces
    /// `\\` slashes to cross-platform `/` slashes.
    pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
        let mut components = path.as_ref().components().peekable();
        let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
            components.next();
            PathBuf::from(c.as_os_str())
        } else {
            PathBuf::new()
        };

        for component in components {
            match component {
                Component::Prefix(..) => unreachable!(),
                Component::RootDir => {
                    ret.push(component.as_os_str());
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    ret.pop();
                }
                Component::Normal(c) => {
                    ret.push(c);
                }
            }
        }

        // The resource registry uses normalized paths with `/` slashes, and this step is needed
        // mostly on Windows which uses `\` slashes.
        replace_slashes(ret)
    }

    /// Returns a reference to the actual container of the resource entries.
    pub fn inner(&self) -> &RegistryContainer {
        &self.paths
    }

    /// Sets a new path for the registry, but **does not** saves it.
    pub fn set_path(&mut self, path: impl AsRef<Path>) {
        self.path = path.as_ref().to_owned();
    }

    /// Returns a path to which the resource registry is (or may) be saved.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a directory to which the resource registry is (or may) be saved.
    pub fn directory(&self) -> Option<&Path> {
        self.path.parent()
    }

    /// Asynchronously saves the registry.
    pub async fn save(&self) {
        match self.paths.save(&self.path, &*self.io).await {
            Err(error) => {
                err!(
                    "Unable to write the resource registry at the {} path! Reason: {:?}",
                    self.path.display(),
                    error
                )
            }
            Ok(_) => {
                info!(
                    "The registry was successfully saved to {}!",
                    self.path.display()
                )
            }
        }
    }

    /// Same as [`Self::save`], but synchronous.
    pub fn save_sync(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match self.paths.save_sync(&self.path, &*self.io) {
                Err(error) => {
                    err!(
                        "Unable to write the resource registry at the {} path! Reason: {:?}",
                        self.path.display(),
                        error
                    )
                }
                Ok(_) => {
                    info!(
                        "The registry was successfully saved to {}!",
                        self.path.display()
                    )
                }
            }
        }
    }

    /// Begins registry modification. See [`ResourceRegistryRefMut`] docs for more info.
    pub fn modify(&mut self) -> ResourceRegistryRefMut<'_> {
        ResourceRegistryRefMut { registry: self }
    }

    /// Tries to get a path associated with the given resource UUID.
    pub fn uuid_to_path(&self, uuid: Uuid) -> Option<&Path> {
        self.paths.get(&uuid).map(|path| path.as_path())
    }

    /// Same as [`Self::uuid_to_path`], but returns [`PathBuf`] instead of `&Path`.
    pub fn uuid_to_path_buf(&self, uuid: Uuid) -> Option<PathBuf> {
        self.uuid_to_path(uuid).map(|path| path.to_path_buf())
    }

    /// Tries to find a UUID that corresponds for the given path.
    pub fn path_to_uuid(&self, path: &Path) -> Option<Uuid> {
        self.paths
            .iter()
            .find_map(|(k, v)| if v == path { Some(*k) } else { None })
    }

    /// Checks if the path is registered in the resource registry.
    pub fn is_registered(&self, path: &Path) -> bool {
        self.path_to_uuid(path).is_some()
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
        excluded_folders: FxHashSet<PathBuf>,
    ) -> RegistryContainer {
        let registry_path = root.as_ref();
        let registry_folder = registry_path
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        info!(
            "Scanning {} folder for supported resources...",
            registry_folder.display()
        );

        let mut container = RegistryContainer::default();

        let mut paths_to_visit = ok_or_return!(
            resource_io.read_directory(&registry_folder).await,
            container
        )
        .collect::<Vec<_>>();

        while let Some(fs_path) = paths_to_visit.pop() {
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

            if excluded_folders.contains(&path) {
                warn!(
                    "Skipping {} folder, because it is in the excluded folders list!",
                    path.display()
                );

                continue;
            }

            if resource_io.is_dir(&path).await {
                // Continue iterating on subfolders.
                if let Ok(iter) = resource_io.read_directory(&path).await {
                    paths_to_visit.extend(iter);
                }

                continue;
            }

            if !loaders.safe_lock().is_supported_resource(&path) {
                if path
                    .extension()
                    .is_some_and(|ext| ext != "meta" && ext != "registry")
                {
                    info!(
                        "Skipping {} file, because there's no loader for it.",
                        path.display()
                    );
                }

                continue;
            }

            let metadata_path = append_extension(&path, ResourceMetadata::EXTENSION);
            let metadata =
                match ResourceMetadata::load_from_file_async(&metadata_path, &*resource_io).await {
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
                        if let Err(err) =
                            new_metadata.save_async(&metadata_path, &*resource_io).await
                        {
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
