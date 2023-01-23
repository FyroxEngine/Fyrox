//! `Reflect` implementations for `std` types

use crate::{
    delegate_reflect,
    reflect::{blank_reflect, prelude::*, ReflectArray, ReflectInheritableVariable, ReflectList},
    uuid::Uuid,
};
use fyrox_core_derive::impl_reflect;
use parking_lot::Mutex;
use std::{
    any::Any,
    cell::Cell,
    fmt::Debug,
    ops::{Deref, DerefMut, Range},
    sync::Arc,
    time::{Duration, Instant},
};

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
    bool,
    String,
    std::path::PathBuf,
    Duration, Instant,
}

macro_rules! impl_reflect_tuple {
    (
        $(
            ( $($t:ident,)* );
        )*
    ) => {
        $(
            impl< $($t: Reflect),* > Reflect for ( $($t,)* ) {
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

impl<const N: usize, T: Reflect> Reflect for [T; N] {
    blank_reflect!();

    fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
        func(Some(self))
    }

    fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
        func(Some(self))
    }
}

impl<const N: usize, T: Reflect> ReflectArray for [T; N] {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect> {
        if let Some(item) = self.get(index) {
            Some(item)
        } else {
            None
        }
    }

    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        if let Some(item) = self.get_mut(index) {
            Some(item)
        } else {
            None
        }
    }

    fn reflect_len(&self) -> usize {
        self.len()
    }
}

impl_reflect! {
    #[reflect(ReflectList, ReflectArray)]
    pub struct Vec<T: Reflect + 'static>;
}

impl<T: Reflect + 'static> ReflectArray for Vec<T> {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect> {
        self.get(index).map(|x| x as &dyn Reflect)
    }

    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.get_mut(index).map(|x| x as &mut dyn Reflect)
    }

    fn reflect_len(&self) -> usize {
        self.len()
    }
}

/// REMARK: `Reflect` is implemented for `Vec<T>` where `T: Reflect` only.
impl<T: Reflect + 'static> ReflectList for Vec<T> {
    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        self.push(*value.downcast::<T>()?);
        Ok(())
    }

    fn reflect_pop(&mut self) -> Option<Box<dyn Reflect>> {
        if let Some(item) = self.pop() {
            Some(Box::new(item))
        } else {
            None
        }
    }

    fn reflect_remove(&mut self, index: usize) -> Option<Box<dyn Reflect>> {
        if index < self.len() {
            Some(Box::new(self.remove(index)))
        } else {
            None
        }
    }

    fn reflect_insert(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), Box<dyn Reflect>> {
        self.insert(index, *value.downcast::<T>()?);
        Ok(())
    }
}

impl Reflect for () {
    blank_reflect!();
}

impl_reflect! { pub struct Uuid; }

impl_reflect! {
    pub struct Cell<T: Debug + Copy>;
}

impl_reflect! {
    pub enum Option<T> {
        Some(T),
        None
    }
}

impl_reflect! {
    pub struct Range<Idx> {
        pub start: Idx,
        pub end: Idx,
    }
}

impl<T: ?Sized + Reflect> Reflect for Box<T> {
    delegate_reflect!();
}

macro_rules! impl_mutex_reflect {
    ($self:ident, $acquire_lock_guard:block) => {
        fn type_name(&$self) -> &'static str {
            std::any::type_name::<T>()
        }

        fn fields_info(&$self, func: &mut dyn FnMut(Vec<FieldInfo>)) {
            let guard = $acquire_lock_guard;
            guard.fields_info(func)
        }

        fn into_any($self: Box<Self>) -> Box<dyn Any> {
            // Clone the inner value and box it.
            let guard = $acquire_lock_guard;
            Box::new(guard.clone())
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

        fn set_field(
            &mut $self,
            field: &str,
            value: Box<dyn Reflect>,
            func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),
        ) {
            let mut guard = $acquire_lock_guard;
            guard.set_field(field, value, func)
        }

        fn fields(&$self, func: &mut dyn FnMut(Vec<&dyn Reflect>)) {
            let guard = $acquire_lock_guard;
            guard.fields(func)
        }

        fn fields_mut(&mut $self, func: &mut dyn FnMut(Vec<&mut dyn Reflect>)) {
            let mut guard = $acquire_lock_guard;
            guard.fields_mut(func)
        }

        fn field(&$self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
            let guard = $acquire_lock_guard;
            guard.field(name, func)
        }

        fn field_mut(&mut $self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
            let mut guard = $acquire_lock_guard;
            guard.field_mut(name, func)
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
    };
}

impl<T: Reflect + Clone> Reflect for Mutex<T> {
    impl_mutex_reflect!(self, { self.lock() });
}

#[allow(clippy::mut_mutex_lock)]
impl<T: Reflect + Clone> Reflect for std::sync::Mutex<T> {
    impl_mutex_reflect!(self, { self.lock().unwrap() });
}

impl<T: Reflect + Clone> Reflect for Arc<Mutex<T>> {
    impl_mutex_reflect!(self, { self.lock() });
}

impl<T: Reflect + Clone> Reflect for Arc<std::sync::Mutex<T>> {
    impl_mutex_reflect!(self, { self.lock().unwrap() });
}
