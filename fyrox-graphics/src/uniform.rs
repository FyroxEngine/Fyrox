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

use crate::core::{
    algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
    array_as_u8_slice,
    arrayvec::ArrayVec,
    color::Color,
    value_as_u8_slice,
};

pub trait ByteStorage: Default {
    fn bytes(&self) -> &[u8];
    fn bytes_count(&self) -> usize;
    fn write_bytes(&mut self, bytes: &[u8]);
}

impl<const N: usize> ByteStorage for ArrayVec<u8, N> {
    fn bytes(&self) -> &[u8] {
        self.as_slice()
    }

    fn bytes_count(&self) -> usize {
        self.len()
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.try_extend_from_slice(bytes).unwrap()
    }
}

impl ByteStorage for Vec<u8> {
    fn bytes(&self) -> &[u8] {
        self.as_slice()
    }

    fn bytes_count(&self) -> usize {
        self.len()
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes)
    }
}

#[derive(Default)]
pub struct UniformBuffer<S: ByteStorage> {
    storage: S,
}

pub type StaticUniformBuffer<const N: usize> = UniformBuffer<ArrayVec<u8, N>>;
pub type DynamicUniformBuffer = UniformBuffer<Vec<u8>>;

pub trait Std140 {
    const ALIGNMENT: usize;

    fn write<T: ByteStorage>(&self, dest: &mut T);
}

macro_rules! default_write_impl {
    () => {
        fn write<T: ByteStorage>(&self, dest: &mut T) {
            dest.write_bytes(value_as_u8_slice(self))
        }
    };
}

impl Std140 for f32 {
    const ALIGNMENT: usize = 4;
    default_write_impl!();
}

impl Std140 for u32 {
    const ALIGNMENT: usize = 4;
    default_write_impl!();
}

impl Std140 for i32 {
    const ALIGNMENT: usize = 4;
    default_write_impl!();
}

impl Std140 for Vector2<f32> {
    const ALIGNMENT: usize = 8;
    default_write_impl!();
}

impl Std140 for Vector3<f32> {
    const ALIGNMENT: usize = 16;
    default_write_impl!();
}

impl Std140 for Vector4<f32> {
    const ALIGNMENT: usize = 16;
    default_write_impl!();
}

impl Std140 for [f32; 2] {
    const ALIGNMENT: usize = 8;
    default_write_impl!();
}

impl Std140 for [f32; 3] {
    const ALIGNMENT: usize = 16;

    fn write<T: ByteStorage>(&self, dest: &mut T) {
        dest.write_bytes(value_as_u8_slice(self));
        dest.write_bytes(&[0; size_of::<f32>()]);
    }
}

impl Std140 for Matrix4<f32> {
    const ALIGNMENT: usize = 16;
    default_write_impl!();
}

impl Std140 for Matrix3<f32> {
    const ALIGNMENT: usize = 16;

    fn write<T: ByteStorage>(&self, dest: &mut T) {
        for row in self.as_ref() {
            dest.write_bytes(array_as_u8_slice(row));
            dest.write_bytes(&[0; size_of::<f32>()]);
        }
    }
}

impl Std140 for Matrix2<f32> {
    const ALIGNMENT: usize = 16;

    fn write<T: ByteStorage>(&self, dest: &mut T) {
        for row in self.as_ref() {
            dest.write_bytes(array_as_u8_slice(row));
            dest.write_bytes(&[0; 2 * size_of::<f32>()]);
        }
    }
}

impl Std140 for [f32; 4] {
    const ALIGNMENT: usize = 16;
    default_write_impl!();
}

impl Std140 for Color {
    const ALIGNMENT: usize = 16;

    fn write<T: ByteStorage>(&self, dest: &mut T) {
        let frgba = self.as_frgba();
        dest.write_bytes(value_as_u8_slice(&frgba));
    }
}

impl Std140 for bool {
    const ALIGNMENT: usize = 4;

    fn write<T: ByteStorage>(&self, dest: &mut T) {
        let integer = if *self { 1 } else { 0 };
        dest.write_bytes(value_as_u8_slice(&integer));
    }
}

impl<S> UniformBuffer<S>
where
    S: ByteStorage + Default,
{
    pub fn new() -> Self {
        Self {
            storage: S::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.storage.bytes_count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn push_padding(&mut self, alignment: usize) {
        debug_assert!(alignment.is_power_of_two());
        let bytes_count = self.storage.bytes_count();
        let remainder = (alignment - 1) & bytes_count;
        if remainder > 0 {
            let padding = alignment - remainder;
            match padding {
                2 => self.storage.write_bytes(&[0; 2]),
                4 => self.storage.write_bytes(&[0; 4]),
                8 => self.storage.write_bytes(&[0; 8]),
                12 => self.storage.write_bytes(&[0; 12]),
                16 => self.storage.write_bytes(&[0; 16]),
                _ => {
                    for _ in 0..padding {
                        self.storage.write_bytes(&[0]);
                    }
                }
            }
        }
    }

    pub fn push<T>(&mut self, value: &T) -> &mut Self
    where
        T: Std140,
    {
        self.push_padding(T::ALIGNMENT);
        value.write(&mut self.storage);
        self
    }

    pub fn with<T>(mut self, value: &T) -> Self
    where
        T: Std140,
    {
        self.push(value);
        self
    }

    pub fn push_slice<T>(&mut self, slice: &[T]) -> &mut Self
    where
        T: Std140,
    {
        for item in slice {
            self.push_padding(16);
            item.write(&mut self.storage);
            self.push_padding(16);
        }
        self
    }

    pub fn with_slice<T>(mut self, slice: &[T]) -> Self
    where
        T: Std140,
    {
        self.push_slice(slice);
        self
    }

    pub fn finish(mut self) -> S {
        self.push_padding(16);
        self.storage
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::algebra::{Matrix3, Vector3, Vector4},
        uniform::DynamicUniformBuffer,
    };

    #[test]
    fn test_uniform_buffer() {
        let mut buffer = DynamicUniformBuffer::default();
        buffer.push(&123.321);
        assert_eq!(buffer.len(), 4);
        buffer.push(&Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(buffer.len(), 28);
        buffer.push(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(buffer.len(), 48);
        buffer.push(&Matrix3::default());
        assert_eq!(buffer.len(), 96);
        buffer.push(&123.0);
        assert_eq!(buffer.len(), 100);
        buffer.push_slice(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(buffer.len(), 176);
        let bytes = buffer.finish();
        assert_eq!(bytes.len(), 176);
    }

    #[test]
    fn test_uniform_buffer_mixed_alignment() {
        let mut buffer = DynamicUniformBuffer::default();
        buffer.push(&Vector3::repeat(1.0));
        assert_eq!(buffer.len(), 12);
        buffer.push(&1.0);
        assert_eq!(buffer.len(), 16);
    }
}
