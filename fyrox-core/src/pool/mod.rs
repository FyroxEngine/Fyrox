//! A generational arena - a contiguous growable array type which allows removing
//! from the middle without shifting and therefore without invalidating other indices.
//!
//! Pool is a contiguous block of memory with fixed-size entries, each entry can be
//! either vacant or occupied. When you put an object into the pool you get a handle to
//! that object. You can use that handle later on to borrow a reference to an object.
//! A handle can point to some object or be invalid, this may look similar to raw
//! pointers, but there is two major differences:
//!
//! 1) We can check if a handle is valid before accessing the object it might point to.
//! 2) We can ensure the handle we're using is still valid for the object it points to
//! to make sure it hasn't been replaced with a different object on the same position.
//! Each handle stores a special field called generation which is shared across the entry
//! and the handle, so the handle is valid if these fields are the same on both the entry
//! and the handle. This protects from situations where you have a handle that has
//! a valid index of a record, but the payload in this record has been replaced.
//!
//! Contiguous memory block increases efficiency of memory operations - the CPU will
//! load portions of data into its cache piece by piece, it will be free from any
//! indirections that might cause cache invalidation. This is the so called cache
//! friendliness.

use crate::{reflect::prelude::*, visitor::prelude::*, ComponentProvider};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    ops::{Index, IndexMut},
    sync::atomic::{self, AtomicIsize},
};

pub mod handle;
pub mod multiborrow;
pub mod payload;

pub use handle::*;
pub use multiborrow::*;
pub use payload::*;

const INVALID_GENERATION: u32 = 0;

/// Pool allows to create as many objects as you want in contiguous memory
/// block. It allows to create and delete objects much faster than if they'll
/// be allocated on heap. Also since objects stored in contiguous memory block
/// they can be effectively accessed because such memory layout is cache-friendly.
#[derive(Debug)]
pub struct Pool<T, P = Option<T>>
where
    T: Sized,
    P: PayloadContainer<Element = T>,
{
    records: Vec<PoolRecord<T, P>>,
    free_stack: Vec<u32>,
}

impl<T, P> Reflect for Pool<T, P>
where
    T: Reflect,
    P: PayloadContainer<Element = T> + Reflect,
{
    #[inline]
    fn source_path() -> &'static str {
        file!()
    }

    #[inline]
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    #[inline]
    fn doc(&self) -> &'static str {
        ""
    }

    #[inline]
    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        func(&[])
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        func(self)
    }

    #[inline]
    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        func(self)
    }

    #[inline]
    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        func(self)
    }

    #[inline]
    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        func(self)
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let this = std::mem::replace(self, value.take()?);
        Ok(Box::new(this))
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    #[inline]
    fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
        func(Some(self))
    }

    #[inline]
    fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
        func(Some(self))
    }
}

impl<T, P> ReflectArray for Pool<T, P>
where
    T: Reflect,
    P: PayloadContainer<Element = T> + Reflect,
{
    #[inline]
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect> {
        self.at(index as u32).map(|p| p as &dyn Reflect)
    }

    #[inline]
    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.at_mut(index as u32).map(|p| p as &mut dyn Reflect)
    }

    #[inline]
    fn reflect_len(&self) -> usize {
        self.get_capacity() as usize
    }
}

impl<T, P> PartialEq for Pool<T, P>
where
    T: PartialEq,
    P: PayloadContainer<Element = T> + PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.records == other.records
    }
}

// Zero - non-borrowed.
// Negative values - amount of mutable borrows, positive - amount of immutable borrows.
#[derive(Default, Debug)]
struct RefCounter(pub AtomicIsize);

impl RefCounter {
    fn increment(&self) {
        self.0.fetch_add(1, atomic::Ordering::Relaxed);
    }

    fn decrement(&self) {
        self.0.fetch_sub(1, atomic::Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct PoolRecord<T, P = Option<T>>
where
    T: Sized,
    P: PayloadContainer<Element = T>,
{
    ref_counter: RefCounter,
    // Generation number, used to keep info about lifetime. The handle is valid
    // only if record it points to is of the same generation as the pool record.
    // Notes: Zero is unknown generation used for None handles.
    generation: u32,
    // Actual payload.
    payload: Payload<P>,
}

impl<T, P> PartialEq for PoolRecord<T, P>
where
    T: PartialEq,
    P: PayloadContainer<Element = T> + PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation && self.payload.get() == other.payload.get()
    }
}

impl<T, P> Default for PoolRecord<T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    #[inline]
    fn default() -> Self {
        Self {
            ref_counter: Default::default(),
            generation: INVALID_GENERATION,
            payload: Payload::new_empty(),
        }
    }
}

impl<T, P> Visit for PoolRecord<T, P>
where
    T: Visit + 'static,
    P: PayloadContainer<Element = T> + Visit,
{
    #[inline]
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.generation.visit("Generation", &mut region)?;
        self.payload.get_mut().visit("Payload", &mut region)?;

        Ok(())
    }
}

impl<T> Clone for Handle<T> {
    #[inline]
    fn clone(&self) -> Handle<T> {
        *self
    }
}

