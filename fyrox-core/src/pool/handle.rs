use crate::{
    combine_uuids, pool::INVALID_GENERATION, reflect::prelude::*, uuid_provider,
    visitor::prelude::*, TypeUuidProvider,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::atomic::{self, AtomicUsize},
};
use uuid::Uuid;

/// Handle is some sort of non-owning reference to content in a pool. It stores
/// index of object and additional information that allows to ensure that handle
/// is still valid (points to the same object as when handle was created).
#[derive(Reflect, Serialize, Deserialize)]
pub struct Handle<T> {
    /// Index of object in pool.
    #[reflect(read_only, description = "Index of an object in a pool.")]
    pub(super) index: u32,
    /// Generation number, if it is same as generation of pool record at
    /// index of handle then this is valid handle.
    #[reflect(read_only, description = "Generation of an object in a pool.")]
    pub(super) generation: u32,
    /// Type holder.
    #[reflect(hidden)]
    #[serde(skip)]
    pub(super) type_marker: PhantomData<T>,
}

impl<T> Copy for Handle<T> {}

impl<T> Eq for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    #[inline]
    fn eq(&self, other: &Handle<T>) -> bool {
        self.generation == other.generation && self.index == other.index
    }
}

impl<T> Hash for Handle<T> {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> Handle<T> {
    pub const NONE: Handle<T> = Handle {
        index: 0,
        generation: INVALID_GENERATION,
        type_marker: PhantomData,
    };

    #[inline(always)]
    pub fn is_none(self) -> bool {
        self.index == 0 && self.generation == INVALID_GENERATION
    }

    #[inline(always)]
    pub fn is_some(self) -> bool {
        !self.is_none()
    }

    #[inline(always)]
    pub fn index(self) -> u32 {
        self.index
    }

    #[inline(always)]
    pub fn generation(self) -> u32 {
        self.generation
    }

    #[inline(always)]
    pub fn new(index: u32, generation: u32) -> Self {
        Handle {
            index,
            generation,
            type_marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn transmute<U>(&self) -> Handle<U> {
        Handle {
            index: self.index,
            generation: self.generation,
            type_marker: Default::default(),
        }
    }

    #[inline(always)]
    pub fn decode_from_u128(num: u128) -> Self {
        Self {
            index: num as u32,
            generation: (num >> 32) as u32,
            type_marker: Default::default(),
        }
    }

    #[inline(always)]
    pub fn encode_to_u128(&self) -> u128 {
        (self.index as u128) | ((self.generation as u128) << 32)
    }
}

impl<T> TypeUuidProvider for Handle<T>
where
    T: TypeUuidProvider,
{
    #[inline]
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid::uuid!("30c0668d-7a2c-47e6-8c7b-208fdcc905a1"),
            T::type_uuid(),
        )
    }
}

impl<T> PartialOrd for Handle<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Handle<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.index.cmp(&other.index)
    }
}

unsafe impl<T> Send for Handle<T> {}
unsafe impl<T> Sync for Handle<T> {}

impl<T> Display for Handle<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.index, self.generation)
    }
}

/// Atomic handle.
pub struct AtomicHandle(AtomicUsize);

impl Clone for AtomicHandle {
    #[inline]
    fn clone(&self) -> Self {
        Self(AtomicUsize::new(self.0.load(atomic::Ordering::Relaxed)))
    }
}

impl Default for AtomicHandle {
    #[inline]
    fn default() -> Self {
        Self::none()
    }
}

impl AtomicHandle {
    #[inline]
    pub fn none() -> Self {
        Self(AtomicUsize::new(0))
    }

    #[inline]
    pub fn new(index: u32, generation: u32) -> Self {
        let handle = Self(AtomicUsize::new(0));
        handle.set(index, generation);
        handle
    }

    #[inline]
    pub fn set(&self, index: u32, generation: u32) {
        let index = (index as usize) << (usize::BITS / 2) >> (usize::BITS / 2);
        let generation = (generation as usize) << (usize::BITS / 2);
        self.0.store(index | generation, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn set_from_handle<T>(&self, handle: Handle<T>) {
        self.set(handle.index, handle.generation)
    }

    #[inline(always)]
    pub fn is_some(&self) -> bool {
        self.generation() != INVALID_GENERATION
    }

    #[inline(always)]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    #[inline]
    pub fn index(&self) -> u32 {
        let bytes = self.0.load(atomic::Ordering::Relaxed);
        ((bytes << (usize::BITS / 2)) >> (usize::BITS / 2)) as u32
    }

    #[inline]
    pub fn generation(&self) -> u32 {
        let bytes = self.0.load(atomic::Ordering::Relaxed);
        (bytes >> (usize::BITS / 2)) as u32
    }
}

impl Display for AtomicHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.index(), self.generation())
    }
}

impl Debug for AtomicHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Idx: {}; Gen: {}]", self.index(), self.generation())
    }
}

/// Type-erased handle.
#[derive(
    Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq, Hash, Reflect, Visit, Serialize, Deserialize,
)]
pub struct ErasedHandle {
    /// Index of object in pool.
    #[reflect(read_only)]
    index: u32,
    /// Generation number, if it is same as generation of pool record at
    /// index of handle then this is valid handle.
    #[reflect(read_only)]
    generation: u32,
}

uuid_provider!(ErasedHandle = "50131acc-8b3b-40b5-b495-e2c552c94db3");

