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

//! Resource loader. It manages resource loading.

use crate::{
    core::{uuid::Uuid, TypeUuidProvider},
    io::ResourceIo,
    options::BaseImportOptions,
    state::LoadError,
    ResourceData, TypedResourceData,
};
use fyrox_core::io::FileError;
use fyrox_core::platform::TargetPlatform;
use fyrox_core::visitor::{Format, Visitor};
use std::{
    any::Any,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub trait BaseResourceLoader: Any {}
#[cfg(target_arch = "wasm32")]
impl<T: Any> BaseResourceLoader for T {}

#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub trait BaseResourceLoader: Any + Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Any + Send> BaseResourceLoader for T {}

fn convert_ascii_to_binary<F>(
    src_path: PathBuf,
    dest_path: PathBuf,
    is_native_extension: F,
    _platform: TargetPlatform,
    io: Arc<dyn ResourceIo>,
) -> Pin<Box<dyn Future<Output = Result<(), FileError>>>>
where
    F: Fn(&str) -> bool,
{
    if src_path
        .extension()
        .and_then(|src_ext| src_ext.to_str())
        .is_some_and(is_native_extension)
    {
        Box::pin(async move {
            let data = io.load_file(&src_path).await?;
            match Visitor::detect_format_from_slice(&data) {
                Format::Unknown => Err(FileError::Custom("Unknown format!".to_string())),
                Format::Binary => {
                    // Copy the binary format as-is.
                    Ok(io.copy_file(&src_path, &dest_path).await?)
                }
                Format::Ascii => {
                    // Resave the ascii format as binary.
                    let visitor = Visitor::load_from_memory(&data).map_err(|err| {
                        FileError::Custom(format!(
                            "Unable to load {}. Reason: {err}",
                            src_path.display()
                        ))
                    })?;
                    visitor.save_binary_to_file(dest_path).map_err(|err| {
                        FileError::Custom(format!(
                            "Unable to save {}. Reason: {err}",
                            src_path.display()
                        ))
                    })?;
                    Ok(())
                }
            }
        })
    } else {
        Box::pin(async move { io.copy_file(&src_path, &dest_path).await })
    }
}

/// Trait for resource loading.
pub trait ResourceLoader: BaseResourceLoader {
    /// Returns a list of file extensions supported by the loader. Resource manager will use this list
    /// to pick the correct resource loader when the user requests a resource.
    fn extensions(&self) -> &[&str];

    /// Returns `true` if the given extension corresponds to a resource in the native file format.
    /// The default implementation returns `false`, which assumes that the extension corresponds
    /// to a foreign file format.
    fn is_native_extension(&self, #[allow(unused_variables)] ext: &str) -> bool {
        false
    }

    /// Checks if the given extension is supported by this loader. Comparison is case-insensitive.
    fn supports_extension(&self, ext: &str) -> bool {
        self.extensions()
            .iter()
            .any(|e| fyrox_core::cmp_strings_case_insensitive(e, ext))
    }

    /// Must return a type uuid of the resource data type.
    fn data_type_uuid(&self) -> Uuid;

    /// Loads or reloads a resource.
    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture;

    /// Loads a resource from the given path and converts it to a format, that is the most efficient
    /// fot the given platform. This method is usually used to convert resources for production builds;
    /// to make the resources as efficient as possible for the given platform. This method saves the
    /// converted resource to the given `dest_path`. If the resource is already in its final form,
    /// then this method should just copy the file from `src_path` to `dest_path`.
    fn convert(
        &self,
        src_path: PathBuf,
        dest_path: PathBuf,
        #[allow(unused_variables)] platform: TargetPlatform,
        io: Arc<dyn ResourceIo>,
    ) -> Pin<Box<dyn Future<Output = Result<(), FileError>>>> {
        convert_ascii_to_binary(
            src_path,
            dest_path,
            |ext| self.is_native_extension(ext),
            platform,
            io,
        )
    }

    /// Tries to load import settings for a resource.
    fn try_load_import_settings(
        &self,
        #[allow(unused_variables)] resource_path: PathBuf,
        #[allow(unused_variables)] io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move { None })
    }

    /// Returns default import options for the resource.
    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        None
    }
}

/// A result of executing a resource loader.
pub struct LoaderPayload(pub(crate) Box<dyn ResourceData>);

impl LoaderPayload {
    /// Creates a new resource loader payload.
    pub fn new<T: ResourceData>(data: T) -> Self {
        Self(Box::new(data))
    }
}

/// Future type for resource loading. See 'ResourceLoader'.
#[cfg(target_arch = "wasm32")]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = Result<LoaderPayload, LoadError>>>>;

/// Future type for resource loading. See 'ResourceLoader'.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = Result<LoaderPayload, LoadError>> + Send>>;

/// Future type for resource import options loading.
pub type BoxedImportOptionsLoaderFuture =
    Pin<Box<dyn Future<Output = Option<Box<dyn BaseImportOptions>>>>>;

/// Container for resource loaders.
#[derive(Default)]
pub struct ResourceLoadersContainer {
    loaders: Vec<Box<dyn ResourceLoader>>,
}

