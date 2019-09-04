use std::marker::PhantomData;

pub mod pool;
pub mod visitor;
pub mod color_gradient;

pub struct UnsafeCollectionView<T> {
    items: *const T,
    len: usize,
}

impl<T> UnsafeCollectionView<T> {
    pub fn empty() -> UnsafeCollectionView<T> {
        UnsafeCollectionView {
            items: std::ptr::null(),
            len: 0,
        }
    }

    pub fn from_slice(vec: &[T]) -> UnsafeCollectionView<T> {
        UnsafeCollectionView {
            items: vec.as_ptr(),
            len: vec.len(),
        }
    }

    pub fn iter(&self) -> UnsafeCollectionViewIterator<T> {
        unsafe {
            UnsafeCollectionViewIterator {
                current: self.items,
                end: self.items.add(self.len),
                marker: PhantomData,
            }
        }
    }
}

pub struct UnsafeCollectionViewIterator<'a, T> {
    current: *const T,
    end: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> Iterator for UnsafeCollectionViewIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        unsafe {
            if self.current != self.end {
                let value = self.current;
                self.current = self.current.offset(1);
                Some(&*value)
            } else {
                None
            }
        }
    }
}