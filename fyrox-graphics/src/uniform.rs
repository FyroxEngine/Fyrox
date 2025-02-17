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

#![warn(missing_docs)]

//! Uniform buffer is a special byte storage that ensures correct data alignment suitable for GPU.
//! Current implementation supports `std140` data layout scheme.

use crate::core::{
    algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
    array_as_u8_slice,
    arrayvec::ArrayVec,
    color::Color,
    value_as_u8_slice,
};

/// A trait for any storage suitable to store bytes for uniforms.
pub trait ByteStorage {
    /// Clears the storage.
    fn reset(&mut self);
    /// Returns a reference to the internal bytes array.
    fn bytes(&self) -> &[u8];
    /// Returns total number of bytes that is currently in the storage.
    fn bytes_count(&self) -> usize;
    /// Writes the given number of bytes to the storage.
    fn write_bytes(&mut self, bytes: &[u8]);
    /// Writes the given number of zero bytes to the storage.
    fn write_zeros(&mut self, count: usize);
    /// Writes zeros to ensure that the last byte in the storage has a position that is multiple with
    /// the given alignment. The alignment must be power of two.
    fn push_padding(&mut self, alignment: usize) {
        debug_assert!(alignment.is_power_of_two());
        let bytes_count = self.bytes_count();
        let remainder = (alignment - 1) & bytes_count;
        if remainder > 0 {
            let padding = alignment - remainder;
            self.write_zeros(padding);
        }
    }
}

impl<const N: usize> ByteStorage for ArrayVec<u8, N> {
    fn reset(&mut self) {
        self.clear();
    }

    fn bytes(&self) -> &[u8] {
        self.as_slice()
    }

    fn bytes_count(&self) -> usize {
        self.len()
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.try_extend_from_slice(bytes).unwrap()
    }

    fn write_zeros(&mut self, count: usize) {
        let old_len = self.len();
        let new_len = old_len + count;
        assert!(new_len <= self.capacity());
        // SAFETY: Out-of-bounds writes prevented by the above assert.
        unsafe {
            self.set_len(new_len);
            std::ptr::write_bytes(self.as_mut_ptr().add(old_len), 0, count)
        }
    }
}

impl ByteStorage for Vec<u8> {
    fn reset(&mut self) {
        self.clear();
    }

    fn bytes(&self) -> &[u8] {
        self.as_slice()
    }

    fn bytes_count(&self) -> usize {
        self.len()
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes)
    }

    // clippy is not smart enough here. In reality there's no uninitialized content in the vec,
    // because we do `write_bytes` right after.
    #[allow(clippy::uninit_vec)]
    fn write_zeros(&mut self, count: usize) {
        let old_len = self.len();
        let new_len = old_len + count;
        self.reserve(count);
        // SAFETY: Out-of-bounds writes prevented by the `reserve` call.
        unsafe {
            self.set_len(new_len);
            std::ptr::write_bytes(self.as_mut_ptr().add(old_len), 0, count)
        }
    }
}

/// Uniform buffer is a special byte storage that ensures correct data alignment suitable for GPU.
/// Current implementation supports `std140` data layout scheme.
///
/// ## Examples
///
/// ```rust
/// # use fyrox_core::{
/// #     algebra::{Matrix4, Vector3},
/// #     color::Color
/// # };
/// # use fyrox_graphics::uniform::StaticUniformBuffer;
/// let bytes = StaticUniformBuffer::<256>::new()
///     .with(&Matrix4::identity())
///     .with(&Color::WHITE)
///     .with(&Vector3::new(0.0, 1.0, 0.0))
///     .finish();
/// ```
#[derive(Default)]
pub struct UniformBuffer<S: ByteStorage> {
    storage: S,
}

/// A uniform buffer backed by an array of fixed size.
pub type StaticUniformBuffer<const N: usize> = UniformBuffer<ArrayVec<u8, N>>;

/// A uniform buffer backed by a dynamic array.
pub type DynamicUniformBuffer = UniformBuffer<Vec<u8>>;

