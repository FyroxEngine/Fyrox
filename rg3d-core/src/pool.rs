//! Pool is contiguous block of memory with fixed-size entries, each entry can be
//! either vacant or occupied. When you put an object into pool you get handle to
//! that object. You can use that handle later on to borrow a reference to an object.
//! Handle can point to some object or be invalid, this may look similar to raw
//! pointers, but there is two major differences:
//!
//! 1) We can check if handle is valid before accessing object it might point to.
//! 2) We can ensure that handle we using still valid for an object it points to.
//! Each handle store special field that is shared across entry and handle, so
//! handle is valid if these field in both handle and entry are same. This protects
//! from situations where you have a handle that has valid index of a record, but
//! payload in this record was replaced.
//!
//! Contiguous memory block increases efficiency of memory operations - CPU will
//! load portions of data into its cache piece by piece, it will be free from any
//! indirections that might cause cache invalidation. This is so called cache
//! friendliness.

#![allow(clippy::unneeded_field_pattern)]

use crate::visitor::{Visit, VisitResult, Visitor};
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};
use std::{
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

const INVALID_GENERATION: u32 = 0;

/// Pool allows to create as many objects as you want in contiguous memory
/// block. It allows to create and delete objects much faster than if they'll
/// be allocated on heap. Also since objects stored in contiguous memory block
/// they can be effectively accessed because such memory layout is cache-friendly.
#[derive(Debug)]
pub struct Pool<T: Sized> {
    records: Vec<PoolRecord<T>>,
    free_stack: Vec<u32>,
}

/// Handle is some sort of non-owning reference to content in a pool. It stores
/// index of object and additional information that allows to ensure that handle
/// is still valid (points to the same object as when handle was created).
pub struct Handle<T> {
    /// Index of object in pool.
    index: u32,
    /// Generation number, if it is same as generation of pool record at
    /// index of handle then this is valid handle.
    generation: u32,
    /// Type holder.
    type_marker: PhantomData<T>,
}

unsafe impl<T> Send for Handle<T> {}
unsafe impl<T> Sync for Handle<T> {}

/// Type-erased handle.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, PartialEq, Eq)]
pub struct ErasedHandle {
    /// Index of object in pool.
    index: u32,
    /// Generation number, if it is same as generation of pool record at
    /// index of handle then this is valid handle.
    generation: u32,
}

impl Default for ErasedHandle {
    fn default() -> Self {
        Self::none()
    }
}

impl<T> From<ErasedHandle> for Handle<T> {
    fn from(erased_handle: ErasedHandle) -> Self {
        Handle {
            index: erased_handle.index,
            generation: erased_handle.generation,
            type_marker: PhantomData,
        }
    }
}

impl<T> From<Handle<T>> for ErasedHandle {
    fn from(h: Handle<T>) -> Self {
        Self {
            index: h.index,
            generation: h.generation,
        }
    }
}

impl ErasedHandle {
    pub fn none() -> Self {
        ErasedHandle {
            index: 0,
            generation: INVALID_GENERATION,
        }
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
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.index.visit("Index", visitor)?;
        self.generation.visit("Generation", visitor)?;

        visitor.leave_region()
    }
}

impl<T> Default for Handle<T> {
    fn default() -> Self {
        Self::NONE
    }
}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Idx: {}; Gen: {}]", self.index, self.generation)
    }
}

#[derive(Debug)]
struct PoolRecord<T: Sized> {
    /// Generation number, used to keep info about lifetime. The handle is valid
    /// only if record it points to is of the same generation as the pool record.
    /// Notes: Zero is unknown generation used for None handles.
    generation: u32,
    /// Actual payload.
    payload: Option<T>,
}

impl<T> Default for PoolRecord<T> {
    fn default() -> Self {
        Self {
            generation: INVALID_GENERATION,
            payload: None,
        }
    }
}

