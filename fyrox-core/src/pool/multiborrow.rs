use super::{Handle, PayloadContainer, Pool, RefCounter};
use crate::ComponentProvider;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::{
    any::TypeId,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic,
};

pub struct Ref<'a, 'b, T>
where
    T: ?Sized,
{
    data: &'a T,
    ref_counter: &'a RefCounter,
    phantom: PhantomData<&'b ()>,
}

impl<'a, 'b, T> Debug for Ref<'a, 'b, T>
where
    T: ?Sized + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl<'a, 'b, T> Deref for Ref<'a, 'b, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, 'b, T> Drop for Ref<'a, 'b, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.ref_counter.decrement();
    }
}

pub struct RefMut<'a, 'b, T>
where
    T: ?Sized,
{
    data: &'a mut T,
    ref_counter: &'a RefCounter,
    phantom: PhantomData<&'b ()>,
}

impl<'a, 'b, T> Debug for RefMut<'a, 'b, T>
where
    T: ?Sized + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl<'a, 'b, T> Deref for RefMut<'a, 'b, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, 'b, T> DerefMut for RefMut<'a, 'b, T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<'a, 'b, T> Drop for RefMut<'a, 'b, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.ref_counter.increment();
    }
}

/// Multi-borrow context allows you to get as many **unique** references to elements in
/// a pool as you want.
pub struct MultiBorrowContext<'a, T, P = Option<T>>
where
    T: Sized,
    P: PayloadContainer<Element = T> + 'static,
{
    pool: &'a mut Pool<T, P>,
    free_indices: RefCell<Vec<u32>>,
}

#[derive(PartialEq)]
pub enum MultiBorrowError<T> {
    Empty(Handle<T>),
    NoSuchComponent(Handle<T>),
    MutablyBorrowed(Handle<T>),
    ImmutablyBorrowed(Handle<T>),
    InvalidHandleIndex(Handle<T>),
    InvalidHandleGeneration(Handle<T>),
}

impl<T> Debug for MultiBorrowError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<T> Display for MultiBorrowError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty(handle) => {
                write!(f, "There's no object at {handle} handle.")
            }
            Self::NoSuchComponent(handle) => write!(
                f,
                "An object at {handle} handle does not have such component.",
            ),
            Self::MutablyBorrowed(handle) => {
                write!(
                    f,
                    "An object at {handle} handle cannot be borrowed immutably, because it is \
                    already borrowed mutably."
                )
            }
            Self::ImmutablyBorrowed(handle) => {
                write!(
                    f,
                    "An object at {handle} handle cannot be borrowed mutably, because it is \
                    already borrowed immutably."
                )
            }
            Self::InvalidHandleIndex(handle) => {
                write!(
                    f,
                    "The index {} in {handle} handle is out of bounds.",
                    handle.index
                )
            }
            Self::InvalidHandleGeneration(handle) => {
                write!(
                    f,
                    "The generation {} in {handle} handle does not match the record's generation. \
                    It means that the object at the handle was freed and it position was taken \
                    by some other object.",
                    handle.generation
                )
            }
        }
    }
}

impl<'a, T, P> Drop for MultiBorrowContext<'a, T, P>
where
    T: Sized,
    P: PayloadContainer<Element = T> + 'static,
{
    fn drop(&mut self) {
        self.pool
            .free_stack
            .extend_from_slice(&self.free_indices.borrow())
    }
}

