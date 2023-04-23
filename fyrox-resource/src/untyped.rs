use crate::{state::ResourceState, ResourceData, ResourceLoadError};
use fyrox_core::{parking_lot::Mutex, visitor::prelude::*};
use std::{
    fmt::{Debug, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone)]
pub struct UntypedResource(pub Arc<Mutex<ResourceState>>);

impl Visit for UntypedResource {
    // Delegating implementation.
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Default for UntypedResource {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_pending(
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
    pub fn new_pending(path: PathBuf) -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_pending(path))))
    }

    pub fn new_ok<T: ResourceData>(data: T) -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_ok(data))))
    }

    pub fn new_load_error(path: PathBuf, error: Option<Arc<dyn ResourceLoadError>>) -> Self {
        Self(Arc::new(Mutex::new(ResourceState::new_load_error(
            path, error,
        ))))
    }

    /// Returns true if the resource is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(*self.0.lock(), ResourceState::Pending { .. })
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
}

impl Future for UntypedResource {
    type Output = Result<Self, Option<Arc<dyn ResourceLoadError>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.0.clone();
        match *state.lock() {
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