impl ResourceLoadersContainer {
    /// Creates new empty resource loaders container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds new resource loader or replaces existing. There could be only one loader of a given type
    /// at the same time. You can use this method to replace resource loaders with your own loaders.
    pub fn set<T>(&mut self, loader: T) -> Option<T>
    where
        T: ResourceLoader,
    {
        if let Some(existing_loader) = self
            .loaders
            .iter_mut()
            .find_map(|l| (&mut **l as &mut dyn Any).downcast_mut::<T>())
        {
            Some(std::mem::replace(existing_loader, loader))
        } else {
            self.loaders.push(Box::new(loader));
            None
        }
    }

    /// Searches for an instance of a resource loader of type `Prev` and replaces it with an other instance
    /// of a type `New`.
    pub fn try_replace<Prev, New>(&mut self, new_loader: New) -> Option<Prev>
    where
        Prev: ResourceLoader,
        New: ResourceLoader,
    {
        if let Some(pos) = self
            .loaders
            .iter()
            .position(|l| (&**l as &dyn Any).is::<Prev>())
        {
            let prev_untyped = std::mem::replace(&mut self.loaders[pos], Box::new(new_loader));
            (prev_untyped as Box<dyn Any>)
                .downcast::<Prev>()
                .ok()
                .map(|boxed| *boxed)
        } else {
            None
        }
    }

    /// Tries to find an instance of a resource loader of the given type `T.
    pub fn find<T>(&self) -> Option<&T>
    where
        T: ResourceLoader,
    {
        self.loaders
            .iter()
            .find_map(|loader| (&**loader as &dyn Any).downcast_ref())
    }

    /// Tries to find an instance of a resource loader of the given type `T.
    pub fn find_mut<T>(&mut self) -> Option<&mut T>
    where
        T: ResourceLoader,
    {
        self.loaders
            .iter_mut()
            .find_map(|loader| (&mut **loader as &mut dyn Any).downcast_mut())
    }

    /// Returns total amount of resource loaders in the container.
    pub fn len(&self) -> usize {
        self.loaders.len()
    }

    /// Return `true` if the container contains no resource loaders.
    pub fn is_empty(&self) -> bool {
        self.loaders.is_empty()
    }

    /// Returns an iterator yielding shared references to "untyped" resource loaders.
    pub fn iter(&self) -> impl Iterator<Item = &dyn ResourceLoader> {
        self.loaders.iter().map(|boxed| &**boxed)
    }

    /// Returns an iterator yielding mutable references to "untyped" resource loaders.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut dyn ResourceLoader> {
        self.loaders.iter_mut().map(|boxed| &mut **boxed)
    }

    /// Returns `true` if there's at least one resource loader, that supports the extension of the
    /// file at the given path.
    pub fn is_supported_resource(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            self.loaders
                .iter()
                .any(|loader| loader.supports_extension(extension))
        } else {
            false
        }
    }

    /// Checks if there's a resource loader for the given path and the data type produced by the
    /// loader matches the given type `T`.
    pub fn is_extension_matches_type<T>(&self, path: &Path) -> bool
    where
        T: TypedResourceData,
    {
        path.extension().is_some_and(|extension| {
            self.loaders
                .iter()
                .find(|loader| loader.supports_extension(&extension.to_string_lossy()))
                .is_some_and(|loader| {
                    loader.data_type_uuid() == <T as TypeUuidProvider>::type_uuid()
                })
        })
    }

    /// Checks if there's a loader for the given path.
    pub fn loader_for(&self, path: &Path) -> Option<&dyn ResourceLoader> {
        path.extension().and_then(|extension| {
            self.loaders
                .iter()
                .find(|loader| loader.supports_extension(&extension.to_string_lossy()))
                .map(|l| &**l)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Eq, PartialEq, Debug)]
    struct MyResourceLoader;

    impl ResourceLoader for MyResourceLoader {
        fn extensions(&self) -> &[&str] {
            &[]
        }

        fn data_type_uuid(&self) -> Uuid {
            Default::default()
        }

        fn load(&self, _path: PathBuf, _io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
            todo!()
        }
    }

    #[test]
    fn resource_loader_container_new() {
        let container = ResourceLoadersContainer::new();
        assert!(container.loaders.is_empty());

        let container = ResourceLoadersContainer::default();
        assert!(container.loaders.is_empty());
    }

    #[test]
    fn resource_loader_container_set() {
        let mut container = ResourceLoadersContainer::new();
        let res = container.set(MyResourceLoader);
        let res2 = container.set(MyResourceLoader);
        assert_eq!(res, None);
        assert_eq!(res2, Some(MyResourceLoader));

        assert_eq!(container.len(), 1);
    }

    #[test]
    fn resource_loader_container_find() {
        let mut container = ResourceLoadersContainer::new();

        let res = container.find::<MyResourceLoader>();
        assert_eq!(res, None);

        container.set(MyResourceLoader);
        let res = container.find::<MyResourceLoader>();

        assert_eq!(res, Some(&MyResourceLoader));
    }

    #[test]
    fn resource_loader_container_find_mut() {
        let mut container = ResourceLoadersContainer::new();

        let res = container.find_mut::<MyResourceLoader>();
        assert_eq!(res, None);

        container.set(MyResourceLoader);
        let res = container.find_mut::<MyResourceLoader>();

        assert_eq!(res, Some(&mut MyResourceLoader));
    }

    #[test]
    fn resource_loader_container_getters() {
        let mut container = ResourceLoadersContainer::new();
        assert!(container.is_empty());
        assert_eq!(container.len(), 0);

        container.set(MyResourceLoader);
        container.set(MyResourceLoader);
        assert!(!container.is_empty());
        assert_eq!(container.len(), 1);
    }
}
