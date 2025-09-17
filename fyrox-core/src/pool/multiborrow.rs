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

use super::{Handle, Pool, RefCounter};
use crate::pool::{NodeOrNodeVariant};
use crate::{
    pool::{BorrowErrorKind, HandleInfo},
    ComponentProvider,
};
use std::{
    any::TypeId,
    cell::RefCell,
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
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
pub struct MultiBorrowContext<'a, T>
where
    T: Sized,
{
    pool: &'a mut Pool<T>,
    free_indices: RefCell<Vec<u32>>,
}

#[derive(PartialEq)]
pub struct MultiBorrowError {
    pub kind: MultiBorrowErrorKind,
    pub handle_info: HandleInfo,
}

impl MultiBorrowError {
    pub fn new(kind: MultiBorrowErrorKind, handle_info: HandleInfo) -> Self {
        Self { kind, handle_info }
    }
}

#[derive(PartialEq)]
pub enum MultiBorrowErrorKind {
    SingleBorrowError(BorrowErrorKind),
    MutablyBorrowed,
    ImmutablyBorrowed,
}

impl From<BorrowErrorKind> for MultiBorrowErrorKind {
    fn from(kind: BorrowErrorKind) -> Self {
        Self::SingleBorrowError(kind)
    }
}

impl Debug for MultiBorrowErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for MultiBorrowErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiBorrowErrorKind::SingleBorrowError(kind) => write!(f, "{}", kind),
            MultiBorrowErrorKind::MutablyBorrowed => {
                write!(f, "Element is already mutably borrowed.")
            }
            MultiBorrowErrorKind::ImmutablyBorrowed => {
                write!(f, "Element is already immutably borrowed.")
            }
        }
    }
}

impl Display for MultiBorrowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.kind, self.handle_info)
    }
}

impl Debug for MultiBorrowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<T> Drop for MultiBorrowContext<'_, T>
where
    T: Sized,
{
    fn drop(&mut self) {
        self.pool
            .free_stack
            .extend_from_slice(&self.free_indices.borrow())
    }
}

