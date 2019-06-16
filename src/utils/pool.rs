use std::marker::PhantomData;
use std::hash::{Hash, Hasher};

///
/// Pool allows to create as many objects as you want in contiguous memory
/// block. It allows to create and delete objects very fast.
///
pub struct Pool<T: Sized> {
    records: Vec<PoolRecord<T>>,
    free_stack: Vec<u32>
}

///
/// Handle is some sort of non-owning reference to content in a pool.
/// It stores index of object and additional information that
/// allows to ensure that handle is still valid.
///
pub struct Handle<T> {
    index: u32,
    stamp: u32,
    type_marker: PhantomData<T>,
}

struct PoolRecord<T: Sized> {
    stamp: u32,
    generation: u32,
    payload: Option<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Handle<T> {
        Handle {
            index: self.index,
            stamp: self.stamp,
            type_marker: PhantomData
        }
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Handle<T>) -> bool {
        self.stamp == other.stamp && self.index == other.index
    }
}

impl<T> Eq for Handle<T> {

}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.stamp.hash(state);
    }
}

impl<T> Handle<T> {
    #[inline]
    pub fn none() -> Self {
        Handle {
            index: 0,
            stamp: 0,
            type_marker: PhantomData
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.index == 0 && self.stamp == 0
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
    pub fn spawn(&mut self, payload: T) -> Handle<T> {
        if let Some(free_index) = self.free_stack.pop() {
            let record =  &mut self.records[free_index as usize];
            record.generation += 1;
            record.payload.replace(payload);
            return Handle {
                index: free_index,
                stamp: record.generation,
                type_marker: PhantomData
            };
        }

        // No free records, create new one
        let record = PoolRecord {
            stamp: 1,
            generation: 1,
            payload: Some(payload)
        };

        let handle = Handle {
            index: self.records.len() as u32,
            stamp: record.generation,
            type_marker: PhantomData
        };

        self.records.push(record);

        handle
    }

    #[inline]
    pub fn borrow(&self, handle: &Handle<T>) -> Option<&T> {
        if let Some(record) = self.records.get(handle.index as usize) {
            if record.stamp == handle.stamp {
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
            if record.stamp == handle.stamp {
                if let Some(payload) = &mut record.payload {
                    return Some(payload);
                }
            }
        }
        None
    }

    #[inline]
    pub fn free(&mut self, handle: Handle<T>) {
        let index = handle.index as usize;
        if index < self.records.len() {
            self.free_stack.push(handle.index);
            // move out payload and drop it
            self.records[index].payload.take();
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
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
}
