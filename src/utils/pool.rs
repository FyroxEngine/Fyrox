use std::marker::PhantomData;

struct PoolRecord<T: Sized> {
    stamp: u32,
    generation: u32,
    payload: Option<T>,
}

pub struct Pool<T: Sized> {
    records: Vec<PoolRecord<T>>,
    free_stack: Vec<u32>
}

pub struct Handle<T> {
    pub(crate) index: u32,
    stamp: u32,
    type_marker: PhantomData<T>,
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

impl<T> Handle<T> {
    pub fn none() -> Self {
        Handle {
            index: 0,
            stamp: 0,
            type_marker: PhantomData
        }
    }

    pub fn is_none(&self) -> bool {
        self.index == 0 && self.stamp == 0
    }
}

impl<T> Pool<T> {
    pub fn new() -> Self {
        Pool {
            records: Vec::<PoolRecord<T>>::new(),
            free_stack: Vec::new(),
        }
    }

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

    pub fn borrow(&self, handle: &Handle<T>) -> Option<&T> {
        let index = handle.index as usize;
        if index < self.records.len() {
            unsafe {
                // _unchecked here because we already checked index
                let record = self.records.get_unchecked(index);
                if record.stamp == handle.stamp {
                    if let Some(payload) = &record.payload {
                        return Some(payload);
                    }
                }
            }
        }
        None
    }

    pub fn borrow_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        let index = handle.index as usize;
        if index < self.records.len() {
            unsafe {
                // _unchecked here because we already checked index
                let record = self.records.get_unchecked_mut(index);
                if record.stamp == handle.stamp {
                    if let Some(payload) = &mut record.payload {
                        return Some(payload);
                    }
                }
            }
        }
        None
    }

    pub fn free(&mut self, handle: Handle<T>) {
        let index = handle.index as usize;
        if index < self.records.len() {
            self.free_stack.push(handle.index);
            // move out payload and drop it
            self.records[index].payload.take();
        }
    }

    pub fn capacity(&self) -> usize {
        self.records.len()
    }

    pub fn at_mut(&mut self, n: usize) -> Option<&mut T> {
        if n < self.records.len() {
            if let Some(payload) = &mut self.records[n].payload {
                return Some(payload);
            }
        }
        None
    }

    pub fn at(&self, n: usize) -> Option<&T> {
        if n < self.records.len() {
            if let Some(payload) = &self.records[n].payload {
                return Some(payload);
            }
        }
        None
    }
}
