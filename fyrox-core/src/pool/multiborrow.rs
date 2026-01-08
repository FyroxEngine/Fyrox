// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use super::{Handle, ObjectOrVariant, PayloadContainer, Pool, PoolError, RefCounter};
use crate::ComponentProvider;
use std::{
    any::TypeId,
    cell::RefCell,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub struct Ref<'a, 'b, T>
where
    T: ?Sized,
{
    data: &'a T,
    ref_counter: &'a RefCounter,
    phantom: PhantomData<&'b ()>,
}

impl<T> Debug for Ref<'_, '_, T>
where
    T: ?Sized + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl<T> Deref for Ref<'_, '_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> Drop for Ref<'_, '_, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        // SAFETY: This is safe, because this ref lifetime is managed by the borrow checker,
        // so it cannot outlive the pool record.
        unsafe {
            self.ref_counter.decrement();
        }
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

impl<T> Debug for RefMut<'_, '_, T>
where
    T: ?Sized + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.data, f)
    }
}

impl<T> Deref for RefMut<'_, '_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T> DerefMut for RefMut<'_, '_, T>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}

impl<T> Drop for RefMut<'_, '_, T>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        // SAFETY: This is safe, because this ref lifetime is managed by the borrow checker,
        // so it cannot outlive the pool record.
        unsafe {
            self.ref_counter.increment();
        }
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

impl<T, P> Drop for MultiBorrowContext<'_, T, P>
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
    ) -> Result<Ref<'a, 'b, C>, PoolError>
    where
        C: ?Sized,
        F: FnOnce(&T) -> Result<&C, PoolError>,
    {
        let record = self.pool.records_get(handle.index)?;

        if handle.generation != record.generation {
            return Err(PoolError::InvalidGeneration(handle.generation));
        }

        let current_ref_count = unsafe { record.ref_counter.get() };
        if current_ref_count < 0 {
            return Err(PoolError::MutablyBorrowed(handle.into()));
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &*record.payload.0.get() };

        let Some(payload) = payload_container.as_ref() else {
            return Err(PoolError::Empty(handle.into()));
        };

        unsafe {
            record.ref_counter.increment();
        }

        Ok(Ref {
            data: func(payload)?,
            ref_counter: &record.ref_counter,
            phantom: PhantomData,
        })
    }

    /// Tries to get a mutable reference to a pool element located at the given handle. The method could
    /// fail in the two main reasons:
    ///
    /// 1) A reference to an element is already taken - returning multiple mutable references to the
    /// same element is forbidden by Rust safety rules.
    /// 2) A given handle is invalid.
    #[inline]
    pub fn try_get<'b, U>(&'b self, handle: Handle<U>) -> Result<Ref<'a, 'b, U>, PoolError>
    where
        'b: 'a,
        U: ObjectOrVariant<T>,
    {
        self.try_get_internal(handle.to_base(), |obj| {
            U::convert_to_dest_type(obj).ok_or(PoolError::InvalidType(handle.into()))
        })
    }

    #[inline]
    pub fn get<'b, U>(&'b self, handle: Handle<U>) -> Ref<'a, 'b, U>
    where
        'b: 'a,
        U: ObjectOrVariant<T>,
    {
        self.try_get(handle).unwrap()
    }

    #[inline]
    fn try_get_mut_internal<'b: 'a, C, F>(
        &'b self,
        handle: Handle<T>,
        func: F,
    ) -> Result<RefMut<'a, 'b, C>, PoolError>
    where
        C: ?Sized,
        F: FnOnce(&mut T) -> Result<&mut C, PoolError>,
    {
        let record = self.pool.records_get(handle.index)?;

        if handle.generation != record.generation {
            return Err(PoolError::InvalidGeneration(handle.generation));
        }

        // SAFETY: It is safe to access the counter because of borrow checker guarantees that
        // the record is alive.
        let current_ref_count = unsafe { record.ref_counter.get() };
        match current_ref_count.cmp(&0) {
            Ordering::Less => {
                return Err(PoolError::MutablyBorrowed(handle.into()));
            }
            Ordering::Greater => {
                return Err(PoolError::ImmutablyBorrowed(handle.into()));
            }
            _ => (),
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &mut *record.payload.0.get() };

        let Some(payload) = payload_container.as_mut() else {
            return Err(PoolError::Empty(handle.into()));
        };

        // SAFETY: It is safe to access the counter because of borrow checker guarantees that
        // the record is alive.
        unsafe {
            record.ref_counter.decrement();
        }

        Ok(RefMut {
            data: func(payload)?,
            ref_counter: &record.ref_counter,
            phantom: PhantomData,
        })
    }

    #[inline]
    pub fn try_get_mut<'b, U>(&'b self, handle: Handle<U>) -> Result<RefMut<'a, 'b, U>, PoolError>
    where
        'b: 'a,
        U: ObjectOrVariant<T>,
    {
        self.try_get_mut_internal(handle.to_base(), |obj| {
            U::convert_to_dest_type_mut(obj).ok_or(PoolError::InvalidType(handle.into()))
        })
    }

    #[inline]
    pub fn get_mut<'b, U>(&'b self, handle: Handle<U>) -> RefMut<'a, 'b, U>
    where
        'b: 'a,
        U: ObjectOrVariant<T>,
    {
        self.try_get_mut(handle).unwrap()
    }

    #[inline]
    pub fn free(&self, handle: Handle<T>) -> Result<T, PoolError> {
        let record = self.pool.records_get(handle.index)?;

        if handle.generation != record.generation {
            return Err(PoolError::InvalidGeneration(handle.generation));
        }

        // The record must be non-borrowed to be freed.
        // SAFETY: It is safe to access the counter because of borrow checker guarantees that
        // the record is alive.
        let current_ref_count = unsafe { record.ref_counter.get() };
        match current_ref_count.cmp(&0) {
            Ordering::Less => {
                return Err(PoolError::MutablyBorrowed(handle.into()));
            }
            Ordering::Greater => {
                return Err(PoolError::ImmutablyBorrowed(handle.into()));
            }
            _ => (),
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &mut *record.payload.0.get() };

        let Some(payload) = payload_container.take() else {
            return Err(PoolError::Empty(handle.into()));
        };

        self.free_indices.borrow_mut().push(handle.index);

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
    ) -> Result<Ref<'a, 'b, C>, PoolError>
    where
        C: 'static,
    {
        self.try_get_internal(handle, move |obj| {
            obj.query_component_ref(TypeId::of::<C>())
                .and_then(|c| c.downcast_ref())
                .ok_or(PoolError::NoSuchComponent(handle.into()))
        })
    }

    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component_of_type_mut<'b: 'a, C>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<RefMut<'a, 'b, C>, PoolError>
    where
        C: 'static,
    {
        self.try_get_mut_internal(handle, move |obj| {
            obj.query_component_mut(TypeId::of::<C>())
                .and_then(|c| c.downcast_mut())
                .ok_or(PoolError::NoSuchComponent(handle.into()))
        })
    }
}

