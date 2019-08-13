use std::{
    marker::PhantomData,
    hash::{Hash, Hasher},
};
use serde::{Serialize, Deserialize};
use std::cell::Cell;

///
/// RcPool allows to create as many objects as you want in contiguous memory
/// block. It allows to create and delete objects very fast.
///
/// Main difference between RcPool and Pool is that RcPool stores reference
/// count for every object in pool so it can be used as storage for shared
/// resources.
///
#[derive(Serialize, Deserialize)]
pub struct RcPool<T: Sized> {
    records: Vec<RcPoolRecord<T>>,
    free_stack: Vec<u32>,
}

impl<T> Default for RcPool<T> {
    fn default() -> Self {
        RcPool::new()
    }
}

///
/// Handle is some sort of non-owning reference to content in a pool.
/// It stores index of object and additional information that
/// allows to ensure that handle is still valid.
///
/// IMPORTANT: Handle is non-owning index of record in pool so it can't
/// automatically decrease reference count when it dies. Design your
/// program properly!
///
#[derive(Serialize, Deserialize)]
pub struct RcHandle<T> {
    /// Index of object in pool.
    index: u32,
    /// Generation number, if it is same as generation of pool record at
    /// index of handle then this is valid handle.
    generation: u32,
    /// Type holder.
    #[serde(skip)]
    type_marker: PhantomData<T>,
}

#[derive(Serialize, Deserialize)]
struct RcPoolRecord<T: Sized> {
    /// Generation number, used to keep info about lifetime.
    /// The handle is valid only if record it points to is of the
    /// same generation as the pool record.
    /// Notes: Zero is unknown generation used for None handles.
    generation: u32,
    /// Actual amount of handles created for this record.
    /// Record will die only if there is no handles left.
    ref_count: Cell<u32>,
    /// Actual payload.
    payload: Option<T>,
}

impl<T> PartialEq for RcHandle<T> {
    fn eq(&self, other: &RcHandle<T>) -> bool {
        self.generation == other.generation && self.index == other.index
    }
}

impl<T> Eq for RcHandle<T> {}

