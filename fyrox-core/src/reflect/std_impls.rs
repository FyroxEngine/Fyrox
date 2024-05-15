//! `Reflect` implementations for `std` types

use crate::{
    delegate_reflect,
    reflect::{blank_reflect, prelude::*},
    sstorage::ImmutableString,
    uuid::Uuid,
};
use fyrox_core_derive::impl_reflect;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Debug,
    hash::{BuildHasher, Hash},
    ops::{Deref, DerefMut, Range},
    rc::Rc,
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

impl<K, V, S> Reflect for HashMap<K, V, S>
where
    K: Reflect + Debug + Eq + Hash + 'static,
    V: Reflect + Debug + 'static,
    S: BuildHasher + 'static,
{
    blank_reflect!();

    fn as_hash_map(&self, func: &mut dyn FnMut(Option<&dyn ReflectHashMap>)) {
        func(Some(self))
    }

    fn as_hash_map_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectHashMap>)) {
        func(Some(self))
    }
}

impl<K, V, S> ReflectHashMap for HashMap<K, V, S>
where
    K: Reflect + Debug + Eq + Hash + 'static,
    V: Reflect + Debug + 'static,
    S: BuildHasher + 'static,
{
    fn reflect_insert(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        if let Ok(key) = key.downcast::<K>() {
            if let Ok(value) = value.downcast::<V>() {
                if let Some(previous) = self.insert(*key, *value) {
                    return Some(Box::new(previous));
                }
            }
        }

        None
    }

    fn reflect_len(&self) -> usize {
        self.len()
    }

    fn reflect_get(&self, key: &dyn Reflect, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        key.downcast_ref::<K>(&mut |result| match result {
            Some(key) => match self.get(key) {
                Some(value) => func(Some(value as &dyn Reflect)),
                None => func(None),
            },
            None => func(None),
        })
    }

    fn reflect_get_mut(
        &mut self,
        key: &dyn Reflect,
        func: &mut dyn FnMut(Option<&mut dyn Reflect>),
    ) {
        key.downcast_ref::<K>(&mut |result| match result {
            Some(key) => match self.get_mut(key) {
                Some(value) => func(Some(value as &mut dyn Reflect)),
                None => func(None),
            },
            None => func(None),
        })
    }

    fn reflect_get_nth_value_ref(&self, index: usize) -> Option<&dyn Reflect> {
        self.values().nth(index).map(|v| v as &dyn Reflect)
    }

    fn reflect_get_nth_value_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.values_mut().nth(index).map(|v| v as &mut dyn Reflect)
    }

    fn reflect_get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
        self.iter()
            .nth(index)
            .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect))
    }

    fn reflect_get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)> {
        self.iter_mut()
            .nth(index)
            .map(|(k, v)| (k as &dyn Reflect, v as &mut dyn Reflect))
    }

    fn reflect_remove(
        &mut self,
        key: &dyn Reflect,
        func: &mut dyn FnMut(Option<Box<dyn Reflect>>),
    ) {
        key.downcast_ref::<K>(&mut |result| match result {
            Some(key) => func(
                self.remove(key)
                    .map(|value| Box::new(value) as Box<dyn Reflect>),
            ),
            None => func(None),
        })
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

macro_rules! impl_reflect_inner_mutability {
    ($self:ident, $acquire_lock_guard:block, $into_inner:block) => {
        fn source_path() -> &'static str {
            file!()
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

        fn fields_info(&$self, func: &mut dyn FnMut(&[FieldInfo])) {
            let guard = $acquire_lock_guard;
            guard.fields_info(func)
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

        fn set_field(
            &mut $self,
            field: &str,
            value: Box<dyn Reflect>,
            func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),
        ) {
            let mut guard = $acquire_lock_guard;
            guard.set_field(field, value, func)
        }

        fn fields(&$self, func: &mut dyn FnMut(&[&dyn Reflect])) {
            let guard = $acquire_lock_guard;
            guard.fields(func)
        }

        fn fields_mut(&mut $self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
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

impl<T: Reflect> Reflect for parking_lot::Mutex<T> {
    impl_reflect_inner_mutability!(self, { self.lock() }, { self.into_inner() });
}

impl<T: Reflect> Reflect for parking_lot::RwLock<T> {
    impl_reflect_inner_mutability!(self, { self.write() }, { self.into_inner() });
}

#[allow(clippy::mut_mutex_lock)]
impl<T: Reflect> Reflect for std::sync::Mutex<T> {
    impl_reflect_inner_mutability!(self, { self.lock().unwrap() }, { self.into_inner() });
}

impl<T: Reflect> Reflect for std::sync::RwLock<T> {
    impl_reflect_inner_mutability!(self, { self.write().unwrap() }, { self.into_inner() });
}

impl<T: Reflect> Reflect for Arc<parking_lot::Mutex<T>> {
    impl_reflect_inner_mutability!(self, { self.lock() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect> Reflect for Arc<std::sync::Mutex<T>> {
    impl_reflect_inner_mutability!(self, { self.lock().unwrap() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect> Reflect for Arc<std::sync::RwLock<T>> {
    impl_reflect_inner_mutability!(self, { self.write().unwrap() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect> Reflect for Arc<parking_lot::RwLock<T>> {
    impl_reflect_inner_mutability!(self, { self.write() }, {
        Arc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}

impl<T: Reflect> Reflect for RefCell<T> {
    impl_reflect_inner_mutability!(self, { self.borrow_mut() }, { self.into_inner() });
}

impl<T: Reflect> Reflect for Rc<RefCell<T>> {
    impl_reflect_inner_mutability!(self, { self.borrow_mut() }, {
        Rc::into_inner(*self)
            .expect("Value cannot be shared!")
            .into_inner()
    });
}
