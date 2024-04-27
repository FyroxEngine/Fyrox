//! Immutable string + immutable string storage. See docs of [`ImmutableString`] and
//! [`ImmutableStringStorage`] for more info.

#![warn(missing_docs)]

use crate::{
    parking_lot::Mutex,
    uuid_provider,
    visitor::{Visit, VisitResult, Visitor},
};
use fxhash::{FxHashMap, FxHasher};
pub use fyrox_core_derive::TypeUuidProvider;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, Debug)]
struct State {
    string: String,
    hash: u64,
}

/// Immutable string is a string with constant content. Immutability gives some nice properties:
///
/// - Address of the string could be used as a hash, which improves hashing performance dramatically
/// and basically making it constant in terms of complexity (O(1))
/// - Equality comparison becomes constant in terms of complexity.
/// - Uniqueness guarantees - means that calling multiple times will allocate memory only once
/// `ImmutableString::new("foo")` and in consecutive calls existing string will be used.
///
/// # Use cases
///
/// Most common use case for immutable strings is hash map keys in performance-critical places.
#[derive(Clone)]
pub struct ImmutableString(Arc<State>);

uuid_provider!(ImmutableString = "452caac1-19f7-43d6-9e33-92c2c9163332");

impl Display for ImmutableString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.string.as_ref())
    }
}

impl Debug for ImmutableString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0.string, f)
    }
}

impl Visit for ImmutableString {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Serialize/deserialize as ordinary string.
        let mut string = self.0.string.clone();
        string.visit(name, visitor)?;

        // Deduplicate on deserialization.
        if visitor.is_reading() {
            *self = SSTORAGE.lock().insert(string);
        }

        Ok(())
    }
}

impl Serialize for ImmutableString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ImmutableString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(ImmutableString::new(
            deserializer.deserialize_string(ImmutableStringVisitor {})?,
        ))
    }
}

struct ImmutableStringVisitor {}

impl serde::de::Visitor<'_> for ImmutableStringVisitor {
    type Value = ImmutableString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ImmutableString::new(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
    }
}

impl Default for ImmutableString {
    fn default() -> Self {
        Self::new("")
    }
}

impl AsRef<str> for ImmutableString {
    fn as_ref(&self) -> &str {
        self.deref()
    }
}

impl ImmutableString {
    /// Creates new immutable string from given string slice.
    ///
    /// # Performance
    ///
    /// This method has amortized O(1) complexity, in worst case (when there is no such string
    /// in backing storage) it allocates memory which could lead to complexity defined by current
    /// memory allocator.
    #[inline]
    pub fn new<S: AsRef<str>>(string: S) -> ImmutableString {
        SSTORAGE.lock().insert(string)
    }

    /// Returns unique identifier of the string. Keep in mind that uniqueness is guaranteed only
    /// for a single session, uniqueness is not preserved between application runs.
    #[inline]
    pub fn id(&self) -> u64 {
        self.0.hash
    }

    /// Clones content of inner immutable string to a mutable string.
    #[inline]
    pub fn to_mutable(&self) -> String {
        self.0.string.clone()
    }

    /// Get a reference to the inner str.
    pub fn as_str(&self) -> &str {
        self.deref()
    }
}

impl From<&str> for ImmutableString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ImmutableString {
    fn from(value: String) -> Self {
        SSTORAGE.lock().insert_owned(value)
    }
}

impl Deref for ImmutableString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.string.as_ref()
    }
}

impl Hash for ImmutableString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.id())
    }
}

impl PartialEq for ImmutableString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for ImmutableString {}

/// Immutable string storage is a backing storage for every immutable string in the application,
/// storage is a singleton. In normal circumstances you should never use it directly.
#[derive(Default)]
pub struct ImmutableStringStorage {
    vec: FxHashMap<u64, Arc<State>>,
}

impl ImmutableStringStorage {
    #[inline]
    fn insert<S: AsRef<str>>(&mut self, string: S) -> ImmutableString {
        let mut hasher = FxHasher::default();
        string.as_ref().hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(existing) = self.vec.get(&hash) {
            ImmutableString(existing.clone())
        } else {
            let immutable = Arc::new(State {
                string: string.as_ref().to_owned(),
                hash,
            });
            self.vec.insert(hash, immutable.clone());
            ImmutableString(immutable)
        }
    }
    /// Insert without copying the given String.
    #[inline]
    fn insert_owned(&mut self, string: String) -> ImmutableString {
        let mut hasher = FxHasher::default();
        string.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(existing) = self.vec.get(&hash) {
            ImmutableString(existing.clone())
        } else {
            let immutable = Arc::new(State { string, hash });
            self.vec.insert(hash, immutable.clone());
            ImmutableString(immutable)
        }
    }
}

impl ImmutableStringStorage {
    /// Returns total amount of immutable strings in the storage.
    pub fn entry_count() -> usize {
        SSTORAGE.lock().vec.len()
    }
}

lazy_static! {
    static ref SSTORAGE: Arc<Mutex<ImmutableStringStorage>> =
        Arc::new(Mutex::new(ImmutableStringStorage::default()));
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_immutable_string_distinctness() {
        let a = ImmutableString::new("Foobar");
        let b = ImmutableString::new("rabooF");

        assert_ne!(a.id(), b.id())
    }

    #[test]
    fn test_immutable_string_uniqueness() {
        let a = ImmutableString::new("Foobar");
        let b = ImmutableString::new("Foobar");

        // All tests share the same ImmutableStringStorage, so there is no way
        // to know what this value should be. It depends on the order the test
        // are run.
        // assert_eq!(ImmutableStringStorage::entry_count(), 2);
        assert_eq!(a.id(), b.id())
    }

    #[test]
    fn test_immutable_string_uniqueness_from_owned() {
        let a = ImmutableString::new("Foobar");
        let b = ImmutableString::from("Foobar".to_owned());

        assert_eq!(a.id(), b.id())
    }

    #[test]
    fn visit_for_immutable_string() {
        let mut a = ImmutableString::new("Foobar");
        let mut visitor = Visitor::default();

        assert!(a.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn debug_for_immutable_string() {
        let a = ImmutableString::new("Foobar");

        assert_eq!(format!("{a:?}"), "\"Foobar\"");
    }

    #[test]
    fn debug_for_immutable_string_from_owned() {
        let a = ImmutableString::from("Foobar".to_owned());

        assert_eq!(format!("{a:?}"), "\"Foobar\"");
    }

    #[test]
    fn default_for_immutable_string() {
        let a = ImmutableString::default();

        assert_eq!(a.0.string, "");
    }

    #[test]
    fn immutable_string_to_mutable() {
        let a = ImmutableString::new("Foobar");

        assert_eq!(a.to_mutable(), String::from("Foobar"));
    }

    #[test]
    fn deref_for_immutable_string() {
        let s = "Foobar";
        let a = ImmutableString::new(s);

        assert_eq!(a.deref(), s);
    }

    #[test]
    fn eq_for_immutable_string() {
        let a = ImmutableString::new("Foobar");
        let b = ImmutableString::new("Foobar");

        assert!(a == b);
    }
}
