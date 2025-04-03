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

//! A module for untyped resources. See [`UntypedResource`] docs for more info.

use crate::{
    core::{
        math::curve::Curve, parking_lot::Mutex, reflect::prelude::*, uuid, uuid::Uuid,
        visitor::prelude::*, visitor::RegionGuard, TypeUuidProvider,
    },
    manager::ResourceManager,
    state::{LoadError, ResourceState},
    Resource, ResourceData, ResourceLoadError, TypedResourceData, CURVE_RESOURCE_UUID,
    MODEL_RESOURCE_UUID, SHADER_RESOURCE_UUID, SOUND_BUFFER_RESOURCE_UUID, TEXTURE_RESOURCE_UUID,
};
use std::{
    error::Error,
    ffi::OsStr,
    fmt::{Debug, Display, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

// Heuristic function to guess resource uuid based on inner content of a resource.
fn guess_uuid(region: &mut RegionGuard) -> Uuid {
    assert!(region.is_reading());

    let mut region = region.enter_region("Details").unwrap();

    let mut mip_count = 0u32;
    if mip_count.visit("MipCount", &mut region).is_ok() {
        return TEXTURE_RESOURCE_UUID;
    }

    let mut curve = Curve::default();
    if curve.visit("Curve", &mut region).is_ok() {
        return CURVE_RESOURCE_UUID;
    }

    let mut id = 0u32;
    if id.visit("Id", &mut region).is_ok() {
        return SOUND_BUFFER_RESOURCE_UUID;
    }

    let mut path = PathBuf::new();
    if path.visit("Path", &mut region).is_ok() {
        let ext = path.extension().unwrap_or_default().to_ascii_lowercase();
        if ext == OsStr::new("rgs")
            || ext == OsStr::new("fbx")
            || ext == OsStr::new("gltf")
            || ext == OsStr::new("glb")
        {
            return MODEL_RESOURCE_UUID;
        } else if ext == OsStr::new("shader")
            || path == OsStr::new("Standard")
            || path == OsStr::new("StandardTwoSides")
            || path == OsStr::new("StandardTerrain")
        {
            return SHADER_RESOURCE_UUID;
        }
    }

    Default::default()
}

/// Kind of a resource. It defines how the resource manager will treat a resource content on serialization.
#[derive(Default, Reflect, Debug, Visit, Clone, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// The content of embedded resources will be fully serialized.
    #[default]
    Embedded,
    /// The content of external resources will not be serialized, instead only the path to the content
    /// will be serialized and the content will be loaded from it when needed.
    ///
    /// ## Built-in Resources
    ///
    /// This resource kind could also be used to create built-in resources (the data of which is
    /// embedded directly in the executable using [`include_bytes`] macro). All that is needed is to
    /// create a static resource variable and register it in built-in resources of the resource manager.
    /// In this case, the path becomes an identifier and it must be unique. See [`ResourceManager`] docs
    /// for more info about built-in resources.
    External(PathBuf),
}

impl From<Option<PathBuf>> for ResourceKind {
    fn from(value: Option<PathBuf>) -> Self {
        match value {
            None => Self::Embedded,
            Some(path) => Self::External(path),
        }
    }
}

impl From<PathBuf> for ResourceKind {
    fn from(value: PathBuf) -> Self {
        Self::External(value)
    }
}

impl<'a> From<&'a str> for ResourceKind {
    fn from(value: &'a str) -> Self {
        Self::External(value.into())
    }
}

impl ResourceKind {
    /// Switches the resource kind to [`Self::External`].
    #[inline]
    pub fn make_external(&mut self, path: PathBuf) {
        *self = Self::External(path);
    }

    /// Switches the resource kind to [`Self::Embedded`]
    #[inline]
    pub fn make_embedded(&mut self) {
        *self = Self::Embedded;
    }

    /// Checks, if the resource kind is [`Self::Embedded`]
    #[inline]
    pub fn is_embedded(&self) -> bool {
        matches!(self, Self::Embedded)
    }

    /// Checks, if the resource kind is [`Self::External`]
    #[inline]
    pub fn is_external(&self) -> bool {
        !self.is_embedded()
    }

    /// Tries to fetch a resource path, returns [`None`] for [`Self::Embedded`] resources.
    #[inline]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Embedded => None,
            Self::External(path) => Some(path),
        }
    }

    /// Tries to fetch a resource path, returns [`None`] for [`Self::Embedded`] resources.
    #[inline]
    pub fn path_owned(&self) -> Option<PathBuf> {
        self.path().map(|p| p.to_path_buf())
    }

    /// Tries to convert the resource kind into its path, returns [`None`] for [`Self::Embedded`]
    /// resources.
    #[inline]
    pub fn into_path(self) -> Option<PathBuf> {
        match self {
            Self::Embedded => None,
            Self::External(path) => Some(path),
        }
    }
}

