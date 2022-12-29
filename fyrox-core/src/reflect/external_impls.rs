//! `Reflect` implementations for external types othern than `std` types

use fyrox_core_derive::impl_reflect;
use nalgebra::*;
use std::fmt::Debug;

use crate::reflect::prelude::*;

impl_reflect! {
    pub struct Matrix<T: 'static, R: Dim + 'static, C: Dim + 'static, S: 'static> {
        pub data: S,
        // _phantoms: PhantomData<(T, R, C)>,
    }
}

impl_reflect! {
    pub struct ArrayStorage<T: Debug, const R: usize, const C: usize>(pub [[T; R]; C]);
}

impl_reflect! {
    pub struct Unit<T: Debug + 'static> {
        // pub(crate) value: T,
    }
}

impl_reflect! {
    pub struct Quaternion<T: Debug> {
        pub coords: Vector4<T>,
    }
}
