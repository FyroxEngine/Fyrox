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
pub use sstorage::ImmutableString;
pub use uuid;

use crate::visitor::{Visit, VisitResult, Visitor};
use fxhash::FxHashMap;
use std::ffi::OsString;
use std::hash::Hasher;
use std::{
    borrow::Borrow,
    cmp,
    hash::Hash,
    path::{Path, PathBuf},
};

use bytemuck::Pod;
pub mod color;
pub mod color_gradient;
pub mod io;
pub mod log;
pub mod math;
pub mod net;
pub mod numeric_range;
pub mod pool;
pub mod profiler;
pub mod quadtree;
pub mod rectpack;
pub mod reflect;
pub mod sparse;
pub mod sstorage;
pub mod task;
pub mod type_traits;
pub mod variable;
pub mod visitor;
pub mod watcher;

pub use futures;
pub use instant;

pub use notify;

#[cfg(target_arch = "wasm32")]
pub use js_sys;
use std::marker::PhantomData;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures;
#[cfg(target_arch = "wasm32")]
pub use web_sys;

pub use type_traits::prelude::*;
/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
#[macro_export]
macro_rules! define_is_as {
    ($typ:tt : $kind:ident -> ref $result:path => fn $is:ident, fn $as_ref:ident, fn $as_mut:ident) => {
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

/// Utility function that replaces back slashes \ to forward /. Internally, it converts the input
/// path to string (lossy - see [`Path::to_string_lossy`]) and replaces the slashes in the string.
/// Finally, it converts the string to the PathBuf and returns it. This method is intended to be
/// used only for paths, that does not contain non-unicode characters.
pub fn replace_slashes<P: AsRef<Path>>(path: P) -> PathBuf {
    PathBuf::from(
        path.as_ref()
            .to_string_lossy()
            .to_string()
            .replace('\\', "/"),
    )
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

    pub fn contains_key<Q: ?Sized + Hash + Eq>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
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

    pub fn contains_value<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> bool
    where
        V: Borrow<Q>,
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

/// "Transmutes" array of any sized type to a slice of bytes.
pub fn array_as_u8_slice<T: Sized + Pod>(v: &[T]) -> &'_ [u8] {
    // SAFETY: It is safe to reinterpret data to read it.
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// "Transmutes" array of any sized type to a slice of some other type.
pub fn transmute_slice<T: Sized, U: Sized>(v: &[T]) -> &'_ [U] {
    // SAFETY: It is safe to reinterpret data to read it.
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const U,
            std::mem::size_of_val(v) / std::mem::size_of::<U>(),
        )
    }
}

/// "Transmutes" value of any sized type to a slice of bytes.
pub fn value_as_u8_slice<T: Sized + Pod>(v: &T) -> &'_ [u8] {
    // SAFETY: It is safe to reinterpret data to read it.
    unsafe { std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>()) }
}

/// Takes a vector of trivially-copyable values and turns it into a vector of bytes.
pub fn transmute_vec_as_bytes<T: Pod>(vec: Vec<T>) -> Vec<u8> {
    unsafe {
        let mut vec = std::mem::ManuallyDrop::new(vec);
        Vec::from_raw_parts(
            vec.as_mut_ptr() as *mut u8,
            vec.len() * std::mem::size_of::<T>(),
            vec.capacity() * std::mem::size_of::<T>(),
        )
    }
}

/// Performs hashing of a sized value by interpreting it as raw memory.
pub fn hash_as_bytes<T: Sized + Pod, H: Hasher>(value: &T, hasher: &mut H) {
    hasher.write(value_as_u8_slice(value))
}

/// Compares two strings using case-insensitive comparison. This function does not allocate any
/// any memory and significantly faster than `a.to_lowercase() == b.to_lowercase()`.
pub fn cmp_strings_case_insensitive(a: impl AsRef<str>, b: impl AsRef<str>) -> bool {
    let a_ref = a.as_ref();
    let b_ref = b.as_ref();

    if a_ref.len() != b_ref.len() {
        return false;
    }

    a_ref
        .chars()
        .zip(b_ref.chars())
        .all(|(ca, cb)| ca.to_lowercase().eq(cb.to_lowercase()))
}