impl Display for ResourceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Embedded => {
                write!(f, "Embedded")
            }
            Self::External(path) => {
                write!(f, "External ({})", path.display())
            }
        }
    }
}

/// Header of a resource, it contains a common data about the resource, such as its data type uuid,
/// its kind, etc.
#[derive(Reflect, Debug)]
pub struct ResourceHeader {
    /// Unique id of a resource. It is controlled strictly by a [`ResourceManager`] instance.
    pub resource_uuid: Uuid,
    /// UUID of the internal data type.
    pub type_uuid: Uuid,
    /// Kind of the resource. See [`ResourceKind`] for more info.
    pub kind: ResourceKind,
    /// Actual state of the resource. See [`ResourceState`] for more info.
    pub state: ResourceState,
}

impl Visit for ResourceHeader {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.is_reading() {
            let mut id: u32 = 0;

            if id.visit("Id", &mut region).is_ok() {
                // Reading old version, convert it to the new.

                let mut type_uuid = Uuid::default();
                if type_uuid.visit("TypeUuid", &mut region).is_err() {
                    // We might be reading the old version, try to guess an actual type uuid by
                    // the inner content of the resource data.
                    type_uuid = guess_uuid(&mut region);
                };

                // We're interested only in embedded resources.
                if id == 2 {
                    let resource_manager = region.blackboard.get::<ResourceManager>().expect(
                        "Resource data constructor container must be \
                provided when serializing resources!",
                    );
                    let resource_manager_state = resource_manager.state();

                    if let Some(mut instance) = resource_manager_state
                        .constructors_container
                        .try_create(&type_uuid)
                    {
                        drop(resource_manager_state);

                        if let Ok(mut details_region) = region.enter_region("Details") {
                            if type_uuid == SOUND_BUFFER_RESOURCE_UUID {
                                let mut sound_region = details_region.enter_region("0")?;
                                let mut path = PathBuf::new();
                                path.visit("Path", &mut sound_region).unwrap();
                                self.kind.make_external(path);
                            } else {
                                let mut path = PathBuf::new();
                                path.visit("Path", &mut details_region).unwrap();
                                self.kind.make_external(path);
                            }
                        }

                        instance.visit("Details", &mut region)?;

                        self.state = ResourceState::Ok(instance);

                        return Ok(());
                    } else {
                        return Err(VisitError::User(format!(
                            "There's no constructor registered for type {type_uuid}!"
                        )));
                    }
                } else {
                    self.state = ResourceState::LoadError {
                        error: LoadError::new("Old resource"),
                    };
                }

                return Ok(());
            }
        }

        self.kind.visit("Kind", &mut region)?;
        self.type_uuid.visit("TypeUuid", &mut region)?;

        if self.kind == ResourceKind::Embedded {
            self.state.visit("State", &mut region)?;
        }

        Ok(())
    }
}

