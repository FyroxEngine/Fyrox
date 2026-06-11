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

use crate::blank_reflect;
use crate::reflect::Reflect;
use crate::reflect::*;
use fyrox_core_derive::impl_reflect;
use uuid::uuid;

/// [`Reflect`] sub trait for working with slices.
pub trait ReflectArray: Reflect {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect>;
    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn reflect_len(&self) -> usize;
}

/// [`Reflect`] sub trait for working with `Vec`-like types
pub trait ReflectList: ReflectArray {
    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;
    fn reflect_pop(&mut self) -> Option<Box<dyn Reflect>>;
    fn reflect_remove(&mut self, index: usize) -> Option<Box<dyn Reflect>>;
    fn reflect_insert(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), Box<dyn Reflect>>;
}

impl<const N: usize, T: Reflect + Clone + PartialEq> ReflectArray for [T; N] {
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

impl<const N: usize, T: Reflect + Clone + PartialEq> Reflect for [T; N] {
    // TODO: combine uuids.
    blank_reflect!("6d2a2f2d-d74e-4125-8840-b4910aa2e0cc");

    fn as_array(&self) -> Option<&dyn ReflectArray> {
        Some(self)
    }

    fn as_array_mut(&mut self) -> Option<&mut dyn ReflectArray> {
        Some(self)
    }
}

impl_reflect! {
    #[reflect(ReflectList, ReflectArray, type_uuid = "2d704c2b-c87e-4489-b680-aa9699ba2c91")]
    pub struct Vec<T: Reflect + Clone + PartialEq>;
}

impl<T: Reflect + Clone + PartialEq> ReflectArray for Vec<T> {
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
impl<T: Reflect + Clone + PartialEq> ReflectList for Vec<T> {
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