/// A trait for entities that supports `std140` data layout.
pub trait Std140 {
    /// Writes self to the given bytes storage.
    fn write(&self, dest: &mut dyn ByteStorage);
}

macro_rules! default_write_impl {
    ($alignment:expr) => {
        fn write(&self, dest: &mut dyn ByteStorage) {
            dest.push_padding($alignment);
            dest.write_bytes(value_as_u8_slice(self))
        }
    };
}

impl Std140 for f32 {
    default_write_impl!(4);
}

impl Std140 for u32 {
    default_write_impl!(4);
}

impl Std140 for i32 {
    default_write_impl!(4);
}

impl Std140 for Vector2<f32> {
    default_write_impl!(8);
}

impl Std140 for Vector3<f32> {
    default_write_impl!(16);
}

impl Std140 for Vector4<f32> {
    default_write_impl!(16);
}

impl Std140 for Matrix4<f32> {
    default_write_impl!(16);
}

impl Std140 for Matrix3<f32> {
    fn write(&self, dest: &mut dyn ByteStorage) {
        dest.push_padding(16);
        for row in (self as &dyn AsRef<[[f32; 3]; 3]>).as_ref() {
            dest.write_bytes(array_as_u8_slice(row));
            dest.write_bytes(&[0; size_of::<f32>()]);
        }
    }
}

impl Std140 for Matrix2<f32> {
    fn write(&self, dest: &mut dyn ByteStorage) {
        dest.push_padding(16);
        for row in (self as &dyn AsRef<[[f32; 2]; 2]>).as_ref() {
            dest.write_bytes(array_as_u8_slice(row));
            dest.write_bytes(&[0; 2 * size_of::<f32>()]);
        }
    }
}

impl Std140 for Color {
    fn write(&self, dest: &mut dyn ByteStorage) {
        dest.push_padding(16);
        let frgba = self.as_frgba();
        dest.write_bytes(value_as_u8_slice(&frgba));
    }
}

impl Std140 for bool {
    fn write(&self, dest: &mut dyn ByteStorage) {
        dest.push_padding(4);
        let integer = if *self { 1 } else { 0 };
        dest.write_bytes(value_as_u8_slice(&integer));
    }
}

fn write_array(arr: &[impl Std140], dest: &mut dyn ByteStorage) {
    for item in arr {
        dest.push_padding(16);
        item.write(dest);
        dest.push_padding(16);
    }
}

impl<T: Std140, const N: usize> Std140 for [T; N] {
    fn write(&self, dest: &mut dyn ByteStorage) {
        write_array(self, dest)
    }
}

impl<T: Std140> Std140 for [T] {
    fn write(&self, dest: &mut dyn ByteStorage) {
        write_array(self, dest)
    }
}