impl<T> Visit for PoolRecord<T>
where
    T: Visit + Default + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.generation.visit("Generation", visitor)?;
        self.payload.visit("Payload", visitor)?;

        visitor.leave_region()
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Handle<T> {
        Handle {
            index: self.index,
            generation: self.generation,
            type_marker: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Eq for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Handle<T>) -> bool {
        self.generation == other.generation && self.index == other.index
    }
}

impl<T> Visit for Pool<T>
where
    T: Default + Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.records.visit("Records", visitor)?;
        self.free_stack.visit("FreeStack", visitor)?;
        visitor.leave_region()
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
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
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Ticket<T> {
    index: u32,
    marker: PhantomData<T>,
}

impl<T: Clone> Clone for PoolRecord<T> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation,
            payload: self.payload.clone(),
        }
    }
}

impl<T: Clone> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            records: self.records.clone(),
            free_stack: self.free_stack.clone(),
        }
    }
}

impl<T> Pool<T> {
    #[inline]
    pub fn new() -> Self {
        Pool {
            records: Vec::new(),
            free_stack: Vec::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Pool {
            records: Vec::with_capacity(capacity),
            free_stack: Vec::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn spawn(&mut self, payload: T) -> Handle<T> {
        if let Some(free_index) = self.free_stack.pop() {
            let record = &mut self.records[free_index as usize];

            if record.payload.is_some() {
                panic!(
                    "Attempt to spawn an object at pool record with payload! Record index is {}",
                    free_index
                );
            }

            record.generation += 1;
            record.payload.replace(payload);
            Handle {
                index: free_index,
                generation: record.generation,
                type_marker: PhantomData,
            }
        } else {
            // No free records, create new one
            let record = PoolRecord {
                generation: 1,
                payload: Some(payload),
            };

            let handle = Handle {
                index: self.records.len() as u32,
                generation: record.generation,
                type_marker: PhantomData,
            };

            self.records.push(record);

            handle
        }
    }

    /// Borrows shared reference to an object by its handle.
    ///
    /// # Panics
    ///
    /// Panics if handle is out of bounds or generation of handle does not match with
    /// generation of pool record at handle index (in other words it means that object
    /// at handle's index is different than the object was there before).
    #[inline]
    #[must_use]
    pub fn borrow(&self, handle: Handle<T>) -> &T {
        if let Some(record) = self.records.get(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(ref payload) = record.payload {
                    payload
                } else {
                    panic!("Attempt to borrow destroyed object at {:?} handle.", handle);
                }
            } else {
                panic!(
                    "Attempt to use dangling handle {:?}. Record has {} generation!",
                    handle, record.generation
                );
            }
        } else {
            panic!(
                "Attempt to borrow object using out-of-bounds handle {:?}! Record count is {}",
                handle,
                self.records.len()
            );
        }
    }

    /// Borrows mutable reference to an object by its handle.
    ///
    /// # Panics
    ///
    /// Panics if handle is out of bounds or generation of handle does not match with
    /// generation of pool record at handle index (in other words it means that object
    /// at handle's index is different than the object was there before).
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let a = pool.spawn(1);
    /// let a = pool.borrow_mut(a);
    /// *a = 11;
    /// ```
    #[inline]
    #[must_use]
    pub fn borrow_mut(&mut self, handle: Handle<T>) -> &mut T {
        let record_count = self.records.len();
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(ref mut payload) = record.payload {
                    payload
                } else {
                    panic!("Attempt to borrow destroyed object at {:?} handle.", handle);
                }
            } else {
                panic!("Attempt to borrow object using dangling handle {:?}. Record has {} generation!", handle, record.generation);
            }
        } else {
            panic!(
                "Attempt to borrow object using out-of-bounds handle {:?}! Record count is {}",
                handle, record_count
            );
        }
    }

