use std::{
    marker::PhantomData,
    hash::{Hash, Hasher},
};
use serde::{Serialize, Deserialize};

///
/// Pool allows to create as many objects as you want in contiguous memory
/// block. It allows to create and delete objects very fast.
///
#[derive(Serialize, Deserialize)]
pub struct Pool<T: Sized> {
    records: Vec<PoolRecord<T>>,
    free_stack: Vec<u32>,
}

///
/// Handle is some sort of non-owning reference to content in a pool.
/// It stores index of object and additional information that
/// allows to ensure that handle is still valid.
///
#[derive(Serialize, Deserialize)]
pub struct Handle<T> {
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
struct PoolRecord<T: Sized> {
    /// Generation number, used to keep info about lifetime.
    /// The handle is valid only if record it points to is of the
    /// same generation as the pool record.
    /// Notes: Zero is unknown generation used for None handles.
    generation: u32,
    /// Actual payload.
    payload: Option<T>,
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

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Handle<T>) -> bool {
        self.generation == other.generation && self.index == other.index
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> Handle<T> {
    #[inline]
    pub fn none() -> Self {
        Handle {
            index: 0,
            generation: Pool::<T>::INVALID_GENERATION,
            type_marker: PhantomData,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.index == 0 && self.generation == Pool::<T>::INVALID_GENERATION
    }

    fn make(index: u32, generation: u32) -> Self {
        Handle {
            index,
            generation,
            type_marker: PhantomData,
        }
    }
}

impl<T> Pool<T> {
    const INVALID_GENERATION: u32 = 0;

    #[inline]
    pub fn new() -> Self {
        Pool {
            records: Vec::<PoolRecord<T>>::new(),
            free_stack: Vec::new(),
        }
    }

    #[inline]
    pub fn spawn(&mut self, payload: T) -> Handle<T> {
        if let Some(free_index) = self.free_stack.pop() {
            let record = &mut self.records[free_index as usize];
            record.generation += 1;
            record.payload.replace(payload);
            return Handle {
                index: free_index,
                generation: record.generation,
                type_marker: PhantomData,
            };
        }

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

    #[inline]
    pub fn borrow(&self, handle: &Handle<T>) -> Option<&T> {
        if let Some(record) = self.records.get(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(payload) = &record.payload {
                    return Some(payload);
                }
            }
        }
        None
    }

    #[inline]
    pub fn borrow_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            if record.generation == handle.generation {
                if let Some(payload) = &mut record.payload {
                    return Some(payload);
                }
            }
        }
        None
    }

    #[inline]
    pub fn free(&mut self, handle: Handle<T>) {
        if let Some(record) = self.records.get_mut(handle.index as usize) {
            // Remember this index as free
            self.free_stack.push(handle.index);
            // Move out payload and drop it so it will be destroyed
            record.payload.take();
        }
    }

    #[inline]
    pub fn get_capacity(&self) -> usize {
        self.records.len()
    }

    #[inline]
    pub fn at_mut(&mut self, n: usize) -> Option<&mut T> {
        if let Some(record) = self.records.get_mut(n) {
            if let Some(ref mut payload) = record.payload {
                return Some(payload);
            }
        }
        None
    }

    #[inline]
    pub fn at(&self, n: usize) -> Option<&T> {
        if let Some(record) = self.records.get(n) {
            if let Some(ref payload) = record.payload {
                return Some(payload);
            }
        }
        None
    }

    #[inline]
    pub fn handle_from_index(&self, n: usize) -> Handle<T> {
        if let Some(record) = self.records.get(n) {
            if record.generation != 0 {
                return Handle::make(n as u32, record.generation);
            }
        }
        Handle::none()
    }

    #[inline]
    pub fn is_valid_handle(&self, handle: &Handle<T>) -> bool {
        if let Some(record) = self.records.get(handle.index as usize) {
            return record.payload.is_some() && record.generation == handle.generation;
        }
        false
    }

    pub fn iter(&self) -> PoolIterator<T> {
        PoolIterator {
            pool: self,
            current: 0,
        }
    }

    pub fn iter_mut(&mut self) -> PoolIteratorMut<T> {
        unsafe {
            PoolIteratorMut {
                ptr: self.records.as_mut_ptr(),
                end: self.records.as_mut_ptr().offset(self.records.len() as isize),
                marker: PhantomData,
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

pub struct PoolIteratorMut<'a, T> {
    ptr: *mut PoolRecord<T>,
    end: *mut PoolRecord<T>,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T> Iterator for PoolIteratorMut<'a, T> {
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

#[test]
fn pool_sanity_tests() {
    let mut pool: Pool<String> = Pool::new();
    let foobar_handle = pool.spawn(String::from("Foobar"));
    assert_eq!(foobar_handle.index, 0);
    assert_ne!(foobar_handle.generation, Pool::<String>::INVALID_GENERATION);
    let foobar_handle_copy = foobar_handle.clone();
    assert_eq!(foobar_handle.index, foobar_handle_copy.index);
    assert_eq!(foobar_handle.generation, foobar_handle_copy.generation);
    let baz_handle = pool.spawn(String::from("Baz"));
    assert_eq!(pool.borrow(&foobar_handle).unwrap(), "Foobar");
    assert_eq!(pool.borrow(&baz_handle).unwrap(), "Baz");
    pool.free(foobar_handle);
    assert_eq!(pool.is_valid_handle(&foobar_handle_copy), false);
    assert_eq!(pool.is_valid_handle(&baz_handle), true);
    let at_foobar_index = pool.spawn(String::from("AtFoobarIndex"));
    assert_eq!(at_foobar_index.index, 0);
    assert_ne!(at_foobar_index.generation, Pool::<String>::INVALID_GENERATION);
    assert_eq!(pool.borrow(&at_foobar_index).unwrap(), "AtFoobarIndex");
}

#[test]
fn pool_iterator_mut_test() {
    let mut pool: Pool<String> = Pool::new();
    pool.spawn(format!("Foobar"));
    let d = pool.spawn(format!("Foo"));
    pool.free(d);
    pool.spawn(format!("Baz"));
    for s in pool.iter_mut() {
        println!("{}", s);
    }
}