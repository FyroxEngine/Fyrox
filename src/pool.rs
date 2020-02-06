use std::{
    marker::PhantomData,
    hash::{Hash, Hasher},
    fmt::{Debug, Formatter},
};
use crate::visitor::{
    Visit,
    VisitResult,
    Visitor,
};

const INVALID_GENERATION: u32 = 0;

/// Pool allows to create as many objects as you want in contiguous memory
/// block. It allows to create and delete objects much faster than if they'll
/// be allocated on heap. Also since objects stored in contiguous memory block
/// they can be effectively accessed because such memory layout if cache-friendly.
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

/// Type-erased handle.
#[derive(Copy, Clone, Debug)]
pub struct ErasedHandle {
    /// Index of object in pool.
    index: u32,
    /// Generation number, if it is same as generation of pool record at
    /// index of handle then this is valid handle.
    generation: u32,
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

impl<T> Into<ErasedHandle> for Handle<T> {
    fn into(self) -> ErasedHandle {
        ErasedHandle {
            index: self.index,
            generation: self.generation,
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

impl<T> Visit for PoolRecord<T> where T: Visit + Default + 'static {
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

impl<T> Visit for Pool<T> where T: Default + Visit + 'static {
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

    fn make(index: u32, generation: u32) -> Self {
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

impl<T> Pool<T> {
    #[inline]
    pub fn new() -> Self {
        Pool {
            records: Vec::<PoolRecord<T>>::new(),
            free_stack: Vec::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn spawn(&mut self, payload: T) -> Handle<T> {
        if let Some(free_index) = self.free_stack.pop() {
            let record = &mut self.records[free_index as usize];

            if record.payload.is_some() {
                panic!("Attempt to spawn an object at pool record with payload! Record index is {}", free_index);
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
                panic!("Attempt to use dangling handle {:?}. Record has {} generation!", handle, record.generation);
            }
        } else {
            panic!("Attempt to borrow object using out-of-bounds handle {:?}! Record count is {}", handle, self.records.len());
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
            panic!("Attempt to borrow object using out-of-bounds handle {:?}! Record count is {}", handle, record_count);
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
        self.records.get(handle.index as usize)
            .and_then(|r| if r.generation == handle.generation {
                r.payload.as_ref()
            } else {
                None
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
        self.records.get_mut(handle.index as usize)
            .and_then(|r| if r.generation == handle.generation {
                r.payload.as_mut()
            } else {
                None
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
    /// let (a, b) = pool.borrow_two_mut((a, b)).unwrap();
    /// *a = 11;
    /// *b = 22;
    /// ```
    #[inline]
    #[must_use = "Handle set must not be ignored"]
    pub fn borrow_two_mut(&mut self, handles: (Handle<T>, Handle<T>))
                          -> Result<(&mut T, &mut T), ()> {
        // Prevent giving two mutable references to same record.
        if handles.0.index != handles.1.index {
            unsafe {
                let this = self as *mut Self;
                Ok(((*this).borrow_mut(handles.0),
                    (*this).borrow_mut(handles.1)))
            }
        } else {
            Err(())
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
    /// let (a, b, c) = pool.borrow_three_mut((a, b, c)).unwrap();
    /// *a = 11;
    /// *b = 22;
    /// *c = 33;
    /// ```
    #[inline]
    #[must_use = "Handle set must not be ignored"]
    pub fn borrow_three_mut(&mut self, handles: (Handle<T>, Handle<T>, Handle<T>))
                            -> Result<(&mut T, &mut T, &mut T), ()> {
        // Prevent giving mutable references to same record.
        if handles.0.index != handles.1.index &&
            handles.0.index != handles.2.index &&
            handles.1.index != handles.2.index {
            unsafe {
                let this = self as *mut Self;
                Ok(((*this).borrow_mut(handles.0),
                    (*this).borrow_mut(handles.1),
                    (*this).borrow_mut(handles.2)))
            }
        } else {
            Err(())
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
    /// let (a, b, c, d) = pool.borrow_four_mut((a, b, c, d)).unwrap();
    /// *a = 11;
    /// *b = 22;
    /// *c = 33;
    /// *d = 44;
    /// ```
    #[inline]
    #[must_use = "Handle set must not be ignored"]
    pub fn borrow_four_mut(&mut self, handles: (Handle<T>, Handle<T>, Handle<T>, Handle<T>))
                           -> Result<(&mut T, &mut T, &mut T, &mut T), ()> {
        // Prevent giving mutable references to same record.
        // This is kinda clunky since const generics are not stabilized yet.
        if handles.0.index != handles.1.index &&
            handles.0.index != handles.2.index &&
            handles.0.index != handles.3.index &&
            handles.1.index != handles.2.index &&
            handles.1.index != handles.3.index &&
            handles.2.index != handles.3.index {
            unsafe {
                let this = self as *mut Self;
                Ok(((*this).borrow_mut(handles.0),
                    (*this).borrow_mut(handles.1),
                    (*this).borrow_mut(handles.2),
                    (*this).borrow_mut(handles.3)))
            }
        } else {
            Err(())
        }
    }

    /// Destroys object by given handle. All handles to the object will become invalid.
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
                // Move out payload and drop it so it will be destroyed
                if let Some(payload) = record.payload.take() {
                    payload
                } else {
                    panic!("Attempt to double free object at handle {:?}!", handle);
                }
            } else {
                panic!("Attempt to free object using dangling handle {:?}! Record generation is {}", handle, record.generation);
            }
        } else {
            panic!("Attempt to free destroyed object using out-of-bounds handle {:?}! Record count is {}", handle, self.records.len());
        }
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
        self.records
            .get_mut(n)
            .and_then(|rec| rec.payload.as_mut())
    }

    #[inline]
    #[must_use]
    pub fn at(&self, n: usize) -> Option<&T> {
        self.records
            .get(n)
            .and_then(|rec| rec.payload.as_ref())
    }

    #[inline]
    #[must_use]
    pub fn handle_from_index(&self, n: usize) -> Handle<T> {
        if let Some(record) = self.records.get(n) {
            if record.generation != INVALID_GENERATION {
                return Handle::make(n as u32, record.generation);
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
        self.records.iter().count()
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
        PoolIterator {
            pool: self,
            current: 0,
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
    pub fn retain<F>(&mut self, mut pred: F) where F: FnMut(&T) -> bool {
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
}

pub struct PoolIterator<'a, T> {
    pool: &'a Pool<T>,
    current: usize,
}

impl<'a, T> Iterator for PoolIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        loop {
            match self.pool.records.get(self.current) {
                Some(record) => {
                    if let Some(payload) = &record.payload {
                        self.current += 1;
                        return Some(payload);
                    }
                    self.current += 1;
                }
                None => {
                    return None;
                }
            }
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
                None => return None
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
                    let handle = Handle::make(self.current as u32, current.generation);
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
}