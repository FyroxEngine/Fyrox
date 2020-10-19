#![warn(missing_docs)]

//! Resource module contains all structures and method to manage resources.

use crate::core::visitor::{Visit, VisitResult, Visitor};
use std::{
    fmt::Debug,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

pub mod fbx;
pub mod model;
pub mod texture;

/// Trait for texture data.
pub trait ResourceData: 'static + Default + Debug + Visit + Send + Sync {
    /// Returns path of resource data.
    fn path(&self) -> &Path;
}

/// Resource could be in three possible states:
/// 1. Pending - it is loading.
/// 2. LoadError - an error has occurred during the load.
/// 3. Ok - resource is fully loaded and ready to use.
///
/// Why it is so complex?
/// Short answer: asynchronous loading.
/// Long answer: when you loading a scene you expect it to be loaded as fast as
/// possible, use all available power of the CPU. To achieve that each resource
/// ideally should be loaded on separate core of the CPU, but since this is
/// asynchronous, we must have the ability to track the state of the resource.  
#[derive(Debug)]
pub enum ResourceState<T: ResourceData, E: 'static + Debug + Send> {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: PathBuf,
        /// List of wakers to wake future when resource is fully loaded.
        wakers: Vec<Waker>,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: PathBuf,
        /// An error. This wrapped in Option only to be Default_ed.        
        error: Option<E>,
    },
    /// Actual resource data when it is fully loaded.
    Ok(T),
}

impl<T: ResourceData, E: 'static + Debug + Send> Visit for ResourceState<T, E> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        // This branch may fail only on load (except some extreme conditions like out of memory).
        if id.visit("Id", visitor).is_ok() {
            if visitor.is_reading() {
                *self = Self::from_id(id)?;
            }
            match self {
                // Unreachable because resource must be .await_ed before serialization.
                Self::Pending { .. } => unreachable!(),
                // This may look strange if we attempting to save an invalid resource, but this may be
                // actually useful - a resource may become loadable at the deserialization.
                Self::LoadError { path, .. } => path.visit("Path", visitor)?,
                Self::Ok(details) => details.visit("Details", visitor)?,
            }

            visitor.leave_region()
        } else {
            visitor.leave_region()?;

            // Keep compatibility with old versions.
            let mut details = T::default();
            details.visit(name, visitor)?;

            *self = Self::Ok(details);
            Ok(())
        }
    }
}

/// See module docs.
#[derive(Debug)]
pub struct Resource<T: ResourceData, E: 'static + Debug + Send> {
    state: Option<Arc<Mutex<ResourceState<T, E>>>>,
}

impl<T: ResourceData, E: 'static + Debug + Send> Resource<T, E> {
    /// Creates new resource with a given state.
    pub fn new(state: ResourceState<T, E>) -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(state))),
        }
    }

    /// Converts self to internal value.
    pub fn into_inner(self) -> Arc<Mutex<ResourceState<T, E>>> {
        self.state.unwrap()
    }

    /// Locks internal mutex provides access to the state.
    pub fn state(&self) -> MutexGuard<'_, ResourceState<T, E>> {
        self.state.as_ref().unwrap().lock().unwrap()
    }

    /// Returns exact amount of users of the resource.
    pub fn use_count(&self) -> usize {
        Arc::strong_count(&self.state.as_ref().unwrap())
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    pub fn key(&self) -> usize {
        (&**self.state.as_ref().unwrap() as *const _) as usize
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> Default for Resource<T, E> {
    fn default() -> Self {
        Self { state: None }
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> Clone for Resource<T, E> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> From<Arc<Mutex<ResourceState<T, E>>>>
    for Resource<T, E>
{
    fn from(state: Arc<Mutex<ResourceState<T, E>>>) -> Self {
        Self { state: Some(state) }
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> Into<Arc<Mutex<ResourceState<T, E>>>>
    for Resource<T, E>
{
    fn into(self) -> Arc<Mutex<ResourceState<T, E>>> {
        self.state.unwrap()
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> Future for Resource<T, E> {
    type Output = Self;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.as_ref().state.clone();
        match *state.unwrap().lock().unwrap() {
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
            ResourceState::LoadError { .. } | ResourceState::Ok(_) => Poll::Ready(self.clone()),
        }
    }
}

impl<T, E> Visit for Resource<T, E>
where
    T: ResourceData,
    E: 'static + Debug + Send,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // This branch may fail only on load (except some extreme conditions like out of memory).
        if self.state.visit("State", visitor).is_err() {
            visitor.leave_region()?;

            // Keep compatibility with old versions.

            // Create default state here, since it is only for compatibility we don't care about
            // redundant memory allocations.
            let mut state = Arc::new(Mutex::new(ResourceState::Ok(Default::default())));

            state.visit(name, visitor)?;

            self.state = Some(state);

            Ok(())
        } else {
            visitor.leave_region()
        }
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> ResourceState<T, E> {
    fn id(&self) -> u32 {
        match self {
            Self::Pending { .. } => 0,
            Self::LoadError { .. } => 1,
            Self::Ok(_) => 2,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Pending {
                path: Default::default(),
                wakers: Default::default(),
            }),
            1 => Ok(Self::LoadError {
                path: Default::default(),
                error: None,
            }),
            2 => Ok(Self::Ok(Default::default())),
            _ => Err(format!("Invalid resource id {}", id)),
        }
    }

    /// Returns a path to the resource source.
    pub fn path(&self) -> &Path {
        match self {
            Self::Pending { path, .. } => path,
            Self::LoadError { path, .. } => path,
            Self::Ok(details) => details.path(),
        }
    }
}

impl<T: ResourceData, E: 'static + Debug + Send> Default for ResourceState<T, E> {
    fn default() -> Self {
        Self::Ok(Default::default())
    }
}