impl<'a, T: 'static> MultiBorrowContext<'a, T> {
    #[inline]
    pub fn new(pool: &'a mut Pool<T>) -> Self {
        Self {
            pool,
            free_indices: Default::default(),
        }
    }

    #[inline]
    fn try_get_node_internal<'b: 'a, C, F>(
        &'b self,
        handle: Handle<T>,
        func: F,
    ) -> Result<Ref<'a, 'b, C>, MultiBorrowError>
    where
        C: ?Sized,
        F: FnOnce(&T) -> Result<&C, MultiBorrowError>,
    {
        let Some(record) = self.pool.records_get(handle.index) else {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::InvalidHandleIndex.into(),
                handle.into(),
            ));
        };

        if handle.generation != record.generation {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::InvalidHandleGeneration.into(),
                handle.into(),
            ));
        }

        let current_ref_count = unsafe { record.ref_counter.get() };
        if current_ref_count < 0 {
            return Err(MultiBorrowError::new(
                MultiBorrowErrorKind::MutablyBorrowed,
                handle.into(),
            ));
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &*record.payload.0.get() };

        let Some(payload) = payload_container.as_ref() else {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::Empty.into(),
                handle.into(),
            ));
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
    /// fail in three main reasons:
    ///
    /// 1) A reference to an element is already taken - returning multiple mutable references to the
    /// same element is forbidden by Rust safety rules.
    /// 2) You're trying to get more references that the context could handle (there is not enough space
    /// in the internal handles storage) - in this case you must increase `N`.
    /// 3) A given handle is invalid.
    // #[inline]
    // pub fn try_get_node<'b: 'a>(
    //     &'b self,
    //     handle: Handle<T>,
    // ) -> Result<Ref<'a, 'b, T>, MultiBorrowError> {
    //     self.try_get_node_internal(handle, |obj| Ok(obj))
    // }
    // #[deprecated(
    //     note = "to be consistent with single borrow naming convention and avoid polluting the API. Call unwrap on try_get_node instead"
    // )]
    // #[inline]
    // pub fn get_node<'b: 'a>(&'b self, handle: Handle<T>) -> Ref<'a, 'b, T> {
    //     self.try_get_node(handle).unwrap()
    // }

    #[inline]
    fn try_get_node_mut_internal<'b: 'a, C, F>(
        &'b self,
        handle: Handle<T>,
        func: F,
    ) -> Result<RefMut<'a, 'b, C>, MultiBorrowError>
    where
        C: ?Sized,
        F: FnOnce(&mut T) -> Result<&mut C, MultiBorrowError>,
    {
        let Some(record) = self.pool.records_get(handle.index) else {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::InvalidHandleIndex.into(),
                handle.into(),
            ));
        };

        if handle.generation != record.generation {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::InvalidHandleGeneration.into(),
                handle.into(),
            ));
        }

        // SAFETY: It is safe to access the counter because of borrow checker guarantees that
        // the record is alive.
        let current_ref_count = unsafe { record.ref_counter.get() };
        match current_ref_count.cmp(&0) {
            Ordering::Less => {
                return Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::MutablyBorrowed,
                    handle.into(),
                ));
            }
            Ordering::Greater => {
                return Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::ImmutablyBorrowed,
                    handle.into(),
                ));
            }
            _ => (),
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &mut *record.payload.0.get() };

        let Some(payload) = payload_container.as_mut() else {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::Empty.into(),
                handle.into(),
            ));
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

    // #[inline]
    // pub fn try_get_node_mut<'b: 'a>(
    //     &'b self,
    //     handle: Handle<T>,
    // ) -> Result<RefMut<'a, 'b, T>, MultiBorrowError> {
    //     self.try_get_node_mut_internal(handle, |obj| Ok(obj))
    // }

    // #[deprecated(
    //     note = "to be consistent with single borrow naming convention and avoid polluting the API. Call unwrap on try_get_node_mut instead"
    // )]
    // #[inline]
    // pub fn get_node_mut<'b: 'a>(&'b self, handle: Handle<T>) -> RefMut<'a, 'b, T> {
    //     self.try_get_node_mut(handle).unwrap()
    // }
}

impl<'a, T: 'static> MultiBorrowContext<'a, T> {
    #[inline]
    pub fn try_get<'b: 'a, U: NodeOrNodeVariant<T> + 'static>(
        &'b self,
        handle: Handle<U>,
    ) -> Result<Ref<'a, 'b, U>, MultiBorrowError> {
        // let node = self.try_get_node(handle.cast())?;
        self.try_get_node_internal(handle.cast(), |node| {
            U::convert_to_dest_type(node).map_err(|e| {
                MultiBorrowError::new(BorrowErrorKind::MismatchedType(e).into(), handle.into())
            })
        })
    }

    #[inline]
    pub fn try_get_mut<'b: 'a, U: NodeOrNodeVariant<T> + 'static>(
        &'b self,
        handle: Handle<U>,
    ) -> Result<RefMut<'a, 'b, U>, MultiBorrowError> {
        self.try_get_node_mut_internal(handle.cast(), |node| {
            U::convert_to_dest_type_mut(node).map_err(|e| {
                MultiBorrowError::new(BorrowErrorKind::MismatchedType(e).into(), handle.into())
            })
        })
    }

    #[inline]
    pub fn free(&self, handle: Handle<T>) -> Result<T, MultiBorrowError> {
        let Some(record) = self.pool.records_get(handle.index) else {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::InvalidHandleIndex.into(),
                handle.into(),
            ));
        };

        if handle.generation != record.generation {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::InvalidHandleGeneration.into(),
                handle.into(),
            ));
        }

        // The record must be non-borrowed to be freed.
        // SAFETY: It is safe to access the counter because of borrow checker guarantees that
        // the record is alive.
        let current_ref_count = unsafe { record.ref_counter.get() };
        match current_ref_count.cmp(&0) {
            Ordering::Less => {
                return Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::MutablyBorrowed,
                    handle.into(),
                ));
            }
            Ordering::Greater => {
                return Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::ImmutablyBorrowed,
                    handle.into(),
                ));
            }
            _ => (),
        }

        // SAFETY: We've enforced borrowing rules by the previous check.
        let payload_container = unsafe { &mut *record.payload.0.get() };

        let Some(payload) = payload_container.take() else {
            return Err(MultiBorrowError::new(
                BorrowErrorKind::Empty.into(),
                handle.into(),
            ));
        };

        self.free_indices.borrow_mut().push(handle.index);

        Ok(payload)
    }
}