impl<T> Hash for RcHandle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> RcHandle<T> {
    #[inline]
    pub fn none() -> Self {
        RcHandle {
            index: 0,
            generation: RcPool::<T>::INVALID_GENERATION,
            type_marker: PhantomData,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.index == 0 && self.generation == RcPool::<T>::INVALID_GENERATION
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    fn make(index: u32, generation: u32) -> Self {
        RcHandle {
            index,
            generation,
            type_marker: PhantomData,
        }
    }
}

impl<T> RcPool<T> {
    const INVALID_GENERATION: u32 = 0;

    #[inline]
    #[must_use]
    pub fn new() -> Self {
        RcPool {
            records: Vec::<RcPoolRecord<T>>::new(),
            free_stack: Vec::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn spawn(&mut self, payload: T) -> RcHandle<T> {
        if let Some(free_index) = self.free_stack.pop() {
            let record = &mut self.records[free_index as usize];
            record.generation += 1;
            record.ref_count.set(1);
            record.payload.replace(payload);
            return RcHandle {
                index: free_index,
                generation: record.generation,
                type_marker: PhantomData,
            };
        }

        // No free records, create new one
        let record = RcPoolRecord {
            generation: 1,
            ref_count: Cell::new(1),
            payload: Some(payload),
        };

        let handle = RcHandle {
            index: self.records.len() as u32,
            generation: record.generation,
            type_marker: PhantomData,
        };

        self.records.push(record);

        handle
    }

    /// Creates copy of specified handle.
    /// Internally increases reference count of record.
    /// If input handle was invalid then RcHandle::none will be returned.
    #[must_use]
    pub fn share_handle(&self, handle: &RcHandle<T>) -> RcHandle<T> {
        if let Some(record) = self.records.get(handle.index as usize) {
            record.ref_count.set(record.ref_count.get() + 1);
            return RcHandle {
                index: handle.index,
                generation: handle.generation,
                type_marker: PhantomData,
            };
        }
        RcHandle::none()
    }

    #[inline]
    #[must_use]
    pub fn borrow(&self, handle: &RcHandle<T>) -> Option<&T> {
        if let Some(record) = self.records.get(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(payload) = &record.payload {
                    return Some(payload);
                } else {
                    println!("RcPool: Payload was empty!");
                }
            } else if handle.generation != RcPool::<T>::INVALID_GENERATION {
                println!("RcPool: Generation does not match: record has {} generation, but handle has {}", record.generation, handle.generation);
            }
        } else {
            println!("RcPool: Invalid index: got {}, but valid range is 0..{}", handle.index, self.records.len());
        }
        None
    }

    #[inline]
    #[must_use]
    pub fn borrow_mut(&mut self, handle: &RcHandle<T>) -> Option<&mut T> {
        let record_count = self.records.len();
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(payload) = &mut record.payload {
                    return Some(payload);
                } else {
                    println!("RcPool: Payload was empty!");
                }
            } else if handle.generation != RcPool::<T>::INVALID_GENERATION {
                println!("RcPool: Generation does not match: record has {} generation, but handle has {}", record.generation, handle.generation);
            }
        } else {
            println!("RcPool: Invalid index: got {}, but valid range is 0..{}", handle.index, record_count);
        }
        None
    }

    /// Decreases reference count of record and if it is zero - moves out of pool.
    #[inline]
    #[must_use]
    pub fn release(&mut self, handle: &RcHandle<T>) -> Option<T> {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                record.ref_count.set(record.ref_count.get() - 1);
                if record.ref_count.get() == 0 {
                    // Remember this index as free
                    self.free_stack.push(handle.index);
                    return record.payload.take();
                }
            } else if handle.generation != RcPool::<T>::INVALID_GENERATION {
                println!("RcPool: Generation does not match: record has {} generation, but handle has {}", record.generation, handle.generation);
            }
        } else {
            println!("RcPool: Invalid index: got {}, but valid range is 0..{}", handle.index, self.records.len());
        }
        None
    }

    pub fn replace(&mut self, handle: &RcHandle<T>, payload: T) -> Option<T> {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                return record.payload.replace(payload);
            } else if handle.generation != RcPool::<T>::INVALID_GENERATION {
                println!("RcPool: Generation does not match: record has {} generation, but handle has {}", record.generation, handle.generation);
            }
        } else {
            println!("RcPool: Invalid index: got {}, but valid range is 0..{}", handle.index, self.records.len());
        }
        None
    }

    #[inline]
    #[must_use]
    pub fn get_capacity(&self) -> usize {
        self.records.len()
    }

    #[inline]
    #[must_use]
    pub fn get_ref_count(&self, handle: &RcHandle<T>) -> Option<u32> {
        if let Some(record) = self.records.get(handle.index as usize) {
            Some(record.ref_count.get())
        } else {
            None
        }
    }

    #[inline]
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.records.iter().count()
    }

    #[inline]
    #[must_use]
    pub fn at_mut(&mut self, n: usize) -> Option<&mut T> {
        if let Some(record) = self.records.get_mut(n) {
            if let Some(ref mut payload) = record.payload {
                return Some(payload);
            }
        }
        None
    }

    #[inline]
    #[must_use]
    pub fn at(&self, n: usize) -> Option<&T> {
        if let Some(record) = self.records.get(n) {
            if let Some(ref payload) = record.payload {
                return Some(payload);
            }
        }
        None
    }

    #[inline]
    #[must_use]
    pub fn handle_from_index(&mut self, n: usize) -> RcHandle<T> {
        if let Some(record) = self.records.get_mut(n) {
            if record.generation != 0 {
                record.ref_count.set(record.ref_count.get() + 1);
                return RcHandle::make(n as u32, record.generation);
            }
        }
        RcHandle::none()
    }

    #[inline]
    #[must_use]
    pub fn is_valid_handle(&self, handle: &RcHandle<T>) -> bool {
        if let Some(record) = self.records.get(handle.index as usize) {
            return record.payload.is_some() && record.generation == handle.generation;
        }
        false
    }

    #[inline]
    #[must_use]
    pub fn iter(&self) -> RcPoolIterator<T> {
        RcPoolIterator {
            pool: self,
            current: 0,
        }
    }

    #[inline]
    #[must_use]
    pub fn iter_mut(&mut self) -> RcPoolIteratorMut<T> {
        unsafe {
            RcPoolIteratorMut {
                ptr: self.records.as_mut_ptr(),
                end: self.records.as_mut_ptr().offset(self.records.len() as isize),
                marker: PhantomData,
            }
        }
    }
}

pub struct RcPoolIterator<'a, T> {
    pool: &'a RcPool<T>,
    current: usize,
}

impl<'a, T> Iterator for RcPoolIterator<'a, T> {
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

pub struct RcPoolIteratorMut<'a, T> {
    ptr: *mut RcPoolRecord<T>,
    end: *mut RcPoolRecord<T>,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for RcPoolIteratorMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
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