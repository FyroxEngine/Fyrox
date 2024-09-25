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
    algebra::{Vector2, Vector3, Vector4},
    arrayvec::ArrayVec,
    value_as_u8_slice,
};
use bytemuck::Pod;

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

pub trait AlignmentProvider: Pod {
    const ALIGNMENT: usize;
}

macro_rules! define_alignment {
    ($inner_type:ty = $alignment:expr) => {
        impl AlignmentProvider for $inner_type {
            const ALIGNMENT: usize = $alignment;
        }
    };
}

define_alignment!(u32 = 4);
define_alignment!(f32 = 4);
define_alignment!(Vector2<f32> = 8);
define_alignment!(Vector3<f32> = 16);
define_alignment!(Vector4<f32> = 16);
define_alignment!([f32; 2] = 8);
define_alignment!([f32; 3] = 16);
define_alignment!([f32; 4] = 16);

impl<S> UniformBuffer<S>
where
    S: ByteStorage,
{
    pub fn len(&self) -> usize {
        self.storage.bytes_count()
    }

    fn push_padding(&mut self, alignment: usize) {
        let bytes_count = self.storage.bytes_count();
        let remainder = bytes_count % alignment;
        if remainder > 0 {
            let padding = alignment - remainder;
            for _ in 0..padding {
                self.storage.write_bytes(&[0]);
            }
        }
    }

    fn push_raw<T>(&mut self, value: &T)
    where
        T: Pod,
    {
        self.storage.write_bytes(value_as_u8_slice(value))
    }

    pub fn push<T>(&mut self, value: &T)
    where
        T: AlignmentProvider,
    {
        self.push_padding(T::ALIGNMENT);
        self.push_raw(value)
    }

    pub fn push_slice<T>(&mut self, slice: &[T])
    where
        T: AlignmentProvider,
    {
        for item in slice {
            self.push_padding(16);
            self.push_raw(item);
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.storage.bytes()
    }
}

#[cfg(test)]
mod test {
    use crate::uniform::DynamicUniformBuffer;
    use fyrox_core::algebra::{Matrix3, Vector3, Vector4};

    #[test]
    fn test_uniform_buffer() {
        let mut buffer = DynamicUniformBuffer::default();
        buffer.push(&123.321);
        assert_eq!(buffer.len(), 4);
        buffer.push(&Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(buffer.len(), 28);
        buffer.push(&Vector4::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(buffer.len(), 48);
        buffer.push_slice(Matrix3::default().as_ref());
    }
}