    /// Borrows shared reference to an object by its handle.
    ///
    /// Returns None if handle is out of bounds or generation of handle does not match with
    /// generation of pool record at handle index (in other words it means that object
    /// at handle's index is different than the object was there before).
    #[inline]
    #[must_use]
    pub fn try_borrow(&self, handle: Handle<T>) -> Option<&T> {
        self.records.get(handle.index as usize).and_then(|r| {
            if r.generation == handle.generation {
                r.payload.as_ref()
            } else {
                None
            }
        })
    }

    /// Borrows mutable reference to an object by its handle.
    ///
    /// Returns None if handle is out of bounds or generation of handle does not match with
    /// generation of pool record at handle index (in other words it means that object
    /// at handle's index is different than the object was there before).
    #[inline]
    #[must_use]
    pub fn try_borrow_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.records.get_mut(handle.index as usize).and_then(|r| {
            if r.generation == handle.generation {
                r.payload.as_mut()
            } else {
                None
            }
        })
    }

    /// Borrows mutable references of objects at the same time. This method will succeed only
    /// if handles are unique (not equal). Borrowing multiple mutable references at the same
    /// time is useful in case if you need to mutate some objects at the same time.
    ///
    /// # Panics
    ///
    /// See [`borrow_mut`].
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let a = pool.spawn(1);
    /// let b = pool.spawn(2);
    /// let (a, b) = pool.borrow_two_mut((a, b));
    /// *a = 11;
    /// *b = 22;
    /// ```
    #[inline]
    #[must_use = "Handle set must not be ignored"]
    pub fn borrow_two_mut(&mut self, handles: (Handle<T>, Handle<T>)) -> (&mut T, &mut T) {
        // Prevent giving two mutable references to same record.
        assert_ne!(handles.0.index, handles.1.index);
        unsafe {
            let this = self as *mut Self;
            ((*this).borrow_mut(handles.0), (*this).borrow_mut(handles.1))
        }
    }

    /// Borrows mutable references of objects at the same time. This method will succeed only
    /// if handles are unique (not equal). Borrowing multiple mutable references at the same
    /// time is useful in case if you need to mutate some objects at the same time.
    ///
    /// # Panics
    ///
    /// See [`borrow_mut`].
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let a = pool.spawn(1);
    /// let b = pool.spawn(2);
    /// let c = pool.spawn(3);
    /// let (a, b, c) = pool.borrow_three_mut((a, b, c));
    /// *a = 11;
    /// *b = 22;
    /// *c = 33;
    /// ```
    #[inline]
    #[must_use = "Handle set must not be ignored"]
    pub fn borrow_three_mut(
        &mut self,
        handles: (Handle<T>, Handle<T>, Handle<T>),
    ) -> (&mut T, &mut T, &mut T) {
        // Prevent giving mutable references to same record.
        assert_ne!(handles.0.index, handles.1.index);
        assert_ne!(handles.0.index, handles.2.index);
        assert_ne!(handles.1.index, handles.2.index);
        unsafe {
            let this = self as *mut Self;
            (
                (*this).borrow_mut(handles.0),
                (*this).borrow_mut(handles.1),
                (*this).borrow_mut(handles.2),
            )
        }
    }

    /// Borrows mutable references of objects at the same time. This method will succeed only
    /// if handles are unique (not equal). Borrowing multiple mutable references at the same
    /// time is useful in case if you need to mutate some objects at the same time.
    ///
    /// # Panics
    ///
    /// See [`borrow_mut`].
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let a = pool.spawn(1);
    /// let b = pool.spawn(2);
    /// let c = pool.spawn(3);
    /// let d = pool.spawn(4);
    /// let (a, b, c, d) = pool.borrow_four_mut((a, b, c, d));
    /// *a = 11;
    /// *b = 22;
    /// *c = 33;
    /// *d = 44;
    /// ```
    #[inline]
    #[must_use = "Handle set must not be ignored"]
    pub fn borrow_four_mut(
        &mut self,
        handles: (Handle<T>, Handle<T>, Handle<T>, Handle<T>),
    ) -> (&mut T, &mut T, &mut T, &mut T) {
        // Prevent giving mutable references to same record.
        // This is kinda clunky since const generics are not stabilized yet.
        assert_ne!(handles.0.index, handles.1.index);
        assert_ne!(handles.0.index, handles.2.index);
        assert_ne!(handles.0.index, handles.3.index);
        assert_ne!(handles.1.index, handles.2.index);
        assert_ne!(handles.1.index, handles.3.index);
        assert_ne!(handles.2.index, handles.3.index);
        unsafe {
            let this = self as *mut Self;
            (
                (*this).borrow_mut(handles.0),
                (*this).borrow_mut(handles.1),
                (*this).borrow_mut(handles.2),
                (*this).borrow_mut(handles.3),
            )
        }
    }

    /// Tries to borrow two objects when a handle to the second object stored in the first object.
    pub fn try_borrow_dependant_mut<F>(
        &mut self,
        handle: Handle<T>,
        func: F,
    ) -> (Option<&mut T>, Option<&mut T>)
    where
        F: FnOnce(&T) -> Handle<T>,
    {
        let this = unsafe { &mut *(self as *mut Pool<T>) };
        let first = self.try_borrow_mut(handle);
        if let Some(first_object) = first.as_ref() {
            let second_handle = func(first_object);
            if second_handle != handle {
                return (first, this.try_borrow_mut(second_handle));
            }
        }

        (first, None)
    }

    /// Moves object out of pool using given handle. All handles to the object will become invalid.
    ///
    /// # Panics
    ///
    /// Panics if given handle is invalid.
    ///
    #[inline]
    pub fn free(&mut self, handle: Handle<T>) -> T {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                // Remember this index as free
                self.free_stack.push(handle.index);
                // Return current payload.
                if let Some(payload) = record.payload.take() {
                    payload
                } else {
                    panic!("Attempt to double free object at handle {:?}!", handle);
                }
            } else {
                panic!(
                    "Attempt to free object using dangling handle {:?}! Record generation is {}",
                    handle, record.generation
                );
            }
        } else {
            panic!("Attempt to free destroyed object using out-of-bounds handle {:?}! Record count is {}", handle, self.records.len());
        }
    }

    /// Moves object out of pool using given handle with a promise that object will be returned back.
    /// Returns pair (ticket, value). Ticket must be used to put value back!
    ///
    /// # Motivation
    ///
    /// This method is useful when you need to take temporary ownership of an object, do something
    /// with it and then put it back while keep all handles valid and be able to put new objects into
    /// pool without overriding a payload at its handle.
    ///
    /// # Notes
    ///
    /// All handles to the object will be invalid until object is returned in pool! Pool record will
    /// be reserved for further put_back call, which means if you lose ticket you will have empty
    /// "unusable" pool record forever.
    ///
    /// # Panics
    ///
    /// Panics if given handle is invalid.
    ///
    #[inline]
    pub fn take_reserve(&mut self, handle: Handle<T>) -> (Ticket<T>, T) {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(payload) = record.payload.take() {
                    let ticket = Ticket {
                        index: handle.index,
                        marker: PhantomData,
                    };
                    (ticket, payload)
                } else {
                    panic!(
                        "Attempt to take already taken object at handle {:?}!",
                        handle
                    );
                }
            } else {
                panic!(
                    "Attempt to take object using dangling handle {:?}! Record generation is {}",
                    handle, record.generation
                );
            }
        } else {
            panic!("Attempt to take destroyed object using out-of-bounds handle {:?}! Record count is {}", handle, self.records.len());
        }
    }

    /// Does same as take_reserve but returns option, instead of panic.
    #[inline]
    pub fn try_take_reserve(&mut self, handle: Handle<T>) -> Option<(Ticket<T>, T)> {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(payload) = record.payload.take() {
                    let ticket = Ticket {
                        index: handle.index,
                        marker: PhantomData,
                    };
                    Some((ticket, payload))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    /// Returns value back into pool using given ticket.
    ///
    /// # Panics
    ///
    /// In normal conditions it must never panic.
    pub fn put_back(&mut self, ticket: Ticket<T>, value: T) -> Handle<T> {
        let record = &mut self.records[ticket.index as usize];
        let old = record.payload.replace(value);
        assert!(old.is_none());
        Handle::new(ticket.index, record.generation)
    }

    /// Forgets that value at ticket was reserved and makes it usable again.
    /// Useful when you don't need to put value back by ticket, but just make
    /// pool record usable again.
    pub fn forget_ticket(&mut self, ticket: Ticket<T>) {
        self.free_stack.push(ticket.index);
    }

    /// Returns total capacity of pool. Capacity has nothing about real amount of objects in pool!
    #[inline]
    #[must_use]
    pub fn get_capacity(&self) -> usize {
        self.records.len()
    }

    /// Destroys all objects in pool. All handles to objects will become invalid.
    ///
    /// # Remarks
    ///
    /// Use this method cautiously if objects in pool have cross "references" (handles)
    /// to each other. This method will make all produced handles invalid and any further
    /// calls for [`borrow`] or [`borrow_mut`] will raise panic.
    ///
    #[inline]
    pub fn clear(&mut self) {
        self.records.clear();
        self.free_stack.clear();
    }

    #[inline]
    #[must_use]
    pub fn at_mut(&mut self, n: usize) -> Option<&mut T> {
        self.records.get_mut(n).and_then(|rec| rec.payload.as_mut())
    }

    #[inline]
    #[must_use]
    pub fn at(&self, n: usize) -> Option<&T> {
        self.records.get(n).and_then(|rec| rec.payload.as_ref())
    }

    #[inline]
    #[must_use]
    pub fn handle_from_index(&self, n: usize) -> Handle<T> {
        if let Some(record) = self.records.get(n) {
            if record.generation != INVALID_GENERATION {
                return Handle::new(n as u32, record.generation);
            }
        }
        Handle::NONE
    }

    /// Returns exact amount of "alive" objects in pool.
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// pool.spawn(123);
    /// pool.spawn(321);
    /// assert_eq!(pool.alive_count(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.iter().count()
    }

    #[inline]
    pub fn replace(&mut self, handle: Handle<T>, payload: T) -> Option<T> {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                self.free_stack.retain(|i| *i != handle.index);

                record.payload.replace(payload)
            } else {
                panic!("Attempt to replace object in pool using dangling handle! Handle is {:?}, but pool record has {} generation", handle, record.generation);
            }
        } else {
            None
        }
    }

    /// Checks if given handle "points" to some object.
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let handle = pool.spawn(123);
    /// assert_eq!(pool.is_valid_handle(handle), true)
    /// ```
    #[inline]
    pub fn is_valid_handle(&self, handle: Handle<T>) -> bool {
        if let Some(record) = self.records.get(handle.index as usize) {
            record.payload.is_some() && record.generation == handle.generation
        } else {
            false
        }
    }

    /// Creates new pool iterator that iterates over filled records in pool.
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// pool.spawn(123);
    /// pool.spawn(321);
    /// let mut iter = pool.iter();
    /// assert_eq!(*iter.next().unwrap(), 123);
    /// assert_eq!(*iter.next().unwrap(), 321);
    /// ```
    #[must_use]
    pub fn iter(&self) -> PoolIterator<T> {
        unsafe {
            PoolIterator {
                ptr: self.records.as_ptr(),
                end: self.records.as_ptr().add(self.records.len()),
                marker: PhantomData,
            }
        }
    }

    /// Creates new pair iterator that iterates over filled records using pair (handle, payload)
    /// Can be useful when there is a need to iterate over pool records and know a handle of
    /// that record.
    pub fn pair_iter(&self) -> PoolPairIterator<T> {
        PoolPairIterator {
            pool: self,
            current: 0,
        }
    }

    /// Creates new pool iterator that iterates over filled records in pool allowing
    /// to modify record payload.
    ///
    /// # Example
    ///
    /// ```
    /// use rg3d_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// pool.spawn(123);
    /// pool.spawn(321);
    /// let mut iter = pool.iter_mut();
    /// assert_eq!(*iter.next().unwrap(), 123);
    /// assert_eq!(*iter.next().unwrap(), 321);
    /// ```
    #[must_use]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<T> {
        unsafe {
            PoolIteratorMut {
                ptr: self.records.as_mut_ptr(),
                end: self.records.as_mut_ptr().add(self.records.len()),
                marker: PhantomData,
            }
        }
    }

    /// Creates new pair iterator that iterates over filled records using pair (handle, payload)
    /// Can be useful when there is a need to iterate over pool records and know a handle of
    /// that record.
    pub fn pair_iter_mut(&mut self) -> PoolPairIteratorMut<T> {
        unsafe {
            PoolPairIteratorMut {
                current: 0,
                ptr: self.records.as_mut_ptr(),
                end: self.records.as_mut_ptr().add(self.records.len()),
                marker: PhantomData,
            }
        }
    }

    /// Retains pool records selected by `pred`. Useful when you need to remove all pool records
    /// by some criteria.
    pub fn retain<F>(&mut self, mut pred: F)
    where
        F: FnMut(&T) -> bool,
    {
        for (i, record) in self.records.iter_mut().enumerate() {
            if record.generation == INVALID_GENERATION {
                continue;
            }

            let retain = if let Some(payload) = record.payload.as_ref() {
                pred(payload)
            } else {
                continue;
            };

            if !retain {
                self.free_stack.push(i as u32);
                record.payload.take(); // and Drop
            }
        }
    }

    fn end(&self) -> *const PoolRecord<T> {
        unsafe { self.records.as_ptr().add(self.records.len()) }
    }

    fn begin(&self) -> *const PoolRecord<T> {
        self.records.as_ptr()
    }

    pub fn handle_of(&self, ptr: &T) -> Handle<T> {
        let begin = self.begin() as usize;
        let end = self.end() as usize;
        let val = ptr as *const T as usize;
        if val >= begin && val < end {
            let record_size = std::mem::size_of::<PoolRecord<T>>();
            let record_location = (val - offset_of!(PoolRecord<T>, payload)) - begin;
            if record_location % record_size == 0 {
                return self.handle_from_index(record_location / record_size);
            }
        }
        Handle::NONE
    }
}

