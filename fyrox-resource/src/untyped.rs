//! A module for untyped resources. See [`UntypedResource`] docs for more info.

use crate::state::LoadError;
use crate::{
    core::{
        log::Log, parking_lot::Mutex, reflect::prelude::*, uuid::Uuid, visitor::prelude::*,
        TypeUuidProvider,
    },
    manager::ResourceManager,
    state::ResourceState,
    Resource, ResourceData, ResourceLoadError, TypedResourceData,
};
use std::{
    fmt::{Debug, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// Untyped resource is a universal way of storing arbitrary resource types. Internally it wraps
/// [`ResourceState`] in a `Arc<Mutex<>` so the untyped resource becomes shareable. In most of the
/// cases you don't need to deal with untyped resources, use typed [`Resource`] wrapper instead.
/// Untyped resource could be useful in cases when you need to collect a set resources of different
/// types in a single collection and do something with them.
///
/// ## Default state
///
/// Default state of every untyped resource is [`ResourceState::LoadError`] with a warning message,
/// that the resource is in default state. This is a trade-off to prevent wrapping internals into
/// `Option`, that in some cases could lead to convoluted code with lots of `unwrap`s and state
/// assumptions.
#[derive(Clone, Reflect)]
pub struct UntypedResource(pub Arc<Mutex<ResourceState>>);

impl Visit for UntypedResource {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)?;

        if visitor.is_reading() {
            // Try to restore the shallow handle.
            let resource_manager = visitor
                .blackboard
                .get::<ResourceManager>()
                .expect("Resource manager must be available when deserializing resources!");

            let path = self.path();

            // There might be a built-in resource, in this case we must restore the "reference" to it.
            let state = resource_manager.state();
            if let Some(built_in_resource) = state.built_in_resources.get(&path) {
                if built_in_resource.type_uuid() == self.type_uuid() {
                    self.0 = built_in_resource.clone().0;
                } else {
                    Log::err(format!(
                        "Built in resource {:?} has changed its type and cannot be restored!",
                        path
                    ));
                }
            } else {
                drop(state);
                let is_procedural = self.is_procedural();
                if !is_procedural {
                    self.0 = resource_manager.request_untyped(path).0;
                }
            }
        }

        Ok(())
    }
}