impl<S> UniformBuffer<S>
where
    S: ByteStorage,
{
    /// Creates a new uniform buffer with an empty storage.
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            storage: S::default(),
        }
    }

    /// Creates a new uniform buffer with the given storage.
    pub fn with_storage(storage: S) -> Self {
        Self { storage }
    }

    /// Clears the uniform buffer.
    pub fn clear(&mut self) {
        self.storage.reset();
    }

    /// Returns total number of bytes stored in the uniform buffer. Keep in mind, that the number
    /// in the vast majority of cases won't match the sum of all pushed elements due to alignment
    /// requirements.
    pub fn len(&self) -> usize {
        self.storage.bytes_count()
    }

    /// Checks if the buffer is empty or not.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pushes the given amount of padding bytes to the storage.
    pub fn push_padding(&mut self, alignment: usize) {
        self.storage.push_padding(alignment);
    }

    /// Pushes a value to the storage. This method ensures that the correct alignment for the pushed
    /// value is preserved.
    pub fn push<T>(&mut self, value: &T) -> &mut Self
    where
        T: Std140 + ?Sized,
    {
        value.write(&mut self.storage);
        self
    }

    /// The same as [`Self::push`], but allows chained calls in a builder manner.
    pub fn with<T>(mut self, value: &T) -> Self
    where
        T: Std140 + ?Sized,
    {
        self.push(value);
        self
    }

    fn push_array_element<T: Std140>(&mut self, item: &T) {
        self.push_padding(16);
        item.write(&mut self.storage);
        self.push_padding(16);
    }

    /// Pushes the given slice into the uniform buffer and pads the rest of the space
    /// (`max_len - slice_len`) with the default value of the underlying type.
    pub fn push_slice_with_max_size<T: Std140 + Default>(
        &mut self,
        slice: &[T],
        max_len: usize,
    ) -> &mut Self {
        let len = slice.len();
        if !slice.is_empty() {
            let end = max_len.min(len);
            self.push(&slice[0..end]);
        }
        let remainder = max_len.saturating_sub(len);
        let item = T::default();
        for _ in 0..remainder {
            self.push_array_element(&item);
        }
        self
    }

    /// Same as [`Self::push_slice_with_max_size`], but allows changed calls with builder-like style.
    pub fn with_slice_with_max_size<T: Std140 + Default>(
        mut self,
        slice: &[T],
        max_len: usize,
    ) -> Self {
        self.push_slice_with_max_size(slice, max_len);
        self
    }

    /// Returns a reference to the internal bytes storage of the uniform buffer.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Finishes buffer filling process and returns the backing storage by consuming the buffer. This
    /// method **must** be called before sending the data GPU, otherwise the buffer may contain misaligned
    /// data.
    pub fn finish(mut self) -> S {
        self.push_padding(16);
        self.storage
    }

    /// Calculates position for the next element including the given alignment.
    pub fn next_write_aligned_position(&self, alignment: usize) -> usize {
        let position = self.storage.bytes_count();
        let remainder = (alignment - 1) & position;
        if remainder > 0 {
            let padding = alignment - remainder;
            position + padding
        } else {
            position
        }
    }

    /// Writes bytes directly to the buffer with the given alignment. Important: this method could
    /// be dangerous if misused, the alignment argument must be correct and comply with `std140`
    /// data layout rules.
    pub fn write_bytes_with_alignment(&mut self, bytes: &[u8], alignment: usize) -> usize {
        self.push_padding(alignment);
        let data_location = self.storage.bytes_count();
        self.storage.write_bytes(bytes);
        data_location
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::algebra::{Matrix3, Vector3, Vector4},
        uniform::DynamicUniformBuffer,
    };
    use fyrox_core::transmute_slice;

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
        buffer.push(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(buffer.len(), 176);
        buffer.push(&[1.0, 2.0, 3.0]);
        assert_eq!(buffer.len(), 224);
        buffer.push(&[1.0, 2.0]);
        assert_eq!(buffer.len(), 256);
        let bytes = buffer.finish();
        assert_eq!(bytes.len(), 256);
    }

    #[test]
    fn test_uniform_buffer_mixed_alignment() {
        let mut buffer = DynamicUniformBuffer::default();
        buffer.push(&Vector3::repeat(1.0));
        assert_eq!(buffer.len(), 12);
        buffer.push(&1.0);
        assert_eq!(buffer.len(), 16);
    }

    #[test]
    fn test_push_with_max_len() {
        let mut buffer = DynamicUniformBuffer::default();
        buffer.push_slice_with_max_size(&[1.0, 2.0, 3.0, 4.0], 6);
        let floats: &[f32] = transmute_slice(buffer.storage().as_slice());
        assert_eq!(
            floats,
            &[
                1.0, 0.0, 0.0, 0.0, // 1
                2.0, 0.0, 0.0, 0.0, // 2
                3.0, 0.0, 0.0, 0.0, // 3
                4.0, 0.0, 0.0, 0.0, // 4
                0.0, 0.0, 0.0, 0.0, // Zero with padding
                0.0, 0.0, 0.0, 0.0, // Zero with padding
            ]
        );
        buffer.clear();
        buffer.push_slice_with_max_size(&[1.0, 2.0], 1);
        let floats: &[f32] = transmute_slice(buffer.storage().as_slice());
        assert_eq!(floats, &[1.0, 0.0, 0.0, 0.0,]);
    }
}