impl<'a, T, P> MultiBorrowContext<'a, T, P>
where
    T: Sized,
    P: PayloadContainer<Element = T> + 'static,
{
    #[inline]
    pub fn new(pool: &'a mut Pool<T, P>) -> Self {
        Self {
            pool,
            free_indices: Default::default(),
        }
    }

    #[inline]
    fn try_get_internal<'b: 'a, C, F>(
        &'b self,
        handle: Handle<T>,
        func: F,
    ) -> Result<Ref<'a, 'b, C>, MultiBorrowError<T>>
    where
        C: ?Sized,
        F: FnOnce(&T) -> Result<&C, MultiBorrowError<T>>,
    {
        let Some(record) = self.pool.records_get(handle.index) else {
            return Err(MultiBorrowError::InvalidHandleIndex(handle));
        };

        if handle.generation != record.generation {
            return Err(MultiBorrowError::InvalidHandleGeneration(handle));
        }

        let current_ref_count = record.ref_counter.0.load(atomic::Ordering::Relaxed);
        if current_ref_count < 0 {
            return Err(MultiBorrowError::MutablyBorrowed(handle));
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &*record.payload.0.get() };

        let Some(payload) = payload_container.as_ref() else {
            return Err(MultiBorrowError::Empty(handle));
        };

        if let Err(ref_count) = record.ref_counter.0.compare_exchange(
            current_ref_count,
            current_ref_count + 1,
            atomic::Ordering::Acquire,
            atomic::Ordering::Relaxed,
        ) {
            // This might happen if other thread have already acquired the mutable reference.
            if ref_count < 0 {
                return Err(MultiBorrowError::MutablyBorrowed(handle));
            }
        }

        Ok(Ref {
            data: func(payload)?,
            ref_counter: &record.ref_counter,
            phantom: PhantomData,
        })
    }

    /// Tries to get a mutable reference to a pool element located at the given handle. The method could
    /// fail in three main reasons:
    ///
    /// 1) A reference to an element is already taken - returning multiple mutable references to the
    /// same element is forbidden by Rust safety rules.
    /// 2) You're trying to get more references that the context could handle (there is not enough space
    /// in the internal handles storage) - in this case you must increase `N`.
    /// 3) A given handle is invalid.
    #[inline]
    pub fn try_get<'b: 'a>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<Ref<'a, 'b, T>, MultiBorrowError<T>> {
        self.try_get_internal(handle, |obj| Ok(obj))
    }

    #[inline]
    pub fn get<'b: 'a>(&'b self, handle: Handle<T>) -> Ref<'a, 'b, T> {
        self.try_get(handle).unwrap()
    }

    #[inline]
    fn try_get_mut_internal<'b: 'a, C, F>(
        &'b self,
        handle: Handle<T>,
        func: F,
    ) -> Result<RefMut<'a, 'b, C>, MultiBorrowError<T>>
    where
        C: ?Sized,
        F: FnOnce(&mut T) -> Result<&mut C, MultiBorrowError<T>>,
    {
        let Some(record) = self.pool.records_get(handle.index) else {
            return Err(MultiBorrowError::InvalidHandleIndex(handle));
        };

        if handle.generation != record.generation {
            return Err(MultiBorrowError::InvalidHandleGeneration(handle));
        }

        let current_ref_count = record.ref_counter.0.load(atomic::Ordering::Relaxed);
        match current_ref_count.cmp(&0) {
            Ordering::Less => {
                return Err(MultiBorrowError::MutablyBorrowed(handle));
            }
            Ordering::Greater => {
                return Err(MultiBorrowError::ImmutablyBorrowed(handle));
            }
            _ => (),
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &mut *record.payload.0.get() };

        let Some(payload) = payload_container.as_mut() else {
            return Err(MultiBorrowError::Empty(handle));
        };

        if let Err(ref_count) = record.ref_counter.0.compare_exchange(
            0,
            -1,
            atomic::Ordering::Acquire,
            atomic::Ordering::Relaxed,
        ) {
            match ref_count.cmp(&0) {
                Ordering::Less => {
                    return Err(MultiBorrowError::MutablyBorrowed(handle));
                }
                Ordering::Greater => {
                    return Err(MultiBorrowError::ImmutablyBorrowed(handle));
                }
                _ => (),
            }
        }

        Ok(RefMut {
            data: func(payload)?,
            ref_counter: &record.ref_counter,
            phantom: PhantomData,
        })
    }

    #[inline]
    pub fn try_get_mut<'b: 'a>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<RefMut<'a, 'b, T>, MultiBorrowError<T>> {
        self.try_get_mut_internal(handle, |obj| Ok(obj))
    }

    #[inline]
    pub fn get_mut<'b: 'a>(&'b self, handle: Handle<T>) -> RefMut<'a, 'b, T> {
        self.try_get_mut(handle).unwrap()
    }

    #[inline]
    pub fn free(&self, handle: Handle<T>) -> Result<T, MultiBorrowError<T>> {
        let Some(record) = self.pool.records_get(handle.index) else {
            return Err(MultiBorrowError::InvalidHandleIndex(handle));
        };

        if handle.generation != record.generation {
            return Err(MultiBorrowError::InvalidHandleGeneration(handle));
        }

        // Acquire temporary lock.
        if let Err(ref_count) = record.ref_counter.0.compare_exchange(
            0,
            -1,
            atomic::Ordering::Acquire,
            atomic::Ordering::Relaxed,
        ) {
            match ref_count.cmp(&0) {
                Ordering::Less => {
                    return Err(MultiBorrowError::MutablyBorrowed(handle));
                }
                Ordering::Greater => {
                    return Err(MultiBorrowError::ImmutablyBorrowed(handle));
                }
                _ => (),
            }
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &mut *record.payload.0.get() };

        let Some(payload) = payload_container.take() else {
            return Err(MultiBorrowError::Empty(handle));
        };

        self.free_indices.borrow_mut().push(handle.index);

        record.ref_counter.increment();

        Ok(payload)
    }
}

