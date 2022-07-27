//! `Reflect` implementations for external types othern than `std` types

use fyrox_core_derive::impl_reflect;
use nalgebra::*;

use crate::reflect::Reflect;

impl_reflect! {
    pub struct Matrix<T: 'static, R: 'static, C: 'static, S: 'static> {
        pub data: S,
        // _phantoms: PhantomData<(T, R, C)>,
    }
}

impl_reflect! {
    pub struct ArrayStorage<T, const R: usize, const C: usize>(pub [[T; R]; C]);
}

impl_reflect! {
    pub struct Unit<T: 'static> {
        // pub(crate) value: T,
    }
}

impl_reflect! {
    pub struct Quaternion<T> {
        pub coords: Vector4<T>,
    }
}
