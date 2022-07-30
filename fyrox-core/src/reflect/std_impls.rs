//! `Reflect` implementations for `std` types

use std::{
    any::Any,
    cell::Cell,
    ops::Range,
    time::{Duration, Instant},
};

use fyrox_core_derive::impl_reflect;

use crate::reflect::{blank_reflect, Reflect, ReflectArray, ReflectList};

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
}

impl_reflect! {
    #[reflect(ReflectList)]
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

impl_reflect! {
    pub struct Cell<T>;
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

impl_reflect! {
    pub struct Box<T>;
}
