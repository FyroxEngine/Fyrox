use crate::{
    parking_lot::Mutex,
    visitor::{Visit, VisitResult, Visitor},
};
use std::fmt::{Display, Formatter};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct ImmutableString(Arc<String>);

impl Display for ImmutableString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

impl Visit for ImmutableString {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Serialize/deserialize as ordinary string.
        self.0.visit(name, visitor)?;

        // Deduplicate on deserialization.
        if visitor.is_reading() {
            *self = SSTORAGE.lock().insert(self.0.as_ref());
        }

        Ok(())
    }
}

impl Default for ImmutableString {
    fn default() -> Self {
        Self::new("")
    }
}

impl ImmutableString {
    #[inline]
    pub fn new<S: AsRef<str>>(string: S) -> ImmutableString {
        SSTORAGE.lock().insert(string)
    }

    #[inline]
    pub fn id(&self) -> u64 {
        &*self.0 as *const _ as u64
    }

    #[inline]
    pub fn to_mutable(&self) -> String {
        (*self.0).clone()
    }
}

impl Deref for ImmutableString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
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

#[derive(Default)]
pub struct ImmutableStringStorage {
    vec: HashMap<u64, Arc<String>>,
}

impl ImmutableStringStorage {
    #[inline]
    fn insert<S: AsRef<str>>(&mut self, string: S) -> ImmutableString {
        let mut hasher = DefaultHasher::new();
        string.as_ref().hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(existing) = self.vec.get(&hash) {
            ImmutableString(existing.clone())
        } else {
            let immutable = Arc::new(string.as_ref().to_owned());
            self.vec.insert(hash, immutable.clone());
            ImmutableString(immutable)
        }
    }
}

impl ImmutableStringStorage {
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
    use crate::sstorage::{ImmutableString, ImmutableStringStorage};

    #[test]
    fn test_immutable_string_uniqueness() {
        let a = ImmutableString::new("Foobar");
        let b = ImmutableString::new("Foobar");

        assert_eq!(ImmutableStringStorage::entry_count(), 1);
        assert_eq!(a.id(), b.id())
    }
}