/// Untyped resource is a universal way of storing arbitrary resource types. Internally it wraps
/// [`ResourceState`] in a `Arc<Mutex<>` so the untyped resource becomes shareable. In most of the
/// cases you don't need to deal with untyped resources, use typed [`Resource`] wrapper instead.
/// Untyped resource could be useful in cases when you need to collect a set of resources of different
/// types in a single collection and do something with them.
///
/// ## Handle
///
/// Since untyped resources stores the actual data in a shared storage, the resource instance could
/// be considered as a handle. Such "handles" have special behaviour on serialization and
/// deserialization to keep pointing to the same storage.
///
/// ## Serialization and Deserialization
///
/// Every resource writes its own kind, type uuid of the data and optionally the data itself.
///
/// Serialization/deserialization of the data is different depending on the actual resource kind
/// (see [`ResourceKind`]):
///
/// 1) [`ResourceKind::Embedded`] - the resource data will be serialized together with the resource
/// handle. The data will be loaded back on deserialization stage from the backing storage.
/// 2) [`ResourceKind::External`] - the resource data won't be serialized and will be reloaded from
/// the external source.
///
/// When the resource is deserialized, the resource system at first looks for an already loaded
/// resource with the same kind and if it is found, replaces current instance with the loaded one.
/// If not - loads the resource and also replaces the instance. This step is crucial for uniqueness
/// of the resource handles.
///
/// To put everything simple: when you save a resource handle, it writes only path to it, then when
/// you load it you need to make sure that all references to a resource points to the same resource
/// instance.
///
/// ## Default state
///
/// Default state of every untyped resource is [`ResourceState::LoadError`] with a warning message,
/// that the resource is in default state. This is a trade-off to prevent wrapping internals into
/// `Option`, that in some cases could lead to convoluted code with lots of `unwrap`s and state
/// assumptions.
#[derive(Clone, Reflect, TypeUuidProvider)]
#[type_uuid(id = "21613484-7145-4d1c-87d8-62fa767560ab")]
pub struct UntypedResource(pub Arc<Mutex<ResourceHeader>>);

impl Visit for UntypedResource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)?;

        // Try to restore the shallow handle on deserialization for external resources.
        if visitor.is_reading() && !self.is_embedded() {
            let resource_manager = visitor
                .blackboard
                .get::<ResourceManager>()
                .expect("Resource manager must be available when deserializing resources!");

            let path = self.kind().path_owned().unwrap();
            self.0 = resource_manager.request_untyped(path).0;
        }

        Ok(())
    }
}

impl Default for UntypedResource {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ResourceHeader {
            resource_uuid: Default::default(),
            kind: Default::default(),
            type_uuid: Default::default(),
            state: ResourceState::new_load_error(LoadError::new(
                "Default resource state of unknown type.",
            )),
        })))
    }
}

impl Debug for UntypedResource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Resource")
    }
}

impl PartialEq for UntypedResource {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.0, &*other.0)
    }
}

impl Eq for UntypedResource {}

impl Hash for UntypedResource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(&*self.0 as *const _ as u64)
    }
}

impl UntypedResource {
    /// Creates new untyped resource in pending state using the given path and type uuid.
    pub fn new_pending(kind: ResourceKind, type_uuid: Uuid) -> Self {
        Self(Arc::new(Mutex::new(ResourceHeader {
            resource_uuid: Default::default(),
            kind,
            type_uuid,
            state: ResourceState::new_pending(),
        })))
    }

    /// Creates new untyped resource in ok (fully loaded) state using the given data of any type, that
    /// implements [`ResourceData`] trait.
    pub fn new_ok<T>(kind: ResourceKind, data: T) -> Self
    where
        T: ResourceData,
    {
        Self(Arc::new(Mutex::new(ResourceHeader {
            resource_uuid: Default::default(),
            kind,
            type_uuid: data.type_uuid(),
            state: ResourceState::new_ok(data),
        })))
    }