impl Display for ErasedHandle {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.index, self.generation)
    }
}

impl Default for ErasedHandle {
    #[inline]
    fn default() -> Self {
        Self::none()
    }
}

impl<T> From<ErasedHandle> for Handle<T> {
    #[inline]
    fn from(erased_handle: ErasedHandle) -> Self {
        Handle {
            index: erased_handle.index,
            generation: erased_handle.generation,
            type_marker: PhantomData,
        }
    }
}

impl<T> From<AtomicHandle> for Handle<T> {
    #[inline]
    fn from(atomic_handle: AtomicHandle) -> Self {
        Handle {
            index: atomic_handle.index(),
            generation: atomic_handle.generation(),
            type_marker: PhantomData,
        }
    }
}

impl<T> From<Handle<T>> for ErasedHandle {
    #[inline]
    fn from(h: Handle<T>) -> Self {
        Self {
            index: h.index,
            generation: h.generation,
        }
    }
}

impl ErasedHandle {
    #[inline]
    pub fn none() -> Self {
        Self {
            index: 0,
            generation: INVALID_GENERATION,
        }
    }

    #[inline]
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    #[inline(always)]
    pub fn is_some(&self) -> bool {
        self.generation != INVALID_GENERATION
    }

    #[inline(always)]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    #[inline(always)]
    pub fn index(self) -> u32 {
        self.index
    }

    #[inline(always)]
    pub fn generation(self) -> u32 {
        self.generation
    }
}

impl<T> Visit for Handle<T> {
    #[inline]
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.index.visit("Index", &mut region)?;
        self.generation.visit("Generation", &mut region)?;

        Ok(())
    }
}

impl<T> Default for Handle<T> {
    #[inline]
    fn default() -> Self {
        Self::NONE
    }
}

impl<T> Debug for Handle<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Idx: {}; Gen: {}]", self.index, self.generation)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        pool::{AtomicHandle, ErasedHandle, Handle, INVALID_GENERATION},
        visitor::{Visit, Visitor},
    };

    #[test]
    fn test_handle_u128_encode_decode() {
        let a = Handle::<()>::new(123, 321);
        let encoded = a.encode_to_u128();
        let decoded = Handle::<()>::decode_from_u128(encoded);
        assert_eq!(decoded, a);
    }

    #[test]
    fn erased_handle_none() {
        assert_eq!(
            ErasedHandle::none(),
            ErasedHandle {
                index: 0,
                generation: INVALID_GENERATION,
            }
        );
    }

    #[test]
    fn erased_handle_new() {
        assert_eq!(
            ErasedHandle::new(0, 1),
            ErasedHandle {
                index: 0,
                generation: 1,
            }
        );
    }

    #[test]
    fn erased_handle_is_some() {
        assert!(ErasedHandle::new(0, 1).is_some());
        assert!(!ErasedHandle::none().is_some());
    }

    #[test]
    fn erased_handle_is_none() {
        assert!(!ErasedHandle::new(0, 1).is_none());
        assert!(ErasedHandle::none().is_none());
    }

    #[test]
    fn erased_handle_index() {
        assert_eq!(
            ErasedHandle {
                index: 42,
                generation: 15
            }
            .index(),
            42
        );
    }

    #[test]
    fn erased_handle_generation() {
        assert_eq!(
            ErasedHandle {
                index: 42,
                generation: 15
            }
            .generation(),
            15
        );
    }

    #[test]
    fn default_for_erased_handle() {
        assert_eq!(ErasedHandle::default(), ErasedHandle::none());
    }

    #[test]
    fn erased_handle_from_handle() {
        let handle = Handle::<u32> {
            index: 0,
            generation: 1,
            type_marker: std::marker::PhantomData,
        };

        assert_eq!(
            ErasedHandle::from(handle),
            ErasedHandle {
                index: 0,
                generation: 1
            }
        );
    }

    #[test]
    fn handle_from_erased_handle() {
        let er = ErasedHandle {
            index: 0,
            generation: 1,
        };

        assert_eq!(
            Handle::from(er),
            Handle::<u32> {
                index: 0,
                generation: 1,
                type_marker: std::marker::PhantomData,
            }
        );
    }

    #[test]
    fn default_for_handle() {
        assert_eq!(Handle::default(), Handle::<u32>::NONE);
    }

    #[test]
    fn visit_for_handle() {
        let mut h = Handle::<u32>::default();
        let mut visitor = Visitor::default();

        assert!(h.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn test_debug_for_handle() {
        let h = Handle::<u32> {
            index: 0,
            generation: 1,
            type_marker: std::marker::PhantomData,
        };

        assert_eq!(format!("{h:?}"), "[Idx: 0; Gen: 1]");
    }

    #[test]
    fn handle_getters() {
        let h = Handle::<u32> {
            index: 0,
            generation: 1,
            type_marker: std::marker::PhantomData,
        };

        assert_eq!(h.index(), 0);
        assert_eq!(h.generation(), 1);
    }

    #[test]
    fn handle_transmute() {
        assert_eq!(
            Handle::<u32>::default().transmute::<f32>(),
            Handle::<f32>::default()
        );
    }

    #[test]
    fn test_atomic_handle() {
        let handle = AtomicHandle::new(123, 321);
        assert!(handle.is_some());
        assert_eq!(handle.index(), 123);
        assert_eq!(handle.generation(), 321);

        let handle = AtomicHandle::default();
        assert!(handle.is_none());
    }
}