impl<T> FromIterator<T> for Pool<T> {
    fn from_iter<C: IntoIterator<Item = T>>(iter: C) -> Self {
        let iter = iter.into_iter();
        let (lower_bound, upper_bound) = iter.size_hint();
        let mut pool = Self::with_capacity(upper_bound.unwrap_or(lower_bound));
        for v in iter.into_iter() {
            let _ = pool.spawn(v);
        }
        pool
    }
}

impl<T> Index<Handle<T>> for Pool<T> {
    type Output = T;

    fn index(&self, index: Handle<T>) -> &Self::Output {
        self.borrow(index)
    }
}

impl<T> IndexMut<Handle<T>> for Pool<T> {
    fn index_mut(&mut self, index: Handle<T>) -> &mut Self::Output {
        self.borrow_mut(index)
    }
}

pub struct PoolIterator<'a, T> {
    ptr: *const PoolRecord<T>,
    end: *const PoolRecord<T>,
    marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for PoolIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.ptr != self.end {
                let current = &*self.ptr;
                if let Some(ref payload) = current.payload {
                    self.ptr = self.ptr.offset(1);
                    return Some(payload);
                }
                self.ptr = self.ptr.offset(1);
            }

            None
        }
    }
}

pub struct PoolPairIterator<'a, T> {
    pool: &'a Pool<T>,
    current: usize,
}