impl<'a, T, P> MultiBorrowContext<'a, T, P>
where
    T: Sized + ComponentProvider,
    P: PayloadContainer<Element = T> + 'static,
{
    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component_of_type<'b: 'a, C>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<Ref<'a, 'b, C>, MultiBorrowError<T>>
    where
        C: 'static,
    {
        self.try_get_internal(handle, move |obj| {
            obj.query_component_ref(TypeId::of::<C>())
                .and_then(|c| c.downcast_ref())
                .ok_or(MultiBorrowError::NoSuchComponent(handle))
        })
    }

    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component_of_type_mut<'b: 'a, C>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<RefMut<'a, 'b, C>, MultiBorrowError<T>>
    where
        C: 'static,
    {
        self.try_get_mut_internal(handle, move |obj| {
            obj.query_component_mut(TypeId::of::<C>())
                .and_then(|c| c.downcast_mut())
                .ok_or(MultiBorrowError::NoSuchComponent(handle))
        })
    }
}

#[cfg(test)]
mod test {
    use super::MultiBorrowError;
    use crate::pool::Pool;
    use std::sync::atomic;

    #[derive(PartialEq, Clone, Copy, Debug)]
    struct MyPayload(u32);

    #[test]
    fn test_multi_borrow_context() {
        let mut pool = Pool::<MyPayload>::new();

        let mut val_a = MyPayload(123);
        let mut val_b = MyPayload(321);
        let mut val_c = MyPayload(42);
        let val_d = MyPayload(666);

        let a = pool.spawn(val_a);
        let b = pool.spawn(val_b);
        let c = pool.spawn(val_c);
        let d = pool.spawn(val_d);

        pool.free(d);

        let ctx = pool.begin_multi_borrow();

        // Test empty.
        {
            assert_eq!(
                ctx.try_get(d).as_deref(),
                Err(MultiBorrowError::Empty(d)).as_ref()
            );
            assert_eq!(
                ctx.try_get_mut(d).as_deref_mut(),
                Err(MultiBorrowError::Empty(d)).as_mut()
            );
        }

        // Test immutable borrowing of the same element.
        {
            let ref_a_1 = ctx.try_get(a);
            let ref_a_2 = ctx.try_get(a);
            assert_eq!(ref_a_1.as_deref(), Ok(&val_a));
            assert_eq!(ref_a_2.as_deref(), Ok(&val_a));
        }

        // Test immutable borrowing of the same element with the following mutable borrowing.
        {
            let ref_a_1 = ctx.try_get(a);
            assert_eq!(
                ref_a_1
                    .as_ref()
                    .unwrap()
                    .ref_counter
                    .0
                    .load(atomic::Ordering::Relaxed),
                1
            );
            let ref_a_2 = ctx.try_get(a);
            assert_eq!(
                ref_a_2
                    .as_ref()
                    .unwrap()
                    .ref_counter
                    .0
                    .load(atomic::Ordering::Relaxed),
                2
            );

            assert_eq!(ref_a_1.as_deref(), Ok(&val_a));
            assert_eq!(ref_a_2.as_deref(), Ok(&val_a));
            assert_eq!(
                ctx.try_get_mut(a).as_deref(),
                Err(MultiBorrowError::ImmutablyBorrowed(a)).as_ref()
            );

            drop(ref_a_1);
            drop(ref_a_2);

            let mut mut_ref_a_1 = ctx.try_get_mut(a);
            assert_eq!(mut_ref_a_1.as_deref_mut(), Ok(&mut val_a));

            assert_eq!(
                mut_ref_a_1
                    .as_ref()
                    .unwrap()
                    .ref_counter
                    .0
                    .load(atomic::Ordering::Relaxed),
                -1
            );
        }

        // Test immutable and mutable borrowing.
        {
            // Borrow two immutable refs to the same element.
            let ref_a_1 = ctx.try_get(a);
            let ref_a_2 = ctx.try_get(a);
            assert_eq!(ref_a_1.as_deref(), Ok(&val_a));
            assert_eq!(ref_a_2.as_deref(), Ok(&val_a));

            // Borrow immutable ref to other element.
            let mut ref_b_1 = ctx.try_get_mut(b);
            let mut ref_b_2 = ctx.try_get_mut(b);
            assert_eq!(ref_b_1.as_deref_mut(), Ok(&mut val_b));
            assert_eq!(
                ref_b_2.as_deref_mut(),
                Err(MultiBorrowError::MutablyBorrowed(b)).as_mut()
            );

            let mut ref_c_1 = ctx.try_get_mut(c);
            let mut ref_c_2 = ctx.try_get_mut(c);
            assert_eq!(ref_c_1.as_deref_mut(), Ok(&mut val_c));
            assert_eq!(
                ref_c_2.as_deref_mut(),
                Err(MultiBorrowError::MutablyBorrowed(c)).as_mut()
            );
        }
    }
}