impl<T, P> Visit for Pool<T, P>
where
    T: Visit + 'static,
    P: PayloadContainer<Element = T> + Default + Visit + 'static,
{
    #[inline]
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;
        self.records.visit("Records", &mut region)?;
        self.free_stack.visit("FreeStack", &mut region)?;
        Ok(())
    }
}

impl<T, P> Default for Pool<T, P>
where
    T: 'static,
    P: PayloadContainer<Element = T> + 'static,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Ticket<T> {
    index: u32,
    marker: PhantomData<T>,
}

impl<T> Drop for Ticket<T> {
    fn drop(&mut self) {
        panic!(
            "An object at index {} must be returned to a pool it was taken from! \
            Call Pool::forget_ticket if you don't need the object anymore.",
            self.index
        )
    }
}

impl<T: Clone> Clone for PoolRecord<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            ref_counter: Default::default(),
            generation: self.generation,
            payload: self.payload.clone(),
        }
    }
}

impl<T: Clone> Clone for Pool<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            records: self.records.clone(),
            free_stack: self.free_stack.clone(),
        }
    }
}

impl<T, P> Pool<T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    #[inline]
    pub fn new() -> Self {
        Pool {
            records: Vec::new(),
            free_stack: Vec::new(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: u32) -> Self {
        let capacity = usize::try_from(capacity).expect("capacity overflowed usize");
        Pool {
            records: Vec::with_capacity(capacity),
            free_stack: Vec::new(),
        }
    }

    fn records_len(&self) -> u32 {
        u32::try_from(self.records.len()).expect("Number of records overflowed u32")
    }

    fn records_get(&self, index: u32) -> Option<&PoolRecord<T, P>> {
        let index = usize::try_from(index).expect("Index overflowed usize");
        self.records.get(index)
    }

    fn records_get_mut(&mut self, index: u32) -> Option<&mut PoolRecord<T, P>> {
        let index = usize::try_from(index).expect("Index overflowed usize");
        self.records.get_mut(index)
    }

    #[inline]
    #[must_use]
    pub fn spawn(&mut self, payload: T) -> Handle<T> {
        self.spawn_with(|_| payload)
    }

    /// Tries to put an object in the pool at given position. Returns `Err(payload)` if a corresponding
    /// entry is occupied.
    ///
    /// # Performance
    ///
    /// The method has O(n) complexity in worst case, where `n` - amount of free records in the pool.
    /// In typical uses cases `n` is very low. It should be noted that if a pool is filled entirely
    /// and you trying to put an object at the end of pool, the method will have O(1) complexity.
    ///
    /// # Panics
    ///
    /// Panics if the index is occupied or reserved (e.g. by [`take_reserve`]).
    ///
    /// [`take_reserve`]: Pool::take_reserve
    #[inline]
    pub fn spawn_at(&mut self, index: u32, payload: T) -> Result<Handle<T>, T> {
        self.spawn_at_internal(index, INVALID_GENERATION, payload)
    }

    /// Tries to put an object in the pool at given handle. Returns `Err(payload)` if a corresponding
    /// entry is occupied.
    ///
    /// # Performance
    ///
    /// The method has O(n) complexity in worst case, where `n` - amount of free records in the pool.
    /// In typical uses cases `n` is very low. It should be noted that if a pool is filled entirely
    /// and you trying to put an object at the end of pool, the method will have O(1) complexity.
    ///
    /// # Panics
    ///
    /// Panics if the index is occupied or reserved (e.g. by [`take_reserve`]).
    ///
    /// [`take_reserve`]: Pool::take_reserve
    #[inline]
    pub fn spawn_at_handle(&mut self, handle: Handle<T>, payload: T) -> Result<Handle<T>, T> {
        self.spawn_at_internal(handle.index, handle.generation, payload)
    }

    fn spawn_at_internal(
        &mut self,
        index: u32,
        desired_generation: u32,
        payload: T,
    ) -> Result<Handle<T>, T> {
        let index_usize = usize::try_from(index).expect("index overflowed usize");
        match self.records.get_mut(index_usize) {
            Some(record) => match record.payload.as_ref() {
                Some(_) => Err(payload),
                None => {
                    let position = self
                        .free_stack
                        .iter()
                        .rposition(|i| *i == index)
                        .expect("free_stack must contain the index of the empty record (most likely attempting to spawn at a reserved index)!");

                    self.free_stack.remove(position);

                    let generation = if desired_generation == INVALID_GENERATION {
                        record.generation + 1
                    } else {
                        desired_generation
                    };

                    record.generation = generation;
                    record.payload = Payload::new(payload);

                    Ok(Handle::new(index, generation))
                }
            },
            None => {
                // Spawn missing records to fill gaps.
                for i in self.records_len()..index {
                    self.records.push(PoolRecord {
                        ref_counter: Default::default(),
                        generation: 1,
                        payload: Payload::new_empty(),
                    });
                    self.free_stack.push(i);
                }

                let generation = if desired_generation == INVALID_GENERATION {
                    1
                } else {
                    desired_generation
                };

                self.records.push(PoolRecord {
                    ref_counter: Default::default(),
                    generation,
                    payload: Payload::new(payload),
                });

                Ok(Handle::new(index, generation))
            }
        }
    }

    #[inline]
    #[must_use]
    /// Construct a value with the handle it would be given.
    /// Note: Handle is _not_ valid until function has finished executing.
    pub fn spawn_with<F: FnOnce(Handle<T>) -> T>(&mut self, callback: F) -> Handle<T> {
        if let Some(free_index) = self.free_stack.pop() {
            let record = self
                .records_get_mut(free_index)
                .expect("free stack contained invalid index");

            if record.payload.is_some() {
                panic!(
                    "Attempt to spawn an object at pool record with payload! Record index is {}",
                    free_index
                );
            }

            let generation = record.generation + 1;
            let handle = Handle {
                index: free_index,
                generation,
                type_marker: PhantomData,
            };

            let payload = callback(handle);

            record.generation = generation;
            record.payload.replace(payload);
            handle
        } else {
            // No free records, create new one
            let generation = 1;

            let handle = Handle {
                index: self.records.len() as u32,
                generation,
                type_marker: PhantomData,
            };

            let payload = callback(handle);

            let record = PoolRecord {
                ref_counter: Default::default(),
                generation,
                payload: Payload::new(payload),
            };

            self.records.push(record);

            handle
        }
    }

    #[inline]
    /// Asynchronously construct a value with the handle it would be given.
    /// Note: Handle is _not_ valid until function has finished executing.
    pub async fn spawn_with_async<F, Fut>(&mut self, callback: F) -> Handle<T>
    where
        F: FnOnce(Handle<T>) -> Fut,
        Fut: Future<Output = T>,
    {
        if let Some(free_index) = self.free_stack.pop() {
            let record = self
                .records_get_mut(free_index)
                .expect("free stack contained invalid index");

            if record.payload.is_some() {
                panic!(
                    "Attempt to spawn an object at pool record with payload! Record index is {}",
                    free_index
                );
            }

            let generation = record.generation + 1;
            let handle = Handle {
                index: free_index,
                generation,
                type_marker: PhantomData,
            };

            let payload = callback(handle).await;

            record.generation = generation;
            record.payload.replace(payload);
            handle
        } else {
            // No free records, create new one
            let generation = 1;

            let handle = Handle {
                index: self.records.len() as u32,
                generation,
                type_marker: PhantomData,
            };

            let payload = callback(handle).await;

            let record = PoolRecord {
                generation,
                ref_counter: Default::default(),
                payload: Payload::new(payload),
            };

            self.records.push(record);

            handle
        }
    }

    /// Generates a set of handles that could be used to spawn a set of objects. This method does not
    /// modify the pool and the generated handles could be used together with [`Self::spawn_at_handle`]
    /// method.
    #[inline]
    pub fn generate_free_handles(&self, amount: usize) -> Vec<Handle<T>> {
        let mut free_handles = Vec::with_capacity(amount);
        free_handles.extend(
            self.free_stack
                .iter()
                .take(amount)
                .map(|i| Handle::new(*i, self.records[*i as usize].generation + 1)),
        );
        if free_handles.len() < amount {
            let remainder = amount - free_handles.len();
            free_handles.extend(
                (self.records.len()..self.records.len() + remainder)
                    .map(|i| Handle::new(i as u32, 1)),
            );
        }
        free_handles
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
        if let Some(record) = self.records_get(handle.index) {
            if record.generation == handle.generation {
                if let Some(payload) = record.payload.as_ref() {
                    payload
                } else {
                    panic!("Attempt to borrow destroyed object at {:?} handle.", handle);
                }
            } else {
                panic!(
                    "Attempt to use dangling handle {:?}. Record has generation {}!",
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
    /// use fyrox_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let a = pool.spawn(1);
    /// let a = pool.borrow_mut(a);
    /// *a = 11;
    /// ```
    #[inline]
    #[must_use]
    pub fn borrow_mut(&mut self, handle: Handle<T>) -> &mut T {
        let record_count = self.records.len();
        if let Some(record) = self.records_get_mut(handle.index) {
            if record.generation == handle.generation {
                if let Some(payload) = record.payload.as_mut() {
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
        self.records_get(handle.index).and_then(|r| {
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
        self.records_get_mut(handle.index).and_then(|r| {
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
    /// See [`borrow_mut`](Self::borrow_mut).
    ///
    /// # Example
    ///
    /// ```
    /// use fyrox_core::pool::Pool;
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
    /// See [`borrow_mut`](Self::borrow_mut).
    ///
    /// # Example
    ///
    /// ```
    /// use fyrox_core::pool::Pool;
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
    /// See [`borrow_mut`](Self::borrow_mut).
    ///
    /// # Example
    ///
    /// ```
    /// use fyrox_core::pool::Pool;
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
    #[inline]
    pub fn try_borrow_dependant_mut<F>(
        &mut self,
        handle: Handle<T>,
        func: F,
    ) -> (Option<&mut T>, Option<&mut T>)
    where
        F: FnOnce(&T) -> Handle<T>,
    {
        let this = unsafe { &mut *(self as *mut Pool<T, P>) };
        let first = self.try_borrow_mut(handle);
        if let Some(first_object) = first.as_ref() {
            let second_handle = func(first_object);
            if second_handle != handle {
                return (first, this.try_borrow_mut(second_handle));
            }
        }

        (first, None)
    }

    /// Moves object out of the pool using the given handle. All handles to the object will become invalid.
    ///
    /// # Panics
    ///
    /// Panics if the given handle is invalid.
    #[inline]
    pub fn free(&mut self, handle: Handle<T>) -> T {
        let index = usize::try_from(handle.index).expect("index overflowed usize");
        if let Some(record) = self.records.get_mut(index) {
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

    /// Tries to move object out of the pool using the given handle. Returns None if given handle
    /// is invalid. After object is moved out if the pool, all handles to the object will become
    /// invalid.
    #[inline]
    pub fn try_free(&mut self, handle: Handle<T>) -> Option<T> {
        let index = usize::try_from(handle.index).expect("index overflowed usize");
        self.records.get_mut(index).and_then(|record| {
            if record.generation == handle.generation {
                if let Some(payload) = record.payload.take() {
                    self.free_stack.push(handle.index);
                    Some(payload)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Moves an object out of the pool using the given handle with a promise that the object will be returned back.
    /// Returns pair (ticket, value). The ticket must be used to put the value back!
    ///
    /// # Motivation
    ///
    /// This method is useful when you need to take temporary ownership of an object, do something
    /// with it and then put it back while preserving all handles to it and being able to put new objects into
    /// the pool without overriding the payload at its handle.
    ///
    /// # Notes
    ///
    /// All handles to the object will be temporarily invalid until the object is returned to the pool! The pool record will
    /// be reserved for a further [`put_back`] call, which means if you lose the ticket you will have an empty
    /// "unusable" pool record forever.
    ///
    /// # Panics
    ///
    /// Panics if the given handle is invalid.
    ///
    /// [`put_back`]: Pool::put_back
    #[inline]
    pub fn take_reserve(&mut self, handle: Handle<T>) -> (Ticket<T>, T) {
        if let Some(record) = self.records_get_mut(handle.index) {
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

    /// Does the same as [`take_reserve`] but returns an option, instead of panicking.
    ///
    /// [`take_reserve`]: Pool::take_reserve
    #[inline]
    pub fn try_take_reserve(&mut self, handle: Handle<T>) -> Option<(Ticket<T>, T)> {
        if let Some(record) = self.records_get_mut(handle.index) {
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

    /// Returns the value back into the pool using the given ticket. See [`take_reserve`] for more
    /// information.
    ///
    /// [`take_reserve`]: Pool::take_reserve
    #[inline]
    pub fn put_back(&mut self, ticket: Ticket<T>, value: T) -> Handle<T> {
        let record = self
            .records_get_mut(ticket.index)
            .expect("Ticket index was invalid");
        let old = record.payload.replace(value);
        assert!(old.is_none());
        let handle = Handle::new(ticket.index, record.generation);
        std::mem::forget(ticket);
        handle
    }

    /// Forgets that value at ticket was reserved and makes it usable again.
    /// Useful when you don't need to put value back by ticket, but just make
    /// pool record usable again.
    #[inline]
    pub fn forget_ticket(&mut self, ticket: Ticket<T>) {
        self.free_stack.push(ticket.index);
        std::mem::forget(ticket);
    }

    /// Returns total capacity of pool. Capacity has nothing about real amount of objects in pool!
    #[inline]
    #[must_use]
    pub fn get_capacity(&self) -> u32 {
        u32::try_from(self.records.len()).expect("records.len() overflowed u32")
    }

    /// Destroys all objects in pool. All handles to objects will become invalid.
    ///
    /// # Remarks
    ///
    /// Use this method cautiously if objects in pool have cross "references" (handles)
    /// to each other. This method will make all produced handles invalid and any further
    /// calls for [`borrow`](Self::borrow) or [`borrow_mut`](Self::borrow_mut) will raise panic.
    #[inline]
    pub fn clear(&mut self) {
        self.records.clear();
        self.free_stack.clear();
    }

    #[inline]
    #[must_use]
    pub fn at_mut(&mut self, n: u32) -> Option<&mut T> {
        self.records_get_mut(n).and_then(|rec| rec.payload.as_mut())
    }

    #[inline]
    #[must_use]
    pub fn at(&self, n: u32) -> Option<&T> {
        self.records_get(n)
            .and_then(|rec| rec.payload.get().as_ref())
    }

    #[inline]
    #[must_use]
    pub fn handle_from_index(&self, n: u32) -> Handle<T> {
        if let Some(record) = self.records_get(n) {
            if record.generation != INVALID_GENERATION {
                return Handle::new(n, record.generation);
            }
        }
        Handle::NONE
    }

    /// Returns the exact number of "alive" objects in the pool.
    ///
    /// Records that have been reserved (e.g. by [`take_reserve`]) are *not* counted.
    ///
    /// It iterates through the entire pool to count the live objects so the complexity is `O(n)`.
    ///
    /// See also [`total_count`].
    ///
    /// # Example
    ///
    /// ```
    /// use fyrox_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// pool.spawn(123);
    /// pool.spawn(321);
    /// assert_eq!(pool.alive_count(), 2);
    /// ```
    ///
    /// [`take_reserve`]: Pool::take_reserve
    /// [`total_count`]: Pool::total_count
    #[inline]
    #[must_use]
    pub fn alive_count(&self) -> u32 {
        let cnt = self.iter().count();
        u32::try_from(cnt).expect("alive_count overflowed u32")
    }

    /// Returns the number of allocated objects in the pool.
    ///
    /// It also counts records that have been reserved (e.g. by [`take_reserve`]).
    ///
    /// This method is `O(1)`.
    ///
    /// See also [`alive_count`].
    ///
    /// [`take_reserve`]: Pool::take_reserve
    /// [`alive_count`]: Pool::alive_count
    #[inline]
    pub fn total_count(&self) -> u32 {
        let free = u32::try_from(self.free_stack.len()).expect("free stack length overflowed u32");
        self.records_len() - free
    }

    #[inline]
    pub fn replace(&mut self, handle: Handle<T>, payload: T) -> Option<T> {
        let index_usize = usize::try_from(handle.index).expect("index overflowed usize");
        if let Some(record) = self.records.get_mut(index_usize) {
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

    /// Returns a reference to the first element in the pool (if any).
    pub fn first_ref(&self) -> Option<&T> {
        self.iter().next()
    }

    /// Returns a reference to the first element in the pool (if any).
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.iter_mut().next()
    }

    /// Checks if given handle "points" to some object.
    ///
    /// # Example
    ///
    /// ```
    /// use fyrox_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// let handle = pool.spawn(123);
    /// assert_eq!(pool.is_valid_handle(handle), true)
    /// ```
    #[inline]
    pub fn is_valid_handle(&self, handle: Handle<T>) -> bool {
        if let Some(record) = self.records_get(handle.index) {
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
    /// use fyrox_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// pool.spawn(123);
    /// pool.spawn(321);
    /// let mut iter = pool.iter();
    /// assert_eq!(*iter.next().unwrap(), 123);
    /// assert_eq!(*iter.next().unwrap(), 321);
    /// ```
    #[must_use]
    #[inline]
    pub fn iter(&self) -> PoolIterator<T, P> {
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
    #[inline]
    pub fn pair_iter(&self) -> PoolPairIterator<T, P> {
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
    /// use fyrox_core::pool::Pool;
    /// let mut pool = Pool::<u32>::new();
    /// pool.spawn(123);
    /// pool.spawn(321);
    /// let mut iter = pool.iter_mut();
    /// assert_eq!(*iter.next().unwrap(), 123);
    /// assert_eq!(*iter.next().unwrap(), 321);
    /// ```
    #[must_use]
    #[inline]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<T, P> {
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
    #[inline]
    pub fn pair_iter_mut(&mut self) -> PoolPairIteratorMut<T, P> {
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
    #[inline]
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

    /// Begins multi-borrow that allows you to borrow as many (`N`) **unique** references to the pool
    /// elements as you need. See [`MultiBorrowContext::try_get`] for more info.
    #[inline]
    pub fn begin_multi_borrow(&mut self) -> MultiBorrowContext<T, P> {
        MultiBorrowContext::new(self)
    }

    /// Removes all elements from the pool.
    #[inline]
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.free_stack.clear();
        self.records.drain(..).filter_map(|mut r| r.payload.take())
    }

    fn end(&self) -> *const PoolRecord<T, P> {
        unsafe { self.records.as_ptr().add(self.records.len()) }
    }

    fn begin(&self) -> *const PoolRecord<T, P> {
        self.records.as_ptr()
    }

    #[inline]
    pub fn handle_of(&self, ptr: &T) -> Handle<T> {
        let begin = self.begin() as usize;
        let end = self.end() as usize;
        let val = ptr as *const T as usize;
        if val >= begin && val < end {
            let record_size = std::mem::size_of::<PoolRecord<T>>();
            let record_location = (val - offset_of!(PoolRecord<T>, payload)) - begin;
            if record_location % record_size == 0 {
                let index = record_location / record_size;
                let index = u32::try_from(index).expect("Index overflowed u32");
                return self.handle_from_index(index);
            }
        }
        Handle::NONE
    }
}

impl<T, P> Pool<T, P>
where
    T: ComponentProvider,
    P: PayloadContainer<Element = T> + 'static,
{
    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component_of_type<C>(&self, handle: Handle<T>) -> Option<&C>
    where
        C: 'static,
    {
        self.try_borrow(handle)
            .and_then(|n| n.query_component_ref(TypeId::of::<C>()))
            .and_then(|c| c.downcast_ref())
    }

    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component_of_type_mut<C>(&mut self, handle: Handle<T>) -> Option<&mut C>
    where
        C: 'static,
    {
        self.try_borrow_mut(handle)
            .and_then(|n| n.query_component_mut(TypeId::of::<C>()))
            .and_then(|c| c.downcast_mut())
    }
}

impl<T> FromIterator<T> for Pool<T>
where
    T: 'static,
{
    #[inline]
    fn from_iter<C: IntoIterator<Item = T>>(iter: C) -> Self {
        let iter = iter.into_iter();
        let (lower_bound, upper_bound) = iter.size_hint();
        let lower_bound = u32::try_from(lower_bound).expect("lower_bound overflowed u32");
        let upper_bound =
            upper_bound.map(|b| u32::try_from(b).expect("upper_bound overflowed u32"));
        let mut pool = Self::with_capacity(upper_bound.unwrap_or(lower_bound));
        for v in iter {
            let _ = pool.spawn(v);
        }
        pool
    }
}

impl<T, P> Index<Handle<T>> for Pool<T, P>
where
    T: 'static,
    P: PayloadContainer<Element = T> + 'static,
{
    type Output = T;

    #[inline]
    fn index(&self, index: Handle<T>) -> &Self::Output {
        self.borrow(index)
    }
}

impl<T, P> IndexMut<Handle<T>> for Pool<T, P>
where
    T: 'static,
    P: PayloadContainer<Element = T> + 'static,
{
    #[inline]
    fn index_mut(&mut self, index: Handle<T>) -> &mut Self::Output {
        self.borrow_mut(index)
    }
}

impl<'a, T, P> IntoIterator for &'a Pool<T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    type Item = &'a T;
    type IntoIter = PoolIterator<'a, T, P>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, P> IntoIterator for &'a mut Pool<T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    type Item = &'a mut T;
    type IntoIter = PoolIteratorMut<'a, T, P>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct PoolIterator<'a, T, P>
where
    P: PayloadContainer<Element = T>,
{
    ptr: *const PoolRecord<T, P>,
    end: *const PoolRecord<T, P>,
    marker: PhantomData<&'a T>,
}

impl<'a, T, P> Iterator for PoolIterator<'a, T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.ptr != self.end {
                let current = &*self.ptr;
                if let Some(payload) = current.payload.as_ref() {
                    self.ptr = self.ptr.offset(1);
                    return Some(payload);
                }
                self.ptr = self.ptr.offset(1);
            }

            None
        }
    }
}

pub struct PoolPairIterator<'a, T, P: PayloadContainer<Element = T>> {
    pool: &'a Pool<T, P>,
    current: usize,
}

impl<'a, T, P> Iterator for PoolPairIterator<'a, T, P>
where
    P: PayloadContainer<Element = T>,
{
    type Item = (Handle<T>, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.pool.records.get(self.current) {
                Some(record) => {
                    if let Some(payload) = record.payload.as_ref() {
                        let handle = Handle::new(self.current as u32, record.generation);
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

pub struct PoolIteratorMut<'a, T, P>
where
    P: PayloadContainer<Element = T>,
{
    ptr: *mut PoolRecord<T, P>,
    end: *mut PoolRecord<T, P>,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T, P> Iterator for PoolIteratorMut<'a, T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.ptr != self.end {
                let current = &mut *self.ptr;
                if let Some(payload) = current.payload.as_mut() {
                    self.ptr = self.ptr.offset(1);
                    return Some(payload);
                }
                self.ptr = self.ptr.offset(1);
            }

            None
        }
    }
}

pub struct PoolPairIteratorMut<'a, T, P>
where
    P: PayloadContainer<Element = T>,
{
    ptr: *mut PoolRecord<T, P>,
    end: *mut PoolRecord<T, P>,
    marker: PhantomData<&'a mut T>,
    current: usize,
}

impl<'a, T, P> Iterator for PoolPairIteratorMut<'a, T, P>
where
    P: PayloadContainer<Element = T> + 'static,
{
    type Item = (Handle<T>, &'a mut T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            while self.ptr != self.end {
                let current = &mut *self.ptr;
                if let Some(payload) = current.payload.as_mut() {
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
    use crate::{
        pool::{AtomicHandle, Handle, Pool, PoolRecord, INVALID_GENERATION},
        visitor::{Visit, Visitor},
    };

    #[test]
    fn pool_sanity_tests() {
        let mut pool: Pool<String> = Pool::new();
        let foobar_handle = pool.spawn(String::from("Foobar"));
        assert_eq!(foobar_handle.index, 0);
        assert_ne!(foobar_handle.generation, INVALID_GENERATION);
        let foobar_handle_copy = foobar_handle;
        assert_eq!(foobar_handle.index, foobar_handle_copy.index);
        assert_eq!(foobar_handle.generation, foobar_handle_copy.generation);
        let baz_handle = pool.spawn(String::from("Baz"));
        assert_eq!(pool.borrow(foobar_handle), "Foobar");
        assert_eq!(pool.borrow(baz_handle), "Baz");
        pool.free(foobar_handle);
        assert!(!pool.is_valid_handle(foobar_handle_copy));
        assert!(pool.is_valid_handle(baz_handle));
        let at_foobar_index = pool.spawn(String::from("AtFoobarIndex"));
        assert_eq!(at_foobar_index.index, 0);
        assert_ne!(at_foobar_index.generation, INVALID_GENERATION);
        assert_eq!(pool.borrow(at_foobar_index), "AtFoobarIndex");
        let bar_handle = pool.spawn_with(|_handle| String::from("Bar"));
        assert_ne!(bar_handle.index, 0);
        assert_ne!(bar_handle.generation, INVALID_GENERATION);
        assert_eq!(pool.borrow(bar_handle), "Bar");
    }

    #[test]
    fn pool_iterator_mut_test() {
        let mut pool: Pool<String> = Pool::new();
        let foobar = pool.spawn("Foobar".to_string());
        let d = pool.spawn("Foo".to_string());
        pool.free(d);
        let baz = pool.spawn("Baz".to_string());
        for s in pool.iter() {
            println!("{}", s);
        }
        for s in pool.iter_mut() {
            println!("{}", s);
        }
        for s in &pool {
            println!("{}", s);
        }
        for s in &mut pool {
            println!("{}", s);
        }
        pool.free(foobar);
        pool.free(baz);
    }

    #[test]
    fn handle_of() {
        #[allow(dead_code)]
        struct Value {
            data: String,
        }

        let mut pool = Pool::<Value>::new();
        let foobar = pool.spawn(Value {
            data: "Foobar".to_string(),
        });
        let bar = pool.spawn(Value {
            data: "Bar".to_string(),
        });
        let baz = pool.spawn(Value {
            data: "Baz".to_string(),
        });
        assert_eq!(pool.handle_of(pool.borrow(foobar)), foobar);
        assert_eq!(pool.handle_of(pool.borrow(bar)), bar);
        assert_eq!(pool.handle_of(pool.borrow(baz)), baz);
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Payload;

    #[test]
    fn pool_test_spawn_at() {
        let mut pool = Pool::<Payload>::new();

        assert_eq!(pool.spawn_at(2, Payload), Ok(Handle::new(2, 1)));
        assert_eq!(pool.spawn_at(2, Payload), Err(Payload));
        assert_eq!(pool.records[0].payload.as_ref(), None);
        assert_eq!(pool.records[1].payload.as_ref(), None);
        assert_ne!(pool.records[2].payload.as_ref(), None);

        assert_eq!(pool.spawn_at(2, Payload), Err(Payload));

        pool.free(Handle::new(2, 1));

        assert_eq!(pool.spawn_at(2, Payload), Ok(Handle::new(2, 2)));

        assert_eq!(pool.spawn(Payload), Handle::new(1, 2));
        assert_eq!(pool.spawn(Payload), Handle::new(0, 2));
    }

    #[test]
    fn pool_test_try_free() {
        let mut pool = Pool::<Payload>::new();

        assert_eq!(pool.try_free(Handle::NONE), None);
        assert_eq!(pool.free_stack.len(), 0);

        let handle = pool.spawn(Payload);
        assert_eq!(pool.try_free(handle), Some(Payload));
        assert_eq!(pool.free_stack.len(), 1);
        assert_eq!(pool.try_free(handle), None);
        assert_eq!(pool.free_stack.len(), 1);
    }

    #[test]
    fn visit_for_pool_record() {
        let mut p = PoolRecord::<u32>::default();
        let mut visitor = Visitor::default();

        assert!(p.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn visit_for_pool() {
        let mut p = Pool::<u32>::default();
        let mut visitor = Visitor::default();

        assert!(p.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn default_for_pool() {
        assert_eq!(Pool::default(), Pool::<u32>::new());
    }

    #[test]
    fn pool_with_capacity() {
        let p = Pool::<u32>::with_capacity(1);
        assert_eq!(p.records, Vec::with_capacity(1));
        assert_eq!(p.free_stack, Vec::new())
    }

    #[test]
    fn pool_try_borrow() {
        let mut pool = Pool::<Payload>::new();
        let a = pool.spawn(Payload);
        let b = Handle::<Payload>::default();

        assert_eq!(pool.try_borrow(a), Some(&Payload));
        assert_eq!(pool.try_borrow(b), None);
    }

    #[test]
    fn pool_borrow_two_mut() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(1);
        let b = pool.spawn(2);
        let (a, b) = pool.borrow_two_mut((a, b));

        assert_eq!(a, &mut 1);
        assert_eq!(b, &mut 2);
    }

    #[test]
    fn pool_borrow_three_mut() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(1);
        let b = pool.spawn(2);
        let c = pool.spawn(3);
        let (a, b, c) = pool.borrow_three_mut((a, b, c));

        assert_eq!(a, &mut 1);
        assert_eq!(b, &mut 2);
        assert_eq!(c, &mut 3);
    }

    #[test]
    fn pool_borrow_four_mut() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(1);
        let b = pool.spawn(2);
        let c = pool.spawn(3);
        let d = pool.spawn(4);
        let (a, b, c, d) = pool.borrow_four_mut((a, b, c, d));

        assert_eq!(a, &mut 1);
        assert_eq!(b, &mut 2);
        assert_eq!(c, &mut 3);
        assert_eq!(d, &mut 4);
    }

    #[test]
    fn pool_try_borrow_dependant_mut() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let b = pool.spawn(5);

        assert_eq!(
            pool.try_borrow_dependant_mut(a, |_| b),
            (Some(&mut 42), Some(&mut 5))
        );

        assert_eq!(
            pool.try_borrow_dependant_mut(a, |_| a),
            (Some(&mut 42), None)
        );
    }

    #[test]
    fn pool_try_take_reserve() {
        let mut pool = Pool::<u32>::new();

        let a = Handle::<u32>::default();
        assert!(pool.try_take_reserve(a).is_none());

        let b = pool.spawn(42);

        let (ticket, payload) = pool.try_take_reserve(b).unwrap();
        assert_eq!(ticket.index, 0);
        assert_eq!(payload, 42);

        assert!(pool.try_take_reserve(a).is_none());
        assert!(pool.try_take_reserve(b).is_none());

        pool.forget_ticket(ticket);
    }

    #[test]
    fn pool_put_back() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let (ticket, value) = pool.take_reserve(a);
        let b = pool.put_back(ticket, value);

        assert_eq!(a, b);
    }

    #[test]
    fn pool_forget_ticket() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let (ticket, _) = pool.take_reserve(a);

        pool.forget_ticket(ticket);

        let b = pool.spawn(42);

        assert_eq!(a.index, b.index);
        assert_ne!(a.generation, b.generation);
    }

    #[test]
    fn pool_get_capacity() {
        let mut pool = Pool::<u32>::new();
        let _ = pool.spawn(42);
        let _ = pool.spawn(5);

        assert_eq!(pool.get_capacity(), 2);
    }

    #[test]
    fn pool_clear() {
        let mut pool = Pool::<u32>::new();
        let _ = pool.spawn(42);

        assert!(!pool.records.is_empty());

        pool.clear();

        assert!(pool.records.is_empty());
        assert!(pool.free_stack.is_empty());
    }

    #[test]
    fn pool_at_mut() {
        let mut pool = Pool::<u32>::new();
        let _ = pool.spawn(42);

        assert_eq!(pool.at_mut(0), Some(&mut 42));
        assert_eq!(pool.at_mut(1), None);
    }

    #[test]
    fn pool_at() {
        let mut pool = Pool::<u32>::new();
        let _ = pool.spawn(42);

        assert_eq!(pool.at(0), Some(&42));
        assert_eq!(pool.at(1), None);
    }

    #[test]
    fn pool_handle_from_index() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);

        assert_eq!(pool.handle_from_index(0), a);
        assert_eq!(pool.handle_from_index(1), Handle::NONE);
    }

    #[test]
    fn pool_alive_count() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let _ = pool.spawn(5);
        let (ticket, _) = pool.take_reserve(a);
        pool.forget_ticket(ticket);

        assert_eq!(pool.alive_count(), 1);
    }

    #[test]
    fn pool_total_count() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let _ = pool.spawn(5);
        let (ticket, _) = pool.take_reserve(a);

        assert_eq!(pool.total_count(), 2);

        pool.forget_ticket(ticket);
    }

    #[test]
    fn pool_replace() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let b = Handle::<u32>::new(1, 1);

        assert_eq!(pool.replace(a, 5), Some(42));
        assert_eq!(pool.replace(b, 5), None);
    }

    #[test]
    fn pool_pair_iter() {
        let pool = Pool::<u32>::new();

        let iter = pool.pair_iter();

        assert_eq!(iter.pool, &pool);
        assert_eq!(iter.current, 0);
    }

    #[test]
    fn pool_pair_iter_mut() {
        let mut pool = Pool::<u32>::new();
        let _ = pool.spawn(42);

        let iter = pool.pair_iter_mut();

        assert_eq!(iter.current, 0);
        assert_eq!(iter.ptr, pool.records.as_mut_ptr());
    }

    #[test]
    fn index_for_pool() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let b = pool.spawn(5);

        assert_eq!(pool[a], 42);
        assert_eq!(pool[b], 5);
    }

    #[test]
    fn index_mut_for_pool() {
        let mut pool = Pool::<u32>::new();
        let a = pool.spawn(42);
        let b = pool.spawn(5);

        pool[a] = 15;

        assert_eq!(pool[a], 15);
        assert_eq!(pool[b], 5);
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

    #[test]
    fn test_generate_free_handles() {
        let mut pool = Pool::<u32>::new();

        let _ = pool.spawn(42);
        let b = pool.spawn(5);
        let _ = pool.spawn(228);

        pool.free(b);

        let h0 = Handle::new(1, 2);
        let h1 = Handle::new(3, 1);
        let h2 = Handle::new(4, 1);
        let h3 = Handle::new(5, 1);
        let h4 = Handle::new(6, 1);

        let free_handles = pool.generate_free_handles(5);
        assert_eq!(free_handles, [h0, h1, h2, h3, h4]);

        // Spawn something on the generated handles.
        for (i, handle) in free_handles.into_iter().enumerate() {
            let instance_handle = pool.spawn_at_handle(handle, i as u32);
            assert_eq!(instance_handle, Ok(handle));
        }

        assert_eq!(pool[h0], 0);
        assert_eq!(pool[h1], 1);
        assert_eq!(pool[h2], 2);
        assert_eq!(pool[h3], 3);
        assert_eq!(pool[h4], 4);
    }
}
