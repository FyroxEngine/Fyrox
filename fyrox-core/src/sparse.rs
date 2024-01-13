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
    pub const UNASSIGNED_INDEX: usize = usize::MAX;

    pub fn unassigned() -> Self {
        Self {
            index: AtomicUsize::new(Self::UNASSIGNED_INDEX),
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

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn atomic_index_get() {
        let ai = AtomicIndex::new(42);
        assert_eq!(ai.get(), 42);
    }

    #[test]
    fn atomic_index_unassigned() {
        let ai = AtomicIndex::unassigned();
        assert_eq!(ai.get(), usize::MAX);
    }

    #[test]
    fn atomic_index_new() {
        let ai = AtomicIndex::new(42);
        assert_eq!(ai.get(), 42);
    }

    #[test]
    fn atomic_index_set() {
        let ai = AtomicIndex::new(42);
        assert_eq!(ai.get(), 42);

        ai.set(1);
        assert_eq!(ai.get(), 1);
    }

    #[test]
    fn default_for_atomic_index() {
        let mut ai = AtomicIndex::default();
        assert_eq!(ai.index.get_mut(), &usize::MAX);
    }

    #[test]
    fn clone_for_atomic_index() {
        let ai = AtomicIndex::default();
        let ai2 = ai.clone();
        assert_eq!(ai2.get(), usize::MAX);
    }

    #[test]
    fn default_for_sparse_buffer() {
        let sb = SparseBuffer::<f32>::default();

        assert_eq!(sb.vec, Vec::<Option<f32>>::default());
        assert_eq!(sb.free, Vec::<usize>::default());
    }

    #[test]
    fn clone_for_sparse_buffer() {
        let sb = SparseBuffer::<f32>::default();
        let sb2 = sb.clone();

        assert_eq!(sb.vec, sb2.vec);
        assert_eq!(sb.free, sb2.free);
    }

    #[test]
    fn sparse_buffer_with_capacity() {
        let sb = SparseBuffer::<f32>::with_capacity(10);

        assert_eq!(sb.vec, Vec::with_capacity(10));
        assert_eq!(sb.free, vec![]);
    }

    #[test]
    fn sparse_buffer_len() {
        let sb = SparseBuffer::<f32>::default();

        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn sparse_buffer_filled() {
        let sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.filled(), 2);
    }

    #[test]
    fn sparse_buffer_is_empty() {
        let sb = SparseBuffer::<f32>::default();

        assert!(sb.is_empty());

        let sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };
        assert!(!sb.is_empty());
    }

    #[test]
    fn sparse_buffer_is_index_valid() {
        let sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert!(!sb.is_index_valid(&AtomicIndex::new(0)));
        assert!(sb.is_index_valid(&AtomicIndex::new(1)));
    }

    #[test]
    fn sparse_buffer_get_raw() {
        let sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.get_raw(0), None);
        assert_eq!(sb.get_raw(1), Some(&1));
    }

    #[test]
    fn sparse_buffer_get_mut_raw() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.get_mut_raw(0), None);
        assert_eq!(sb.get_mut_raw(1), Some(&mut 1));
    }

    #[test]
    fn sparse_buffer_get() {
        let sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.get(&AtomicIndex::new(0)), None);
        assert_eq!(sb.get(&AtomicIndex::new(1)), Some(&1));
    }

    #[test]
    fn sparse_buffer_get_mut() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.get_mut(&AtomicIndex::new(0)), None);
        assert_eq!(sb.get_mut(&AtomicIndex::new(1)), Some(&mut 1));
    }

    #[test]
    fn sparse_buffer_iter() {
        let sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert!(sb.iter().eq([&1]));
    }

    #[test]
    fn sparse_buffer_iter_mut() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert!(sb.iter_mut().eq([&mut 1]));
    }

    #[test]
    fn sparse_buffer_clear() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![1, 2],
        };

        sb.clear();
        assert!(sb.vec.is_empty());
        assert!(sb.free.is_empty());
    }

    #[test]
    fn sparse_buffer_free_raw() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.free_raw(42), None);
        assert_eq!(sb.free_raw(0), None);
        assert_eq!(sb.free_raw(1), Some(1));
        assert_eq!(sb.vec, vec![None, None]);
        assert_eq!(sb.free, vec![1]);
    }

    #[test]
    fn sparse_buffer_free() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![],
        };

        assert_eq!(sb.free(&AtomicIndex::new(0)), None);
        assert_eq!(sb.free(&AtomicIndex::new(1)), Some(1));
        assert_eq!(sb.vec, vec![None, None]);
        assert_eq!(sb.free, vec![1]);
    }

    #[test]
    fn sparse_buffer_spawn() {
        let mut sb = SparseBuffer {
            vec: vec![None, Some(1)],
            free: vec![0],
        };

        assert_eq!(sb.spawn(42).get(), 0);
        assert_eq!(sb.vec, vec![Some(42), Some(1)]);
        assert_eq!(sb.free, vec![]);

        assert_eq!(sb.spawn(5).get(), 2);
        assert_eq!(sb.vec, vec![Some(42), Some(1), Some(5)]);
        assert_eq!(sb.free, vec![]);
    }
}
