//! Core data structures and algorithms used throughout rg3d.
//!
//! Some of them can be useful separately outside the engine.

#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::from_over_into)]

#[macro_use]
extern crate memoffset;
#[macro_use]
extern crate lazy_static;

pub use arrayvec;
pub use byteorder;
pub use nalgebra as algebra;
pub use num_traits;
pub use parking_lot;
pub use rand;
pub use uuid;

use crate::visitor::{Visit, VisitResult, Visitor};
use fxhash::FxHashMap;
use std::{
    borrow::Borrow,
    hash::Hash,
    path::{Path, PathBuf},
};

pub mod color;
pub mod color_gradient;
pub mod curve;
pub mod inspect;
pub mod io;
pub mod math;
pub mod numeric_range;
pub mod octree;
pub mod pool;
pub mod profiler;
pub mod quadtree;
pub mod rectpack;
pub mod sparse;
pub mod sstorage;
pub mod visitor;

pub use futures;
pub use instant;

#[cfg(target_arch = "wasm32")]
pub use js_sys;
use std::iter::FromIterator;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures;
#[cfg(target_arch = "wasm32")]
pub use web_sys;

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
#[macro_export]
macro_rules! define_is_as {
    ($typ:tt : $kind:ident -> ref $result:ty => fn $is:ident, fn $as_ref:ident, fn $as_mut:ident) => {
        /// Returns true if node is instance of given type.
        pub fn $is(&self) -> bool {
            match self {
                $typ::$kind(_) => true,
                _ => false,
            }
        }

        /// Tries to cast shared reference to a node to given type, panics if
        /// cast is not possible.
        pub fn $as_ref(&self) -> &$result {
            match self {
                $typ::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind)),
            }
        }

        /// Tries to cast mutable reference to a node to given type, panics if
        /// cast is not possible.
        pub fn $as_mut(&mut self) -> &mut $result {
            match self {
                $typ::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind)),
            }
        }
    };
}

/// Utility function that replaces back slashes \ to forward /
/// It replaces slashes only on windows!
pub fn replace_slashes<P: AsRef<Path>>(path: P) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if path.as_ref().is_absolute() {
            // Absolute Windows paths are incompatible with other operating systems so
            // don't bother here and return existing path as owned.
            path.as_ref().to_owned()
        } else {
            // Replace all \ to /. This is needed because on macos or linux \ is a valid symbol in
            // file name, and not separator (except linux which understand both variants).
            let mut os_str = std::ffi::OsString::new();
            let count = path.as_ref().components().count();
            for (i, component) in path.as_ref().components().enumerate() {
                os_str.push(component.as_os_str());
                if i != count - 1 {
                    os_str.push("/");
                }
            }
            PathBuf::from(os_str)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        path.as_ref().to_owned()
    }
}

#[derive(Clone, Debug)]
pub struct BiDirHashMap<K, V> {
    forward_map: FxHashMap<K, V>,
    backward_map: FxHashMap<V, K>,
}

impl<K: Hash + Eq + Clone, V: Hash + Eq + Clone> BiDirHashMap<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let existing = self.forward_map.insert(key.clone(), value.clone());
        self.backward_map.insert(value, key);
        existing
    }

    pub fn remove_by_key(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.forward_map.remove(key) {
            self.backward_map.remove(&value);
            Some(value)
        } else {
            None
        }
    }

    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.forward_map.contains_key(key)
    }

    pub fn remove_by_value(&mut self, value: &V) -> Option<K> {
        if let Some(key) = self.backward_map.remove(value) {
            self.forward_map.remove(&key);
            Some(key)
        } else {
            None
        }
    }

    pub fn contains_value<Q: ?Sized>(&self, value: &Q) -> bool
    where
        V: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.backward_map.contains_key(value)
    }

    pub fn value_of(&self, node: &K) -> Option<&V> {
        self.forward_map.get(node)
    }

    pub fn key_of(&self, value: &V) -> Option<&K> {
        self.backward_map.get(value)
    }

    pub fn len(&self) -> usize {
        self.forward_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.forward_map.is_empty()
    }

    pub fn clear(&mut self) {
        self.forward_map.clear();
        self.backward_map.clear();
    }

    pub fn forward_map(&self) -> &FxHashMap<K, V> {
        &self.forward_map
    }

    pub fn backward_map(&self) -> &FxHashMap<V, K> {
        &self.backward_map
    }

    pub fn into_inner(self) -> (FxHashMap<K, V>, FxHashMap<V, K>) {
        (self.forward_map, self.backward_map)
    }
}

impl<K, V> Default for BiDirHashMap<K, V> {
    fn default() -> Self {
        Self {
            forward_map: Default::default(),
            backward_map: Default::default(),
        }
    }
}

impl<K: Hash + Eq + Clone, V: Hash + Eq + Clone> From<FxHashMap<K, V>> for BiDirHashMap<K, V> {
    fn from(forward_map: FxHashMap<K, V>) -> Self {
        let mut backward_map = FxHashMap::default();
        for (k, v) in forward_map.iter() {
            backward_map.insert(v.clone(), k.clone());
        }
        Self {
            forward_map,
            backward_map,
        }
    }
}

impl<K, V> Visit for BiDirHashMap<K, V>
where
    K: Hash + Eq + Clone + Default + Visit,
    V: Hash + Eq + Clone + Default + Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.forward_map.visit("ForwardMap", visitor)?;
        self.backward_map.visit("BackwardMap", visitor)?;

        visitor.leave_region()
    }
}

impl<K, V> FromIterator<(K, V)> for BiDirHashMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Hash + Eq + Clone,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut hm = Self::default();
        for (k, v) in iter {
            hm.forward_map.insert(k.clone(), v.clone());
            hm.backward_map.insert(v, k);
        }
        hm
    }
}

pub trait VecExtensions<T> {
    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` such that `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the
    /// original order, and preserves the order of the retained elements.
    ///
    /// # Notes
    ///
    /// This method is the copy of `retain` method of Vec, but with ability to
    /// modify each element.
    fn retain_mut_ext<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool;
}

impl<T> VecExtensions<T> for Vec<T> {
    fn retain_mut_ext<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut **self;

            for i in 0..len {
                if !f(&mut v[i]) {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }
        if del > 0 {
            self.truncate(len - del);
        }
    }
}

#[inline]
pub fn hash_combine(lhs: u64, rhs: u64) -> u64 {
    lhs ^ (rhs
        .wrapping_add(0x9e3779b9)
        .wrapping_add(lhs << 6)
        .wrapping_add(lhs >> 2))
}