#[cfg(test)]
mod test {
    use super::PoolError;
    use crate::pool::Pool;

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
                Err(PoolError::Empty(d.into())).as_ref()
            );
            assert_eq!(
                ctx.try_get_mut(d).as_deref_mut(),
                Err(PoolError::Empty(d.into())).as_mut()
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
            assert_eq!(unsafe { ref_a_1.as_ref().unwrap().ref_counter.get() }, 1);
            let ref_a_2 = ctx.try_get(a);
            assert_eq!(unsafe { ref_a_2.as_ref().unwrap().ref_counter.get() }, 2);

            assert_eq!(ref_a_1.as_deref(), Ok(&val_a));
            assert_eq!(ref_a_2.as_deref(), Ok(&val_a));
            assert_eq!(
                ctx.try_get_mut(a).as_deref(),
                Err(PoolError::ImmutablyBorrowed(a.into())).as_ref()
            );

            drop(ref_a_1);
            drop(ref_a_2);

            let mut mut_ref_a_1 = ctx.try_get_mut(a);
            assert_eq!(mut_ref_a_1.as_deref_mut(), Ok(&mut val_a));

            assert_eq!(
                unsafe { mut_ref_a_1.as_ref().unwrap().ref_counter.get() },
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
                Err(PoolError::MutablyBorrowed(b.into())).as_mut()
            );

            let mut ref_c_1 = ctx.try_get_mut(c);
            let mut ref_c_2 = ctx.try_get_mut(c);
            assert_eq!(ref_c_1.as_deref_mut(), Ok(&mut val_c));
            assert_eq!(
                ref_c_2.as_deref_mut(),
                Err(PoolError::MutablyBorrowed(c.into())).as_mut()
            );
        }
    }
}