impl<'a, T> Iterator for PoolPairIterator<'a, T> {
    type Item = (Handle<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.pool.records.get(self.current) {
                Some(record) => {
                    if let Some(payload) = &record.payload {
                        let handle = self.pool.handle_from_index(self.current);
                        self.current += 1;
                        return Some((handle, payload));
                    }
                    self.current += 1;
                }
                None => return None,
            }
        }
    }
}

pub struct PoolIteratorMut<'a, T> {
    ptr: *mut PoolRecord<T>,
    end: *mut PoolRecord<T>,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for PoolIteratorMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.ptr != self.end {
                let current = &mut *self.ptr;
                if let Some(ref mut payload) = current.payload {
                    self.ptr = self.ptr.offset(1);
                    return Some(payload);
                }
                self.ptr = self.ptr.offset(1);
            }

            None
        }
    }
}

pub struct PoolPairIteratorMut<'a, T> {
    ptr: *mut PoolRecord<T>,
    end: *mut PoolRecord<T>,
    marker: PhantomData<&'a mut T>,
    current: usize,
}

impl<'a, T> Iterator for PoolPairIteratorMut<'a, T> {
    type Item = (Handle<T>, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.ptr != self.end {
                let current = &mut *self.ptr;
                if let Some(ref mut payload) = current.payload {
                    let handle = Handle::new(self.current as u32, current.generation);
                    self.ptr = self.ptr.offset(1);
                    self.current += 1;
                    return Some((handle, payload));
                }
                self.ptr = self.ptr.offset(1);
                self.current += 1;
            }

            None
        }
    }
}

