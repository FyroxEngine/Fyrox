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

//! Built-in resource is a resource embedded in the application executable file. It is a very useful
//! mechanism when you need to bundle all game resources and put them in the executable file. See
//! [`BuiltInResource`] docs for more info.

use crate::{core::Uuid, untyped::UntypedResource, Resource, TypedResourceData};
use fxhash::FxHashMap;
use std::{
    borrow::Cow,
    ops::Deref,
    path::{Path, PathBuf},
};

/// Data source of a built-in resource.
#[derive(Clone)]
pub struct DataSource {
    /// File extension, associated with the data source.
    pub extension: Cow<'static, str>,
    /// The actual data.
    pub bytes: Cow<'static, [u8]>,
}

impl DataSource {
    /// Creates a new data source with the given path and the data. The data must be embedded in
    /// the application binary using [`include_bytes`] macro or similar. Use [`crate::embedded_data_source`]
    /// macro to combine a call to this method with [`include_bytes`] macro.
    pub fn new(path: &'static str, data: &'static [u8]) -> Self {
        Self {
            extension: Cow::Borrowed(
                Path::new(path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or(""),
            ),
            bytes: Cow::Borrowed(data),
        }
    }
}

/// Combines a call of [`include_bytes`] with the call of [`DataSource::new`]. Prevents you from
/// typing the same path twice.
#[macro_export]
macro_rules! embedded_data_source {
    ($path:expr) => {
        $crate::manager::DataSource::new($path, include_bytes!($path))
    };
}

/// Untyped built-in resource.
#[derive(Clone)]
pub struct UntypedBuiltInResource {
    /// Id of the built-in resource.
    pub id: PathBuf,
    /// Initial data, from which the resource is created from.
    pub data_source: Option<DataSource>,
    /// Uuid of the resource.
    pub resource_uuid: Uuid,
    /// Ready-to-use ("loaded") resource.
    pub resource: UntypedResource,
}

/// Built-in resource is a resource embedded in the application executable file. It is a very useful
/// mechanism when you need to bundle all game resources and put them in the executable file.
///
/// ## Registration
///
/// Every built-in resource must be registered in a resource manager to be accessible via standard
/// [`crate::manager::ResourceManager::request`] method. It could be done pretty easily:
///
/// ```rust
/// use fyrox_resource::{
///     builtin::BuiltInResource,
///     core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*, Uuid},
///     manager::ResourceManager,
///     Resource, ResourceData,
/// };
/// use std::{error::Error, path::Path};
///
/// #[derive(TypeUuidProvider, Default, Debug, Clone, Visit, Reflect)]
/// #[type_uuid(id = "00d036bb-fbed-47f7-94e3-b3fce93dee17")]
/// struct MyResource {
///     some_data: String,
/// }
///
/// impl ResourceData for MyResource {
///     fn type_uuid(&self) -> Uuid {
///         <Self as TypeUuidProvider>::type_uuid()
///     }
///
///     fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
///         Ok(())
///     }
///
///     fn can_be_saved(&self) -> bool {
///         false
///     }
///
///     fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
///         Some(Box::new(self.clone()))
///     }
/// }
///
/// fn register_built_in_resource(resource_manager: &ResourceManager) {
///     let id = "MyResourceId";
///     let some_data = "This string is a built-in resource with MyResourceId id.";
///
///     let resource = Resource::new_embedded(MyResource {
///         some_data: some_data.into(),
///     });
///
///     resource_manager.register_built_in_resource(BuiltInResource::new_no_source(id, resource));
///
///     assert_eq!(
///         resource_manager
///             .request::<MyResource>(id)
///             .data_ref()
///             .some_data,
///         some_data,
///     )
/// }
/// ```
pub struct BuiltInResource<T>
where
    T: TypedResourceData,
{
    /// Id of the built-in resource.
    pub id: PathBuf,
    /// Initial data, from which the resource is created from.
    pub data_source: Option<DataSource>,
    /// Ready-to-use ("loaded") resource.
    pub resource: Resource<T>,
}

impl<T: TypedResourceData> Clone for BuiltInResource<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            data_source: self.data_source.clone(),
            resource: self.resource.clone(),
        }
    }
}

impl<T: TypedResourceData> BuiltInResource<T> {
    /// Creates a new built-in resource with an id, a data source and function that creates the
    /// resource from the given data source.
    pub fn new<F>(id: impl AsRef<Path>, data_source: DataSource, make: F) -> Self
    where
        F: FnOnce(&[u8]) -> Resource<T>,
    {
        let resource = make(&data_source.bytes);
        Self {
            id: id.as_ref().to_path_buf(),
            resource,
            data_source: Some(data_source),
        }
    }

    /// Creates a new built-in resource from an id and arbitrary resource.
    pub fn new_no_source(id: impl AsRef<Path>, resource: Resource<T>) -> Self {
        Self {
            id: id.as_ref().to_path_buf(),
            data_source: None,
            resource,
        }
    }

    /// Returns the wrapped resource instance.
    pub fn resource(&self) -> Resource<T> {
        self.resource.clone()
    }
}

impl<T: TypedResourceData> From<BuiltInResource<T>> for UntypedBuiltInResource {
    fn from(value: BuiltInResource<T>) -> Self {
        Self {
            id: value.id,
            data_source: value.data_source,
            resource_uuid: value.resource.resource_uuid(),
            resource: value.resource.into(),
        }
    }
}

/// A container for built-in resources. Every built-in resource is registered using its id defined
/// at the creation stage.
#[derive(Default, Clone)]
pub struct BuiltInResourcesContainer {
    inner: FxHashMap<PathBuf, UntypedBuiltInResource>,
}

impl BuiltInResourcesContainer {
    /// Adds a new typed built-in resource and removes the previous one with the same id (if any).
    pub fn add<T>(&mut self, resource: BuiltInResource<T>) -> Option<UntypedBuiltInResource>
    where
        T: TypedResourceData,
    {
        self.add_untyped(resource.into())
    }

    /// Adds a new untyped built-in resource and removes the previous one with the same id (if any).
    pub fn add_untyped(
        &mut self,
        resource: UntypedBuiltInResource,
    ) -> Option<UntypedBuiltInResource> {
        let id = resource.id.clone();
        self.inner.insert(id, resource)
    }

    /// Tries to remove a built-in resource by its path.
    pub fn remove(&mut self, id: impl AsRef<Path>) -> Option<UntypedBuiltInResource> {
        self.inner.remove(id.as_ref())
    }

    /// Tries to find a built-in resource by its uuid.
    pub fn find_by_uuid(&self, uuid: Uuid) -> Option<&UntypedBuiltInResource> {
        self.inner.values().find(|r| r.resource_uuid == uuid)
    }

    /// Checks whether the given resource path corresponds to a built-in resource or not.
    pub fn is_built_in_resource_path(&self, resource: impl AsRef<Path>) -> bool {
        self.inner.contains_key(resource.as_ref())
    }

    /// Checks whether the given resource is a built-in resource instance or not.
    pub fn is_built_in_resource(&self, resource: impl AsRef<UntypedResource>) -> bool {
        self.inner
            .values()
            .any(|built_in| &built_in.resource == resource.as_ref())
    }
}

impl Deref for BuiltInResourcesContainer {
    type Target = FxHashMap<PathBuf, UntypedBuiltInResource>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
