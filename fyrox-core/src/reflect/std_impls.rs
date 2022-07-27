//! `Reflect` implementations for `std` types

use std::{
    any::Any,
    cell::Cell,
    ops::Range,
    time::{Duration, Instant},
};

use fyrox_core_derive::impl_reflect;

use crate::reflect::{blank_reflect, Reflect, ReflectList};

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

/// REMARK: `Reflect` is implemented for `Vec<T>` where `T: Reflect` only.
impl<T: Reflect + 'static> ReflectList for Vec<T> {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect> {
        self.get(index).map(|x| x as &dyn Reflect)
    }

    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.get_mut(index).map(|x| x as &mut dyn Reflect)
    }

    fn reflect_push(&mut self, value: Box<dyn Reflect>) {
        if let Ok(value) = value.downcast::<T>() {
            self.push(*value);
        } else {
            // log?
        }
    }

    fn reflect_len(&self) -> usize {
        self.len()
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