impl<'a, T> MultiBorrowContext<'a, T>
where
    T: ComponentProvider + 'static,
{
    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component<'b: 'a, C>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<Ref<'a, 'b, C>, MultiBorrowError>
    where
        C: 'static,
    {
        self.try_get_node_internal(handle, move |node| {
            // node.query_component_ref(TypeId::of::<C>())
            //     .and_then(|c| c.downcast_ref())
            //     .ok_or(MultiBorrowErrorKind::NoSuchComponent(handle))
            let component_any = node.query_component_ref(TypeId::of::<C>()).map_err(|e| {
                MultiBorrowError::new(
                    BorrowErrorKind::NoSuchComponent(e.into()).into(),
                    handle.into(),
                )
            })?;
            Ok(component_any
                .downcast_ref()
                .expect("TypeId matched but downcast failed"))
        })
    }

    /// Tries to mutably borrow an object and fetch its component of specified type.
    #[inline]
    pub fn try_get_component_mut<'b: 'a, C>(
        &'b self,
        handle: Handle<T>,
    ) -> Result<RefMut<'a, 'b, C>, MultiBorrowError>
    where
        C: 'static,
    {
        self.try_get_node_mut_internal(handle, move |node| {
            let component_any = node.query_component_mut(TypeId::of::<C>()).map_err(|e| {
                MultiBorrowError::new(
                    BorrowErrorKind::NoSuchComponent(e.into()).into(),
                    handle.into(),
                )
            })?;
            Ok(component_any
                .downcast_mut()
                .expect("TypeId matched but downcast failed"))
        })
    }
}

#[cfg(test)]
mod test {
    use super::MultiBorrowErrorKind;
    use crate::pool::{BorrowErrorKind, MismatchedTypeError, MultiBorrowError, NodeOrNodeVariant, Pool};

    #[derive(PartialEq, Clone, Copy, Debug)]
    struct MyPayload(u32);

    impl NodeOrNodeVariant<MyPayload> for MyPayload {
        fn convert_to_dest_type(payload: &MyPayload) -> Result<&MyPayload, MismatchedTypeError> {
            Ok(payload)
        }

        fn convert_to_dest_type_mut(
            payload: &mut MyPayload,
        ) -> Result<&mut MyPayload, MismatchedTypeError> {
            Ok(payload)
        }
    }


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
                Err(MultiBorrowError::new(
                    BorrowErrorKind::Empty.into(),
                    d.into()
                ))
                .as_ref()
            );
            assert_eq!(
                ctx.try_get_mut(d).as_deref_mut(),
                Err(MultiBorrowError::new(
                    BorrowErrorKind::Empty.into(),
                    d.into()
                ))
                .as_mut()
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
                Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::ImmutablyBorrowed,
                    a.into()
                ))
                .as_ref()
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
                Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::MutablyBorrowed,
                    b.into()
                ))
                .as_mut()
            );

            let mut ref_c_1 = ctx.try_get_mut(c);
            let mut ref_c_2 = ctx.try_get_mut(c);
            assert_eq!(ref_c_1.as_deref_mut(), Ok(&mut val_c));
            assert_eq!(
                ref_c_2.as_deref_mut(),
                Err(MultiBorrowError::new(
                    MultiBorrowErrorKind::MutablyBorrowed,
                    c.into()
                ))
                .as_mut()
            );
        }
    }
}