pub fn make_pretty_type_name(type_name: &str) -> &str {
    let mut colon_position = None;
    let mut byte_pos = 0;
    for c in type_name.chars() {
        byte_pos += c.len_utf8();
        if c == ':' {
            colon_position = Some(byte_pos);
        } else if c == '<' {
            break;
        }
    }
    if let Some(colon_position) = colon_position {
        type_name.split_at(colon_position).1
    } else {
        type_name
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct PhantomDataSendSync<T: ?Sized>(PhantomData<T>);

// SAFETY: PhantomDataSendSync does not hold any data.
unsafe impl<T: ?Sized> Send for PhantomDataSendSync<T> {}
// SAFETY: PhantomDataSendSync does not hold any data.
unsafe impl<T: ?Sized> Sync for PhantomDataSendSync<T> {}

impl<T: ?Sized> Hash for PhantomDataSendSync<T> {
    #[inline]
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl<T: ?Sized> PartialEq for PhantomDataSendSync<T> {
    fn eq(&self, _other: &PhantomDataSendSync<T>) -> bool {
        true
    }
}

impl<T: ?Sized> Eq for PhantomDataSendSync<T> {}

impl<T: ?Sized> PartialOrd for PhantomDataSendSync<T> {
    fn partial_cmp(&self, _other: &PhantomDataSendSync<T>) -> Option<cmp::Ordering> {
        Some(self.cmp(_other))
    }
}

impl<T: ?Sized> Ord for PhantomDataSendSync<T> {
    fn cmp(&self, _other: &PhantomDataSendSync<T>) -> cmp::Ordering {
        cmp::Ordering::Equal
    }
}

impl<T: ?Sized> Copy for PhantomDataSendSync<T> {}

impl<T: ?Sized> Clone for PhantomDataSendSync<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Default for PhantomDataSendSync<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// A trait for entities that have name.
pub trait NameProvider {
    /// Returns a reference to the name of the entity.
    fn name(&self) -> &str;
}

/// Tries to find an entity by its name in a series of entities produced by an iterator.
pub fn find_by_name_ref<'a, T, I, S, K>(mut iter: I, name: S) -> Option<(K, &'a T)>
where
    T: NameProvider,
    I: Iterator<Item = (K, &'a T)>,
    S: AsRef<str>,
{
    iter.find(|(_, value)| value.name() == name.as_ref())
}

/// Tries to find an entity by its name in a series of entities produced by an iterator.
pub fn find_by_name_mut<'a, T, I, S, K>(mut iter: I, name: S) -> Option<(K, &'a mut T)>
where
    T: NameProvider,
    I: Iterator<Item = (K, &'a mut T)>,
    S: AsRef<str>,
{
    iter.find(|(_, value)| value.name() == name.as_ref())
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{
        append_extension, cmp_strings_case_insensitive, combine_uuids, hash_combine,
        make_relative_path, transmute_vec_as_bytes,
        visitor::{Visit, Visitor},
        BiDirHashMap,
    };
    use fxhash::FxHashMap;
    use std::mem::size_of;
    use uuid::uuid;

    #[test]
    fn test_combine_uuids() {
        let a = uuid!("d1a45bd5-5066-4b28-b103-95c59c230e77");
        let b = uuid!("0a06591a-1c66-4299-ba6f-2b205b795575");

        assert_ne!(combine_uuids(a, b), a);
        assert_ne!(combine_uuids(a, b), b);
    }

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

    #[test]
    fn tests_case_insensitive_str_comparison() {
        assert!(cmp_strings_case_insensitive("FooBar", "FOOBaR"));
        assert!(!cmp_strings_case_insensitive("FooBaz", "FOOBaR"));
        assert!(cmp_strings_case_insensitive("foobar", "foobar"));
    }

    #[test]
    fn test_transmute_vec_as_bytes_length_new_f32() {
        let vec = vec![1.0f32, 2.0, 3.0];
        let byte_vec = transmute_vec_as_bytes(vec.clone());
        let expected_length = vec.len() * size_of::<f32>();
        assert_eq!(byte_vec.len(), expected_length);
    }

    #[test]
    fn test_transmute_vec_as_bytes_length_new_usize() {
        let vec = vec![1usize, 2, 3];
        let byte_vec = transmute_vec_as_bytes(vec.clone());
        let expected_length = vec.len() * size_of::<usize>();
        assert_eq!(byte_vec.len(), expected_length);
    }
}
