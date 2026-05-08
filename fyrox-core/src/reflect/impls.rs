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

//! `Reflect` implementations

use crate::{
    delegate_reflect,
    reflect::{blank_reflect, prelude::*},
    sstorage::ImmutableString,
    uuid::Uuid,
    warn, SafeLock,
};
use fyrox_core_derive::impl_reflect;
use nalgebra::*;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt::Debug,
    ops::{Deref, DerefMut, Range},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

impl_reflect! {
    pub struct Matrix<T: Copy + 'static, R: Dim + 'static, C: Dim + 'static, S: Copy + Debug + FieldValue + 'static> {
        pub data: S,
        // _phantoms: PhantomData<(T, R, C)>,
    }
}

impl_reflect! {
    pub struct ArrayStorage<T: Copy + Debug + Reflect, const R: usize, const C: usize>(pub [[T; R]; C]);
}

impl_reflect! {
    pub struct Unit<T: Copy + Debug + 'static> {
        // pub(crate) value: T,
    }
}

impl_reflect! {
    pub struct Quaternion<T: Copy + Debug + Reflect> {
        pub coords: Vector4<T>,
    }
}

macro_rules! impl_blank_reflect {
    ( $( $ty:ty ),* $(,)? ) => {
        $(
            impl Reflect for $ty {
                blank_reflect!();
            }
        )*
    }
}

impl_blank_reflect! {
    f32, f64,
    usize, u8, u16, u32, u64,
    isize, i8, i16, i32, i64,
    bool, char,
    String,
    std::path::PathBuf,
    Duration, Instant,
    ImmutableString
}

macro_rules! impl_reflect_tuple {
    (
        $(
            ( $($t:ident,)* );
        )*
    ) => {
        $(
            impl< $($t: Clone + Reflect),* > Reflect for ( $($t,)* ) {
                blank_reflect!();
            }
        )*
    }
}

impl_reflect_tuple! {
    (T0,);
    (T0, T1, );
    (T0, T1, T2, );
    (T0, T1, T2, T3,);
    (T0, T1, T2, T3, T4,);
}

impl Reflect for () {
    blank_reflect!();
}

impl_reflect! { pub struct Uuid; }

impl_reflect! {
    pub struct Cell<T: Debug + Copy>;
}

impl_reflect! {
    pub enum Option<T: Clone + Debug + Reflect> {
        Some(T),
        None
    }
}

impl_reflect! {
    pub struct Range<Idx: Clone + Debug + Reflect> {
        pub start: Idx,
        pub end: Idx,
    }
}

impl<T: Reflect + Clone> Reflect for Box<T> {
    delegate_reflect!();
}