    /// Creates new untyped resource in error state.
    pub fn new_load_error(kind: ResourceKind, error: LoadError, type_uuid: Uuid) -> Self {
        Self(Arc::new(Mutex::new(ResourceHeader {
            resource_uuid: Default::default(),
            kind,
            type_uuid,
            state: ResourceState::new_load_error(error),
        })))
    }

    /// Returns actual unique type id of underlying resource data.
    pub fn type_uuid(&self) -> Uuid {
        self.0.lock().type_uuid
    }

    /// Returns true if the resource is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(self.0.lock().state, ResourceState::Pending { .. })
    }

    /// Returns true if the resource is procedural (its data is generated at runtime, not stored in an external
    /// file).
    pub fn is_embedded(&self) -> bool {
        self.0.lock().kind.is_embedded()
    }

    /// Returns exact amount of users of the resource.
    #[inline]
    pub fn use_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    #[inline]
    pub fn key(&self) -> usize {
        (&*self.0 as *const _) as usize
    }

    /// Returns path of the untyped resource.
    pub fn kind(&self) -> ResourceKind {
        self.0.lock().kind.clone()
    }

    /// Set a new path for the untyped resource.
    pub fn set_kind(&self, new_kind: ResourceKind) {
        self.0.lock().kind = new_kind;
    }

    /// Tries to save the resource to the specified path.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut guard = self.0.lock();
        match guard.state {
            ResourceState::Pending { .. } | ResourceState::LoadError { .. } => {
                Err("Unable to save unloaded resource!".into())
            }
            ResourceState::Ok(ref mut data) => data.save(path),
        }
    }

    /// Tries to save the resource back to its external location. This method will fail on attempt
    /// to save embedded resource, because embedded resources does not have external location.
    pub fn save_back(&self) -> Result<(), Box<dyn Error>> {
        match self.kind() {
            ResourceKind::Embedded => Err("Embedded resource cannot be saved!".into()),
            ResourceKind::External(path) => self.save(&path),
        }
    }

    /// Tries to cast untyped resource to a particular type.
    pub fn try_cast<T>(&self) -> Option<Resource<T>>
    where
        T: TypedResourceData,
    {
        if self.type_uuid() == <T as TypeUuidProvider>::type_uuid() {
            Some(Resource {
                untyped: self.clone(),
                phantom: PhantomData::<T>,
            })
        } else {
            None
        }
    }

    /// Changes ResourceState::Pending state to ResourceState::Ok(data) with given `data`.
    /// Additionally, it wakes all futures.
    #[inline]
    pub fn commit(&self, state: ResourceState) {
        self.0.lock().state.commit(state);
    }

    /// Changes internal state to [`ResourceState::Ok`]
    pub fn commit_ok<T: ResourceData>(&self, data: T) {
        let mut guard = self.0.lock();
        guard.type_uuid = data.type_uuid();
        guard.state.commit_ok(data);
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&self, error: E) {
        self.0.lock().state.commit_error(error);
    }
}

impl Future for UntypedResource {
    type Output = Result<Self, LoadError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.0.clone();
        let mut guard = state.lock();
        match guard.state {
            ResourceState::Pending { ref mut wakers, .. } => {
                // Collect wakers, so we'll be able to wake task when worker thread finish loading.
                let cx_waker = cx.waker();
                if let Some(pos) = wakers.iter().position(|waker| waker.will_wake(cx_waker)) {
                    wakers[pos].clone_from(cx_waker);
                } else {
                    wakers.push(cx_waker.clone())
                }

                Poll::Pending
            }
            ResourceState::LoadError { ref error, .. } => Poll::Ready(Err(error.clone())),
            ResourceState::Ok(_) => Poll::Ready(Ok(self.clone())),
        }
    }
}

#[cfg(test)]
mod test {
    use futures::task::noop_waker;
    use fyrox_core::futures;
    use std::error::Error;
    use std::task::{self};

    use super::*;