#[cfg(test)]
mod test {
    use crate::pool::{Pool, INVALID_GENERATION};

    #[test]
    fn pool_sanity_tests() {
        let mut pool: Pool<String> = Pool::new();
        let foobar_handle = pool.spawn(String::from("Foobar"));
        assert_eq!(foobar_handle.index, 0);
        assert_ne!(foobar_handle.generation, INVALID_GENERATION);
        let foobar_handle_copy = foobar_handle.clone();
        assert_eq!(foobar_handle.index, foobar_handle_copy.index);
        assert_eq!(foobar_handle.generation, foobar_handle_copy.generation);
        let baz_handle = pool.spawn(String::from("Baz"));
        assert_eq!(pool.borrow(foobar_handle), "Foobar");
        assert_eq!(pool.borrow(baz_handle), "Baz");
        pool.free(foobar_handle);
        assert_eq!(pool.is_valid_handle(foobar_handle_copy), false);
        assert_eq!(pool.is_valid_handle(baz_handle), true);
        let at_foobar_index = pool.spawn(String::from("AtFoobarIndex"));
        assert_eq!(at_foobar_index.index, 0);
        assert_ne!(at_foobar_index.generation, INVALID_GENERATION);
        assert_eq!(pool.borrow(at_foobar_index), "AtFoobarIndex");
    }

    #[test]
    fn pool_iterator_mut_test() {
        let mut pool: Pool<String> = Pool::new();
        let foobar = pool.spawn(format!("Foobar"));
        let d = pool.spawn(format!("Foo"));
        pool.free(d);
        let baz = pool.spawn(format!("Baz"));
        for s in pool.iter_mut() {
            println!("{}", s);
        }
        pool.free(foobar);
        pool.free(baz);
    }

    #[test]
    fn handle_of() {
        struct Value {
            data: String,
        }

        let mut pool = Pool::new();
        let foobar = pool.spawn(Value {
            data: format!("Foobar"),
        });
        let bar = pool.spawn(Value {
            data: format!("Bar"),
        });
        let baz = pool.spawn(Value {
            data: format!("Baz"),
        });
        assert_eq!(pool.handle_of(pool.borrow(foobar)), foobar);
        assert_eq!(pool.handle_of(pool.borrow(bar)), bar);
        assert_eq!(pool.handle_of(pool.borrow(baz)), baz);
    }
}