macro_rules! impl_reflect_inner_mutability {
    ($self:ident, $acquire_lock_guard:block, $into_inner:block) => {
        fn source_path() -> &'static str {
            file!()
        }

        fn derived_types() -> &'static [std::any::TypeId] {
            // TODO: This seems to be impossible to implement because of `?Sized` trait bound
            // up above.
            &[]
        }

        fn try_clone_box(&$self) -> Option<Box<dyn Reflect>> {
            let guard = $acquire_lock_guard;
            Some(Box::new(guard.clone()))
        }

        fn query_derived_types(&self) -> &'static [std::any::TypeId] {
            T::derived_types()
        }

        fn type_name(&$self) -> &'static str {
            std::any::type_name::<T>()
        }

        fn doc(&$self) -> &'static str {
            ""
        }

        fn assembly_name(&self) -> &'static str {
            env!("CARGO_PKG_NAME")
        }

        fn type_assembly_name() -> &'static str {
            env!("CARGO_PKG_NAME")
        }

        fn fields_ref(&$self, func: &mut dyn FnMut(&[FieldRef])) {
            let guard = $acquire_lock_guard;
            guard.fields_ref(func)
        }

        fn fields_mut(&mut $self, func: &mut dyn FnMut(&mut [FieldMut])) {
            let mut guard = $acquire_lock_guard;
            guard.fields_mut(func)
        }

        fn into_any($self: Box<Self>) -> Box<dyn Any> {
            let inner = $into_inner;
            Box::new(inner)
        }

        fn as_any(&$self, func: &mut dyn FnMut(&dyn Any)) {
            let guard = $acquire_lock_guard;
            (*guard).as_any(func)
        }

        fn as_any_mut(&mut $self, func: &mut dyn FnMut(&mut dyn Any)) {
            let mut guard = $acquire_lock_guard;
            (*guard).as_any_mut(func)
        }

        fn as_reflect(&$self, func: &mut dyn FnMut(&dyn Reflect)) {
            let guard = $acquire_lock_guard;
            (*guard).as_reflect(func)
        }

        fn as_reflect_mut(&mut $self, func: &mut dyn FnMut(&mut dyn Reflect)) {
            let mut guard = $acquire_lock_guard;
            (*guard).as_reflect_mut(func)
        }

        fn set(&mut $self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
            let mut guard = $acquire_lock_guard;
            guard.set(value)
        }

        fn get_field_direct_ref(&self, index: usize) -> Option<FieldRef> {
            warn!("Cannot get a direct field ref {index}, because it is \
                not supported for types with interior mutability.");
            None
        }

        fn get_field_direct_mut(&mut self, index: usize) -> Option<FieldMut> {
            warn!("Cannot get a direct field ref {index}, because it is \
                not supported for types with interior mutability.");
            None
        }

        fn set_field(
            &mut $self,
            field: &str,
            value: Box<dyn Reflect>,
            func: &mut dyn FnMut(Result<Box<dyn Reflect>, SetFieldError>),
        ) {
            let mut guard = $acquire_lock_guard;
            guard.set_field(field, value, func)
        }

        fn find_field(&$self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
            let guard = $acquire_lock_guard;
            guard.find_field(name, func)
        }

        fn find_field_mut(&mut $self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
            let mut guard = $acquire_lock_guard;
            guard.find_field_mut(name, func)
        }

        fn as_array(&$self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
            let guard = $acquire_lock_guard;
            guard.as_array(func)
        }

        fn as_array_mut(&mut $self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
            let mut guard = $acquire_lock_guard;
            guard.as_array_mut(func)
        }

        fn as_list(&$self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
            let guard = $acquire_lock_guard;
            guard.as_list(func)
        }

        fn as_list_mut(&mut $self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
            let mut guard = $acquire_lock_guard;
            guard.as_list_mut(func)
        }

        fn as_inheritable_variable(
            &$self,
            func: &mut dyn FnMut(Option<&dyn ReflectInheritableVariable>),
        ) {
            let guard = $acquire_lock_guard;
            guard.as_inheritable_variable(func)
        }

        fn as_inheritable_variable_mut(
            &mut $self,
            func: &mut dyn FnMut(Option<&mut dyn ReflectInheritableVariable>),
        ) {
            let mut guard = $acquire_lock_guard;
            guard.as_inheritable_variable_mut(func)
        }

        fn as_hash_map(&$self, func: &mut dyn FnMut(Option<&dyn ReflectHashMap>)) {
            let guard = $acquire_lock_guard;
            guard.as_hash_map(func)
        }

        fn as_hash_map_mut(&mut $self, func: &mut dyn FnMut(Option<&mut dyn ReflectHashMap>)) {
            let mut guard = $acquire_lock_guard;
            guard.as_hash_map_mut(func)
        }
    };
}

impl<T: Reflect + Clone> Reflect for parking_lot::Mutex<T> {
    impl_reflect_inner_mutability!(self, { self.safe_lock() }, { self.into_inner() });
}

impl<T: Reflect + Clone> Reflect for parking_lot::RwLock<T> {
    impl_reflect_inner_mutability!(self, { self.write() }, { self.into_inner() });
}

#[allow(clippy::mut_mutex_lock)]
impl<T: Reflect + Clone> Reflect for std::sync::Mutex<T> {
    impl_reflect_inner_mutability!(self, { self.safe_lock().unwrap() }, { self.into_inner() });
}

impl<T: Reflect + Clone> Reflect for std::sync::RwLock<T> {
    impl_reflect_inner_mutability!(self, { self.write().unwrap() }, { self.into_inner() });
}

impl<T: Reflect + Clone> Reflect for Arc<parking_lot::Mutex<T>> {
    impl_reflect_inner_mutability!(self, { self.safe_lock() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect + Clone> Reflect for Arc<std::sync::Mutex<T>> {
    impl_reflect_inner_mutability!(self, { self.lock().unwrap() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect + Clone> Reflect for Arc<std::sync::RwLock<T>> {
    impl_reflect_inner_mutability!(self, { self.write().unwrap() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect + Clone> Reflect for Arc<parking_lot::RwLock<T>> {
    impl_reflect_inner_mutability!(self, { self.write() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect + Clone> Reflect for RefCell<T> {
    impl_reflect_inner_mutability!(self, { self.borrow_mut() }, { self.into_inner() });
}

impl<T: Reflect + Clone> Reflect for Rc<RefCell<T>> {
    impl_reflect_inner_mutability!(self, { self.borrow_mut() }, {
        Rc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}