    #[derive(Debug, Default, Reflect, Visit, Clone, Copy)]
    struct Stub {}

    impl ResourceData for Stub {
        fn type_uuid(&self) -> Uuid {
            Uuid::default()
        }

        fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
            Err("Saving is not supported!".to_string().into())
        }

        fn can_be_saved(&self) -> bool {
            false
        }
    }

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            Uuid::default()
        }
    }

    impl ResourceLoadError for str {}

    #[test]
    fn visit_for_untyped_resource() {
        let mut r = UntypedResource::default();
        let mut visitor = Visitor::default();

        assert!(r.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn debug_for_untyped_resource() {
        let r = UntypedResource::default();

        assert_eq!(format!("{r:?}"), "Resource\n");
    }

    #[test]
    fn untyped_resource_new_pending() {
        let r = UntypedResource::new_pending(PathBuf::from("/foo").into(), Uuid::default());

        assert_eq!(r.0.lock().type_uuid, Uuid::default());
        assert_eq!(
            r.0.lock().kind,
            ResourceKind::External(PathBuf::from("/foo"))
        );
    }

    #[test]
    fn untyped_resource_new_load_error() {
        let r = UntypedResource::new_load_error(
            PathBuf::from("/foo").into(),
            Default::default(),
            Uuid::default(),
        );

        assert_eq!(r.0.lock().type_uuid, Uuid::default());
        assert_eq!(
            r.0.lock().kind,
            ResourceKind::External(PathBuf::from("/foo"))
        );
    }

    #[test]
    fn untyped_resource_use_count() {
        let r = UntypedResource::default();

        assert_eq!(r.use_count(), 1);
    }

    #[test]
    fn untyped_resource_try_cast() {
        let r = UntypedResource::default();
        let r2 = UntypedResource::new_pending(
            PathBuf::from("/foo").into(),
            Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128),
        );

        assert!(r.try_cast::<Stub>().is_some());
        assert!(r2.try_cast::<Stub>().is_none());
    }

    #[test]
    fn untyped_resource_commit() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        let r = UntypedResource::new_pending(path.clone().into(), Default::default());
        assert_eq!(r.0.lock().kind, ResourceKind::External(path.clone()));

        r.commit(ResourceState::Ok(Box::new(stub)));
        assert_eq!(r.0.lock().kind, ResourceKind::External(path));
    }

    #[test]
    fn untyped_resource_commit_ok() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        let r = UntypedResource::new_pending(path.clone().into(), Default::default());
        assert_eq!(r.0.lock().kind, ResourceKind::External(path.clone()));

        r.commit_ok(stub);
        assert_eq!(r.0.lock().kind, ResourceKind::External(path));
    }

    #[test]
    fn untyped_resource_commit_error() {
        let path = PathBuf::from("/foo");
        let path2 = PathBuf::from("/bar");

        let r = UntypedResource::new_pending(path.clone().into(), Default::default());
        assert_eq!(r.0.lock().kind, ResourceKind::External(path));
        assert_ne!(r.0.lock().kind, ResourceKind::External(path2));
    }

    #[test]
    fn untyped_resource_poll() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        let waker = noop_waker();
        let mut cx = task::Context::from_waker(&waker);

        let mut r = UntypedResource(Arc::new(Mutex::new(ResourceHeader {
            resource_uuid: Default::default(),
            kind: path.clone().into(),
            type_uuid: Uuid::default(),
            state: ResourceState::Ok(Box::new(stub)),
        })));
        assert!(Pin::new(&mut r).poll(&mut cx).is_ready());

        let mut r = UntypedResource(Arc::new(Mutex::new(ResourceHeader {
            resource_uuid: Default::default(),
            kind: path.clone().into(),
            type_uuid: Uuid::default(),
            state: ResourceState::LoadError {
                error: Default::default(),
            },
        })));
        assert!(Pin::new(&mut r).poll(&mut cx).is_ready());
    }
}
