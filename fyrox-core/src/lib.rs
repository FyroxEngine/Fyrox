//! Core data structures and algorithms used throughout Fyrox.
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
use std::ffi::OsString;
use std::{
    borrow::Borrow,
    hash::Hash,
    path::{Path, PathBuf},
};

pub mod color;
pub mod color_gradient;
pub mod curve;
pub mod io;
pub mod log;
pub mod math;
pub mod numeric_range;
pub mod octree;
pub mod pool;
pub mod profiler;
pub mod quadtree;
pub mod rectpack;
pub mod reflect;
pub mod sparse;
pub mod sstorage;
pub mod variable;
pub mod visitor;
pub mod watcher;

pub use futures;
pub use instant;

pub use notify;

#[cfg(target_arch = "wasm32")]
pub use js_sys;
use std::iter::FromIterator;
use uuid::Uuid;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures;
#[cfg(target_arch = "wasm32")]
pub use web_sys;

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
#[macro_export]
macro_rules! define_is_as {
    ($typ:tt : $kind:ident -> ref $result:path => fn $is:ident, fn $as_ref:ident, fn $as_mut:ident) => {
        /// Returns true if node is instance of given type.
        pub fn $is(&self) -> bool {
            matches!(self, $typ::$kind(_))
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

/// Appends specified extension to the path.
///
/// # Examples
///
/// ```rust
/// # use std::path::Path;
/// # use fyrox_core::append_extension;
/// let path = Path::new("foo.bar");
/// let new_path = append_extension(path, "baz");
/// assert_eq!(new_path, Path::new("foo.bar.baz"))
/// ```
#[must_use]
pub fn append_extension<P: AsRef<Path>, E: AsRef<str>>(
    path: P,
    additional_extension: E,
) -> PathBuf {
    let mut final_path = path.as_ref().to_path_buf();
    let new_extension = final_path
        .extension()
        .map(|e| {
            let mut ext = e.to_owned();
            ext.push(".");
            ext.push(additional_extension.as_ref());
            ext
        })
        .unwrap_or_else(|| OsString::from(additional_extension.as_ref()));
    final_path.set_extension(new_extension);
    final_path
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
        let mut region = visitor.enter_region(name)?;

        self.forward_map.visit("ForwardMap", &mut region)?;
        self.backward_map.visit("BackwardMap", &mut region)?;

        Ok(())
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

#[inline]
pub fn hash_combine(lhs: u64, rhs: u64) -> u64 {
    lhs ^ (rhs
        .wrapping_add(0x9e3779b9)
        .wrapping_add(lhs << 6)
        .wrapping_add(lhs >> 2))
}

/// Strip working directory from file name. The function may fail for one main reason -
/// input path is not valid, does not exist, or there is some other issues with it.
pub fn make_relative_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, std::io::Error> {
    match path
        .as_ref()
        .canonicalize()?
        .strip_prefix(std::env::current_dir()?.canonicalize()?)
    {
        Ok(relative_path) => Ok(replace_slashes(relative_path)),
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to strip prefix!",
        )),
    }
}

/// A trait for an entity that has unique type identifier.
pub trait TypeUuidProvider: Sized {
    /// Return type UUID.
    fn type_uuid() -> Uuid;
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use fxhash::FxHashMap;

    use crate::{
        append_extension, hash_combine, make_relative_path,
        visitor::{Visit, Visitor},
        BiDirHashMap,
    };

    #[test]
    fn test_append_extension() {
        let path = Path::new("foo.bar");
        let new_path = append_extension(path, "baz");
        assert_eq!(new_path, Path::new("foo.bar.baz"));
    }

    #[test]
    fn bi_dir_hash_map_insert() {
        let mut map = BiDirHashMap::<u32, u32>::default();

        assert!(map.forward_map.is_empty());
        assert!(map.backward_map.is_empty());

        let result = map.insert(1, 42);

        assert_eq!(result, None);
        assert_eq!(map.forward_map.get_key_value(&1), Some((&1, &42)));
        assert_eq!(map.backward_map.get_key_value(&42), Some((&42, &1)));
    }

    #[test]
    fn bi_dir_hash_map_remove_by_key() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        assert_eq!(map.forward_map.get_key_value(&1), Some((&1, &42)));
        assert_eq!(map.backward_map.get_key_value(&42), Some((&42, &1)));

        let result = map.remove_by_key(&42);
        assert_eq!(result, None);

        let result = map.remove_by_key(&1);
        assert_eq!(result, Some(42));
        assert!(map.forward_map.is_empty());
        assert!(map.backward_map.is_empty());
    }

    #[test]
    fn bi_dir_hash_map_remove_by_value() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        assert_eq!(map.forward_map.get_key_value(&1), Some((&1, &42)));
        assert_eq!(map.backward_map.get_key_value(&42), Some((&42, &1)));

        let result = map.remove_by_value(&1);
        assert_eq!(result, None);

        let result = map.remove_by_value(&42);
        assert_eq!(result, Some(1));
        assert!(map.forward_map.is_empty());
        assert!(map.backward_map.is_empty());
    }

    #[test]
    fn bi_dir_hash_map_contains_key() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        assert!(map.contains_key(&1));
        assert!(!map.contains_key(&42));
    }

    #[test]
    fn bi_dir_hash_map_contains_value() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        assert!(map.contains_value(&42));
        assert!(!map.contains_value(&1));
    }

    #[test]
    fn bi_dir_hash_map_value_of() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        assert_eq!(map.value_of(&1), Some(&42));
        assert_eq!(map.value_of(&42), None);
    }

    #[test]
    fn bi_dir_hash_map_key_of() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        assert_eq!(map.key_of(&1), None);
        assert_eq!(map.key_of(&42), Some(&1));
    }

    #[test]
    fn bi_dir_hash_map_getters() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        assert!(map.is_empty());

        map.insert(1, 42);
        assert_eq!(map.len(), 1);

        assert!(map.forward_map().eq(&map.forward_map));
        assert!(map.backward_map().eq(&map.backward_map));

        map.clear();
        assert!(map.is_empty());
    }

    #[test]
    fn bi_dir_hash_map_into_inner() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        map.insert(1, 42);

        let (f, b) = map.clone().into_inner();
        assert!(map.forward_map().eq(&f));
        assert!(map.backward_map().eq(&b));
    }

    #[test]
    fn from_fx_hash_map_for_bi_dir_hash_map() {
        let mut h = FxHashMap::default();
        h.insert(1, 42);

        let map = BiDirHashMap::from(h);
        assert_eq!(map.forward_map.get_key_value(&1), Some((&1, &42)));
        assert_eq!(map.backward_map.get_key_value(&42), Some((&42, &1)));
    }

    #[test]
    fn test_visit_for_bi_dir_hash_map() {
        let mut map = BiDirHashMap::<u32, u32>::default();
        let mut visitor = Visitor::default();

        assert!(map.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn from_iter_for_bi_dir_hash_map() {
        let map = BiDirHashMap::from_iter(vec![(1, 42)]);

        assert_eq!(map.forward_map.get_key_value(&1), Some((&1, &42)));
        assert_eq!(map.backward_map.get_key_value(&42), Some((&42, &1)));
    }

    #[test]
    fn test_hash_combine() {
        assert_eq!(hash_combine(1, 1), 0x9E3779FB);
    }

    #[test]
    fn test_make_relative_path() {
        assert!(make_relative_path(Path::new("foo.txt")).is_err());
        assert!(make_relative_path(Path::new("Cargo.toml")).is_ok());
    }
}
