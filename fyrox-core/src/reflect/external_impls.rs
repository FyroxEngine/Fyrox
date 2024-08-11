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
