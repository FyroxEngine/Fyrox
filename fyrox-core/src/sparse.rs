use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub struct AtomicIndex {
    index: AtomicUsize,
}

impl Clone for AtomicIndex {
    fn clone(&self) -> Self {
        Self {
            index: AtomicUsize::new(self.index.load(Ordering::SeqCst)),
        }
    }
}

impl Default for AtomicIndex {
    fn default() -> Self {
        Self::unassigned()
    }
}

impl AtomicIndex {
    pub fn unassigned() -> Self {
        Self {
            index: AtomicUsize::new(usize::MAX),
        }
    }

    fn new(index: usize) -> Self {
        Self {
            index: AtomicUsize::new(index),
        }
    }

    pub fn set(&self, index: usize) {
        self.index.store(index, Ordering::SeqCst)
    }

    pub fn get(&self) -> usize {
        self.index.load(Ordering::SeqCst)
    }
}

pub struct SparseBuffer<T> {
    vec: Vec<Option<T>>,
    free: Vec<usize>,
}

impl<T> Default for SparseBuffer<T> {
    fn default() -> Self {
        Self {
            vec: Default::default(),
            free: Default::default(),
        }
    }
}

impl<T: Clone> Clone for SparseBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
            free: self.free.clone(),
        }
    }
}

impl<T> SparseBuffer<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            vec: Vec::with_capacity(capacity),
            free: vec![],
        }
    }

    pub fn spawn(&mut self, payload: T) -> AtomicIndex {
        match self.free.pop() {
            Some(free) => {
                let old = self.vec[free].replace(payload);
                debug_assert!(old.is_none());
                AtomicIndex::new(free)
            }
            None => {
                let index = AtomicIndex::new(self.vec.len());
                self.vec.push(Some(payload));
                index
            }
        }
    }

    pub fn free(&mut self, index: &AtomicIndex) -> Option<T> {
        self.free_raw(index.get())
    }

    pub fn free_raw(&mut self, index: usize) -> Option<T> {
        match self.vec.get_mut(index) {
            Some(entry) => match entry.take() {
                Some(payload) => {
                    self.free.push(index);
                    Some(payload)
                }
                None => None,
            },
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.filled() == 0
    }

    pub fn filled(&self) -> usize {
        self.vec.len() - self.free.len()
    }

    pub fn is_index_valid(&self, index: &AtomicIndex) -> bool {
        self.get(index).is_some()
    }

    pub fn get(&self, index: &AtomicIndex) -> Option<&T> {
        self.get_raw(index.get())
    }

    pub fn get_mut(&mut self, index: &AtomicIndex) -> Option<&mut T> {
        self.get_mut_raw(index.get())
    }

    pub fn get_raw(&self, index: usize) -> Option<&T> {
        self.vec.get(index).and_then(|entry| entry.as_ref())
    }

    pub fn get_mut_raw(&mut self, index: usize) -> Option<&mut T> {
        self.vec.get_mut(index).and_then(|entry| entry.as_mut())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter().filter_map(|entry| entry.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.vec.iter_mut().filter_map(|entry| entry.as_mut())
    }

    pub fn clear(&mut self) {
        self.vec.clear();
        self.free.clear();
    }
}