impl Default for UntypedResource {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_load_error(
            Default::default(),
            LoadError::new("Default resource state of unknown type."),
            Default::default(),
        ))))
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
    pub fn new_pending(path: PathBuf, type_uuid: Uuid) -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_pending(
            path, type_uuid,
        ))))
    }

    /// Creates new untyped resource in ok (fully loaded) state using the given data of any type, that
    /// implements [`ResourceData`] trait.
    pub fn new_ok<T: ResourceData>(data: T) -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_ok(data))))
    }

    /// Creates new untyped resource in error state.
    pub fn new_load_error(path: PathBuf, error: LoadError, type_uuid: Uuid) -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_load_error(
            path, error, type_uuid,
        ))))
    }

    /// Returns actual unique type id of underlying resource data.
    pub fn type_uuid(&self) -> Uuid {
        self.0.lock().type_uuid()
    }

    /// Returns true if the resource is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(*self.0.lock(), ResourceState::Pending { .. })
    }

    /// Returns true if the resource is procedural (its data is generated at runtime, not stored in an external
    /// file).
    pub fn is_procedural(&self) -> bool {
        match *self.0.lock() {
            ResourceState::Ok(ref data) => data.is_procedural(),
            // Procedural resources must always be in Ok state.
            _ => false,
        }
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
    pub fn path(&self) -> PathBuf {
        match &*self.0.lock() {
            ResourceState::Pending { path, .. } => path.clone(),
            ResourceState::LoadError { path, .. } => path.clone(),
            ResourceState::Ok(data) => data.path().to_path_buf(),
        }
    }

    /// Set a new path for the untyped resource.
    pub fn set_path(&self, new_path: PathBuf) {
        match &mut *self.0.lock() {
            ResourceState::Pending { path, .. } => {
                *path = new_path;
            }
            ResourceState::LoadError { path, .. } => {
                *path = new_path;
            }
            ResourceState::Ok(data) => {
                data.set_path(new_path);
            }
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
    /// Additionally it wakes all futures.
    #[inline]
    pub fn commit(&self, state: ResourceState) {
        self.0.lock().commit(state);
    }

    /// Changes internal state to [`ResourceState::Ok`]
    pub fn commit_ok<T: ResourceData>(&self, data: T) {
        self.0.lock().commit_ok(data);
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&self, path: PathBuf, error: E) {
        self.0.lock().commit_error(path, error);
    }
}

impl Future for UntypedResource {
    type Output = Result<Self, LoadError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.0.clone();
        let mut guard = state.lock();
        match *guard {
            ResourceState::Pending { ref mut wakers, .. } => {
                // Collect wakers, so we'll be able to wake task when worker thread finish loading.
                let cx_waker = cx.waker();
                if let Some(pos) = wakers.iter().position(|waker| waker.will_wake(cx_waker)) {
                    wakers[pos] = cx_waker.clone();
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
    use std::{
        path::Path,
        task::{self},
    };

    use super::*;

    #[derive(Debug, Default, Reflect, Visit, Clone, Copy)]
    struct Stub {}

    impl ResourceData for Stub {
        fn path(&self) -> std::borrow::Cow<std::path::Path> {
            std::borrow::Cow::Borrowed(Path::new(""))
        }

        fn set_path(&mut self, _path: std::path::PathBuf) {
            unimplemented!()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            unimplemented!()
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            unimplemented!()
        }

        fn type_uuid(&self) -> Uuid {
            Uuid::default()
        }

        fn is_procedural(&self) -> bool {
            unimplemented!()
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
        let r = UntypedResource::new_pending(PathBuf::from("/foo"), Uuid::default());

        assert_eq!(r.0.lock().type_uuid(), Uuid::default());
        assert_eq!(r.0.lock().path(), PathBuf::from("/foo"));
    }

    #[test]
    fn untyped_resource_new_load_error() {
        let r = UntypedResource::new_load_error(PathBuf::from("/foo"), None, Uuid::default());

        assert_eq!(r.0.lock().type_uuid(), Uuid::default());
        assert_eq!(r.0.lock().path(), PathBuf::from("/foo"));
    }

    #[test]
    fn untyped_resource_new_ok() {
        let s = Stub {};
        let r = UntypedResource::new_ok(s);

        assert_eq!(r.0.lock().type_uuid(), s.type_uuid());
        assert_eq!(r.0.lock().path(), s.path());
    }

    #[test]
    fn untyped_resource_is_loading() {
        assert!(UntypedResource(Arc::new(Mutex::new(ResourceState::Pending {
            path: PathBuf::from("/foo"),
            wakers: Vec::new(),
            type_uuid: Uuid::default()
        })))
        .is_loading());

        assert!(
            !UntypedResource(Arc::new(Mutex::new(ResourceState::LoadError {
                path: PathBuf::from("/foo"),
                error: None,
                type_uuid: Uuid::default()
            })))
            .is_loading()
        );

        assert!(
            !UntypedResource(Arc::new(Mutex::new(ResourceState::Ok(Box::new(Stub {})))))
                .is_loading()
        );
    }

    #[test]
    fn untyped_resource_use_count() {
        let r = UntypedResource::default();

        assert_eq!(r.use_count(), 1);
    }

    #[test]
    fn untyped_resource_path() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        assert_eq!(
            UntypedResource(Arc::new(Mutex::new(ResourceState::Pending {
                path: path.clone(),
                wakers: Vec::new(),
                type_uuid: Uuid::default()
            })))
            .path(),
            path
        );

        assert_eq!(
            UntypedResource(Arc::new(Mutex::new(ResourceState::LoadError {
                path: path.clone(),
                error: None,
                type_uuid: Uuid::default()
            })))
            .path(),
            path
        );

        assert_eq!(
            UntypedResource(Arc::new(Mutex::new(ResourceState::Ok(Box::new(stub))))).path(),
            stub.path(),
        );
    }

    #[test]
    fn untyped_resource_try_cast() {
        let r = UntypedResource::default();
        let r2 = UntypedResource::new_pending(
            PathBuf::from("/foo"),
            Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128),
        );

        assert!(r.try_cast::<Stub>().is_some());
        assert!(r2.try_cast::<Stub>().is_none());
    }

    #[test]
    fn untyped_resource_commit() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        let r = UntypedResource::new_pending(path.clone(), Default::default());
        assert_eq!(r.0.lock().path(), path);
        assert_ne!(r.0.lock().path(), stub.path());

        r.commit(ResourceState::Ok(Box::new(stub)));
        assert_ne!(r.0.lock().path(), path);
        assert_eq!(r.0.lock().path(), stub.path());
    }

    #[test]
    fn untyped_resource_commit_ok() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        let r = UntypedResource::new_pending(path.clone(), Default::default());
        assert_eq!(r.0.lock().path(), path);
        assert_ne!(r.0.lock().path(), stub.path());

        r.commit_ok(stub);
        assert_ne!(r.0.lock().path(), path);
        assert_eq!(r.0.lock().path(), stub.path());
    }

    #[test]
    fn untyped_resource_commit_error() {
        let path = PathBuf::from("/foo");
        let path2 = PathBuf::from("/bar");

        let r = UntypedResource::new_pending(path.clone(), Default::default());
        assert_eq!(r.0.lock().path(), path);
        assert_ne!(r.0.lock().path(), path2);

        r.commit_error(path2.clone(), "error");
        assert_ne!(r.0.lock().path(), path);
        assert_eq!(r.0.lock().path(), path2);
    }

    #[test]
    fn untyped_resource_poll() {
        let path = PathBuf::from("/foo");
        let stub = Stub {};

        let waker = noop_waker();
        let mut cx = task::Context::from_waker(&waker);

        let mut r = UntypedResource(Arc::new(Mutex::new(ResourceState::Ok(Box::new(stub)))));
        assert!(Pin::new(&mut r).poll(&mut cx).is_ready());

        let mut r = UntypedResource(Arc::new(Mutex::new(ResourceState::LoadError {
            path: path.clone(),
            error: None,
            type_uuid: Uuid::default(),
        })));
        assert!(Pin::new(&mut r).poll(&mut cx).is_ready());
    }

    #[test]
    fn stub_path() {
        let s = Stub {};
        assert_eq!(s.path(), std::borrow::Cow::Borrowed(Path::new("")));
    }

    #[test]
    #[should_panic]
    fn stub_set_path() {
        let mut s = Stub {};
        s.set_path(PathBuf::new());
    }

    #[test]
    #[should_panic]
    fn stub_set_as_any() {
        let s = Stub {};
        ResourceData::as_any(&s);
    }

    #[test]
    #[should_panic]
    fn stub_set_as_any_mut() {
        let mut s = Stub {};
        ResourceData::as_any_mut(&mut s);
        s.type_uuid();
    }

    #[test]
    fn stub_set_type_uuid() {
        let s = Stub {};
        assert_eq!(s.type_uuid(), Uuid::default());
    }
}
