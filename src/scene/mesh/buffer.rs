//! Vertex buffer with dynamic layout.
//!
//! # Limitations
//!
//! Vertex size cannot be more than 256 bytes, this limitation shouldn't be a problem because  
//! almost every GPU supports up to 16 vertex attributes with 16 bytes of size each, which
//! gives exactly 256 bytes.

#![allow(missing_docs)] // TEMPORARY

use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        arrayvec::ArrayVec,
        byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt},
        futures::io::Error,
        visitor::prelude::*,
    },
    utils::value_as_u8_slice,
};
use std::{marker::PhantomData, mem::MaybeUninit};

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Visit, Debug)]
#[repr(u8)]
pub enum VertexAttributeDataKind {
    F32,
    U32,
    U16,
    U8,
}

impl Default for VertexAttributeDataKind {
    fn default() -> Self {
        Self::F32
    }
}

impl VertexAttributeDataKind {
    pub fn size(self) -> u8 {
        match self {
            VertexAttributeDataKind::F32 | VertexAttributeDataKind::U32 => 4,
            VertexAttributeDataKind::U16 => 2,
            VertexAttributeDataKind::U8 => 1,
        }
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Visit, Debug)]
#[repr(u32)]
pub enum VertexAttributeKind {
    /// Vertex position. Usually Vector2<f32> or Vector3<f32>.
    Position = 0,
    /// Vertex normal. Usually Vector3<f32>, more rare Vector3<u16> (F16).
    Normal = 1,
    /// Vertex tangent. Usually Vector3<f32>.
    Tangent = 2,
    /// First texture coordinates. Usually Vector2<f32>.
    /// It may be used for everything else, not only for texture coordinates.
    TexCoord0 = 3,
    /// Second texture coordinates.
    TexCoord1 = 4,
    /// Third texture coordinates.
    TexCoord2 = 5,
    /// Fourth texture coordinates.
    TexCoord3 = 6,
    /// Fifth texture coordinates.
    TexCoord4 = 7,
    /// Sixth texture coordinates.
    TexCoord5 = 8,
    /// Seventh texture coordinates.
    TexCoord6 = 9,
    /// Eighth texture coordinates.
    TexCoord7 = 10,
    /// Bone weights. Usually Vector4<f32>.
    BoneWeight = 11,
    /// Bone indices. Usually Vector4<u8>.
    BoneIndices = 12,
    /// Maximum amount of attribute kinds.
    Count,
}

impl Default for VertexAttributeKind {
    fn default() -> Self {
        Self::Position
    }
}

#[derive(Debug)]
pub struct VertexAttributeDescriptor {
    pub kind: VertexAttributeKind,
    pub component_type: VertexAttributeDataKind,
    pub size: u8,
    pub divisor: u8,
    /// Defines location of the attribute in a shader (`layout(location = x) attrib;`)
    pub shader_location: u8,
}

#[derive(Visit, Copy, Clone, Default, Debug)]
pub struct VertexAttribute {
    pub kind: VertexAttributeKind,
    pub component_type: VertexAttributeDataKind,
    pub size: u8,
    pub divisor: u8,
    pub offset: u8,
    /// Defines location of the attribute in a shader (`layout(location = x) attrib;`)
    pub shader_location: u8,
}

#[derive(Clone, Visit, Default, Debug)]
pub struct VertexBuffer {
    dense_layout: Vec<VertexAttribute>,
    sparse_layout: [Option<VertexAttribute>; 13],
    vertex_size: u8,
    vertex_count: u32,
    data: Vec<u8>,
}

#[derive(Debug)]
pub enum ValidationError {
    /// Attribute size must be either 1, 2, 3 or 4.
    InvalidAttributeSize,
    /// Data size is not correct.
    InvalidDataSize { expected: usize, actual: usize },
    /// Trying to add vertex of incorrect size.
    InvalidVertexSize,
    /// A duplicate for a descriptor was found.
    DuplicatedAttributeDescriptor,
    /// Duplicate shader locations were found.
    ConflictingShaderLocations,
}

impl VertexBuffer {
    pub fn new<T: Copy>(
        vertex_count: usize,
        layout: &[VertexAttributeDescriptor],
        mut data: Vec<T>,
    ) -> Result<Self, ValidationError> {
        let length = data.len() * std::mem::size_of::<T>();
        let capacity = data.capacity() * std::mem::size_of::<T>();

        let bytes =
            unsafe { Vec::<u8>::from_raw_parts(data.as_mut_ptr() as *mut u8, length, capacity) };

        std::mem::forget(data);

        // Validate for duplicates and invalid layout.
        for descriptor in layout {
            for other_descriptor in layout {
                if !std::ptr::eq(descriptor, other_descriptor) {
                    if descriptor.kind == other_descriptor.kind {
                        return Err(ValidationError::DuplicatedAttributeDescriptor);
                    } else if descriptor.shader_location == other_descriptor.shader_location {
                        return Err(ValidationError::ConflictingShaderLocations);
                    }
                }
            }
        }

        let mut dense_layout = Vec::new();

        // Validate everything as much as possible and calculate vertex size.
        let mut sparse_layout = [None; VertexAttributeKind::Count as usize];
        let mut vertex_size_bytes = 0u8;
        for attribute in layout.iter() {
            if attribute.size < 1 || attribute.size > 4 {
                return Err(ValidationError::InvalidAttributeSize);
            }

            let vertex_attribute = VertexAttribute {
                kind: attribute.kind,
                component_type: attribute.component_type,
                size: attribute.size,
                divisor: attribute.divisor,
                offset: vertex_size_bytes,
                shader_location: attribute.shader_location,
            };

            dense_layout.push(vertex_attribute);

            // Map dense to sparse layout to increase performance.
            sparse_layout[attribute.kind as usize] = Some(vertex_attribute);

            vertex_size_bytes += attribute.size * attribute.component_type.size();
        }

        let expected_data_size = vertex_count * vertex_size_bytes as usize;
        if expected_data_size != bytes.len() {
            return Err(ValidationError::InvalidDataSize {
                expected: expected_data_size,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            vertex_size: vertex_size_bytes,
            vertex_count: vertex_count as u32,
            data: bytes,
            sparse_layout,
            dense_layout,
        })
    }

    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }

    /// Tries to append a vertex to the buffer.
    ///
    /// # Safety and validation
    ///
    /// This method accepts any type that has appropriate size, the size must be equal
    /// with the size defined by layout. The Copy trait bound is required to ensure that
    /// the type does not have any custom destructors.
    pub fn push_vertex<T: Copy>(&mut self, vertex: &T) -> Result<(), ValidationError> {
        if std::mem::size_of::<T>() == self.vertex_size as usize {
            self.data
                .extend_from_slice(unsafe { value_as_u8_slice(vertex) });
            self.vertex_count += 1;
            Ok(())
        } else {
            Err(ValidationError::InvalidVertexSize)
        }
    }

    pub fn has_attribute(&self, kind: VertexAttributeKind) -> bool {
        self.sparse_layout[kind as usize].is_some()
    }

    pub fn layout(&self) -> &[VertexAttribute] {
        &self.dense_layout
    }

    /// Removes last vertex from the buffer.
    pub fn remove_last_vertex(&mut self) {
        self.data
            .drain((self.data.len() - self.vertex_size as usize)..);
        self.vertex_count -= 1;
    }

    /// Copies data of last vertex from the buffer to an instance of variable of a type.
    ///
    /// # Safety and validation
    ///
    /// This method accepts any type that has appropriate size, the size must be equal
    /// with the size defined by layout. The Copy trait bound is required to ensure that
    /// the type does not have any custom destructors.
    pub fn pop_vertex<T: Copy>(&mut self) -> Result<T, ValidationError> {
        if std::mem::size_of::<T>() == self.vertex_size as usize
            && self.data.len() >= self.vertex_size as usize
        {
            unsafe {
                let mut v = MaybeUninit::<T>::uninit();
                std::ptr::copy_nonoverlapping(
                    self.data
                        .as_ptr()
                        .add(self.data.len() - self.vertex_size as usize),
                    v.as_mut_ptr() as *mut u8,
                    self.vertex_size as usize,
                );
                self.data
                    .drain((self.data.len() - self.vertex_size as usize)..);
                self.vertex_count -= 1;
                Ok(v.assume_init())
            }
        } else {
            Err(ValidationError::InvalidVertexSize)
        }
    }

    pub fn cast_data_ref<T: Copy>(&self) -> Result<&[T], ValidationError> {
        if std::mem::size_of::<T>() == self.vertex_size as usize {
            Ok(unsafe {
                std::slice::from_raw_parts(
                    self.data.as_ptr() as *const T,
                    self.data.len() / std::mem::size_of::<T>(),
                )
            })
        } else {
            Err(ValidationError::InvalidVertexSize)
        }
    }

    pub fn cast_data_mut<T: Copy>(&mut self) -> Result<&mut [T], ValidationError> {
        if std::mem::size_of::<T>() == self.vertex_size as usize {
            Ok(unsafe {
                std::slice::from_raw_parts_mut(
                    self.data.as_mut_ptr() as *const T as *mut T,
                    self.data.len() / std::mem::size_of::<T>(),
                )
            })
        } else {
            Err(ValidationError::InvalidVertexSize)
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = VertexViewRef<'_>> + '_ {
        VertexViewRefIterator {
            data: &self.data,
            offset: 0,
            end: self.vertex_size as usize * self.vertex_count as usize,
            vertex_size: self.vertex_size,
            sparse_layout: &self.sparse_layout,
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = VertexViewMut<'_>> + '_ {
        unsafe {
            VertexViewMutIterator {
                ptr: self.data.as_mut_ptr(),
                end: self
                    .data
                    .as_mut_ptr()
                    .add(self.vertex_size as usize * self.vertex_count as usize),
                vertex_size: self.vertex_size,
                sparse_layout: &self.sparse_layout,
                marker: PhantomData,
            }
        }
    }

    pub fn get(&self, n: usize) -> Option<VertexViewRef<'_>> {
        let offset = n * self.vertex_size as usize;
        if offset < self.data.len() {
            Some(VertexViewRef {
                vertex_data: &self.data[offset..(offset + self.vertex_size as usize)],
                sparse_layout: &self.sparse_layout,
            })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, n: usize) -> Option<VertexViewMut<'_>> {
        let offset = n * self.vertex_size as usize;
        if offset < self.data.len() {
            Some(VertexViewMut {
                vertex_data: &mut self.data[offset..(offset + self.vertex_size as usize)],
                sparse_layout: &self.sparse_layout,
            })
        } else {
            None
        }
    }

    pub fn duplicate(&mut self, n: usize) {
        // Vertex cannot be larger than 256 bytes, so having temporary array of
        // such size is ok.
        let mut temp = ArrayVec::<u8, 256>::new();
        temp.try_extend_from_slice(
            &self.data[(n * self.vertex_size as usize)..((n + 1) * self.vertex_size as usize)],
        )
        .unwrap();
        self.data.extend_from_slice(temp.as_slice());
        self.vertex_count += 1;
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn vertex_size(&self) -> u8 {
        self.vertex_size
    }

    pub fn add_attribute<T: Copy>(
        &mut self,
        descriptor: VertexAttributeDescriptor,
        fill_value: T,
    ) -> Result<(), ValidationError> {
        if self.sparse_layout[descriptor.kind as usize].is_some() {
            return Err(ValidationError::DuplicatedAttributeDescriptor);
        } else {
            let vertex_attribute = VertexAttribute {
                kind: descriptor.kind,
                component_type: descriptor.component_type,
                size: descriptor.size,
                divisor: descriptor.divisor,
                offset: self.vertex_size,
                shader_location: descriptor.shader_location,
            };
            self.sparse_layout[descriptor.kind as usize] = Some(vertex_attribute);
            self.dense_layout.push(vertex_attribute);

            let mut new_data = Vec::new();

            for chunk in self.data.chunks_exact(self.vertex_size as usize) {
                let mut temp = ArrayVec::<u8, 256>::new();
                temp.try_extend_from_slice(chunk).unwrap();
                temp.try_extend_from_slice(unsafe { value_as_u8_slice(&fill_value) })
                    .unwrap();
                new_data.extend_from_slice(&temp);
            }

            self.data = new_data;

            self.vertex_size += std::mem::size_of::<T>() as u8;

            Ok(())
        }
    }

    pub fn find_free_shader_location(&self) -> u8 {
        let mut location = None;
        for attribute in self.dense_layout.chunks_exact(2) {
            let left = &attribute[0];
            let right = &attribute[1];

            if (left.shader_location as i32 - right.shader_location as i32).abs() > 1 {
                // We have a gap, use some value from it.
                let origin = left.shader_location.min(right.shader_location);
                location = Some(origin + 1);
                break;
            }
        }

        location.unwrap_or_else(|| {
            self.dense_layout
                .last()
                .map(|a| a.shader_location)
                .unwrap_or(0)
                + 1
        })
    }
}

struct VertexViewRefIterator<'a> {
    data: &'a [u8],
    sparse_layout: &'a [Option<VertexAttribute>],
    offset: usize,
    end: usize,
    vertex_size: u8,
}

impl<'a> Iterator for VertexViewRefIterator<'a> {
    type Item = VertexViewRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.end {
            None
        } else {
            let view = VertexViewRef {
                vertex_data: &self.data[self.offset..(self.offset + self.vertex_size as usize)],
                sparse_layout: self.sparse_layout,
            };
            self.offset += self.vertex_size as usize;
            Some(view)
        }
    }
}

struct VertexViewMutIterator<'a> {
    ptr: *mut u8,
    sparse_layout: &'a [Option<VertexAttribute>],
    end: *mut u8,
    vertex_size: u8,
    marker: PhantomData<&'a mut u8>,
}

impl<'a> Iterator for VertexViewMutIterator<'a> {
    type Item = VertexViewMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr >= self.end {
            None
        } else {
            unsafe {
                let data = std::slice::from_raw_parts_mut(self.ptr, self.vertex_size as usize);
                let view = VertexViewMut {
                    vertex_data: data,
                    sparse_layout: self.sparse_layout,
                };
                self.ptr = self.ptr.add(self.vertex_size as usize);
                Some(view)
            }
        }
    }
}

#[derive(Debug)]
pub struct VertexViewRef<'a> {
    vertex_data: &'a [u8],
    sparse_layout: &'a [Option<VertexAttribute>],
}

impl<'a> PartialEq for VertexViewRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_data == other.vertex_data
    }
}

#[derive(Debug)]
pub struct VertexViewMut<'a> {
    vertex_data: &'a mut [u8],
    sparse_layout: &'a [Option<VertexAttribute>],
}

impl<'a> PartialEq for VertexViewMut<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.vertex_data == other.vertex_data
    }
}

#[derive(Debug)]
pub enum VertexFetchError {
    NoSuchAttribute(VertexAttributeKind),
    Io(std::io::Error),
}

impl From<std::io::Error> for VertexFetchError {
    fn from(e: Error) -> Self {
        Self::Io(e)
    }
}

pub trait VertexReadTrait {
    fn data_layout_ref(&self) -> (&[u8], &[Option<VertexAttribute>]);

    #[inline(always)]
    fn read_2_f32(&self, attribute: VertexAttributeKind) -> Result<Vector2<f32>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            let x = (&data[(attribute.offset as usize)..]).read_f32::<LittleEndian>()?;
            let y = (&data[(attribute.offset as usize + 4)..]).read_f32::<LittleEndian>()?;
            Ok(Vector2::new(x, y))
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }

    #[inline(always)]
    fn read_3_f32(&self, attribute: VertexAttributeKind) -> Result<Vector3<f32>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            let x = (&data[(attribute.offset as usize)..]).read_f32::<LittleEndian>()?;
            let y = (&data[(attribute.offset as usize + 4)..]).read_f32::<LittleEndian>()?;
            let z = (&data[(attribute.offset as usize + 8)..]).read_f32::<LittleEndian>()?;
            Ok(Vector3::new(x, y, z))
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }

    #[inline(always)]
    fn read_4_f32(&self, attribute: VertexAttributeKind) -> Result<Vector4<f32>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            let x = (&data[(attribute.offset as usize)..]).read_f32::<LittleEndian>()?;
            let y = (&data[(attribute.offset as usize + 4)..]).read_f32::<LittleEndian>()?;
            let z = (&data[(attribute.offset as usize + 8)..]).read_f32::<LittleEndian>()?;
            let w = (&data[(attribute.offset as usize + 12)..]).read_f32::<LittleEndian>()?;
            Ok(Vector4::new(x, y, z, w))
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }

    #[inline(always)]
    fn read_4_u8(&self, attribute: VertexAttributeKind) -> Result<Vector4<u8>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            let offset = attribute.offset as usize;
            let x = data[offset];
            let y = data[offset + 1];
            let z = data[offset + 2];
            let w = data[offset + 3];
            Ok(Vector4::new(x, y, z, w))
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }
}

impl<'a> VertexReadTrait for VertexViewRef<'a> {
    fn data_layout_ref(&self) -> (&[u8], &[Option<VertexAttribute>]) {
        (self.vertex_data, self.sparse_layout)
    }
}

pub trait VertexWriteTrait: VertexReadTrait {
    fn data_layout_mut(&mut self) -> (&mut [u8], &[Option<VertexAttribute>]);

    fn write_2_f32(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector2<f32>,
    ) -> Result<(), VertexFetchError>;

    fn write_3_f32(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector3<f32>,
    ) -> Result<(), VertexFetchError>;

    fn write_4_f32(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector4<f32>,
    ) -> Result<(), VertexFetchError>;

    fn write_4_u8(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector4<u8>,
    ) -> Result<(), VertexFetchError>;
}

impl<'a> VertexReadTrait for VertexViewMut<'a> {
    fn data_layout_ref(&self) -> (&[u8], &[Option<VertexAttribute>]) {
        (self.vertex_data, self.sparse_layout)
    }
}

impl<'a> VertexWriteTrait for VertexViewMut<'a> {
    fn data_layout_mut(&mut self) -> (&mut [u8], &[Option<VertexAttribute>]) {
        (self.vertex_data, self.sparse_layout)
    }

    fn write_2_f32(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector2<f32>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            (&mut data[(attribute.offset as usize)..]).write_f32::<LittleEndian>(value.x)?;
            (&mut data[(attribute.offset as usize + 4)..]).write_f32::<LittleEndian>(value.y)?;
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }

    fn write_3_f32(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector3<f32>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            (&mut data[(attribute.offset as usize)..]).write_f32::<LittleEndian>(value.x)?;
            (&mut data[(attribute.offset as usize + 4)..]).write_f32::<LittleEndian>(value.y)?;
            (&mut data[(attribute.offset as usize + 8)..]).write_f32::<LittleEndian>(value.z)?;
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }

    fn write_4_f32(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector4<f32>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            (&mut data[(attribute.offset as usize)..]).write_f32::<LittleEndian>(value.x)?;
            (&mut data[(attribute.offset as usize + 4)..]).write_f32::<LittleEndian>(value.y)?;
            (&mut data[(attribute.offset as usize + 8)..]).write_f32::<LittleEndian>(value.z)?;
            (&mut data[(attribute.offset as usize + 12)..]).write_f32::<LittleEndian>(value.w)?;
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }

    fn write_4_u8(
        &mut self,
        attribute: VertexAttributeKind,
        value: Vector4<u8>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(attribute as usize).unwrap() {
            data[attribute.offset as usize] = value.x;
            data[(attribute.offset + 1) as usize] = value.y;
            data[(attribute.offset + 2) as usize] = value.z;
            data[(attribute.offset + 3) as usize] = value.w;
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(attribute))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::algebra::{Vector2, Vector3, Vector4},
        scene::mesh::buffer::{
            VertexAttributeDataKind, VertexAttributeDescriptor, VertexAttributeKind, VertexBuffer,
            VertexReadTrait,
        },
    };

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct Vertex {
        position: Vector3<f32>,
        tex_coord: Vector2<f32>,
        second_tex_coord: Vector2<f32>,
        normal: Vector3<f32>,
        tangent: Vector4<f32>,
        bone_weights: Vector4<f32>,
        bone_indices: Vector4<u8>,
    }

    impl Vertex {
        pub fn layout() -> &'static [VertexAttributeDescriptor] {
            static LAYOUT: [VertexAttributeDescriptor; 7] = [
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::Position,
                    component_type: VertexAttributeDataKind::F32,
                    size: 3,
                    divisor: 0,
                    shader_location: 0,
                },
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::TexCoord0,
                    component_type: VertexAttributeDataKind::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 1,
                },
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::TexCoord1,
                    component_type: VertexAttributeDataKind::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 2,
                },
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::Normal,
                    component_type: VertexAttributeDataKind::F32,
                    size: 3,
                    divisor: 0,
                    shader_location: 3,
                },
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::Tangent,
                    component_type: VertexAttributeDataKind::F32,
                    size: 4,
                    divisor: 0,
                    shader_location: 4,
                },
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::BoneWeight,
                    component_type: VertexAttributeDataKind::F32,
                    size: 4,
                    divisor: 0,
                    shader_location: 5,
                },
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::BoneIndices,
                    component_type: VertexAttributeDataKind::U8,
                    size: 4,
                    divisor: 0,
                    shader_location: 6,
                },
            ];

            &LAYOUT
        }
    }

    const VERTICES: [Vertex; 3] = [
        Vertex {
            position: Vector3::new(1.0, 2.0, 3.0),
            tex_coord: Vector2::new(0.0, 1.0),
            second_tex_coord: Vector2::new(1.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            tangent: Vector4::new(1.0, 0.0, 0.0, 1.0),
            bone_weights: Vector4::new(0.25, 0.25, 0.25, 0.25),
            bone_indices: Vector4::new(1, 2, 3, 4),
        },
        Vertex {
            position: Vector3::new(1.0, 2.0, 3.0),
            tex_coord: Vector2::new(0.0, 1.0),
            second_tex_coord: Vector2::new(1.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            tangent: Vector4::new(1.0, 0.0, 0.0, 1.0),
            bone_weights: Vector4::new(0.25, 0.25, 0.25, 0.25),
            bone_indices: Vector4::new(1, 2, 3, 4),
        },
        Vertex {
            position: Vector3::new(1.0, 2.0, 3.0),
            tex_coord: Vector2::new(0.0, 1.0),
            second_tex_coord: Vector2::new(1.0, 0.0),
            normal: Vector3::new(0.0, 1.0, 0.0),
            tangent: Vector4::new(1.0, 0.0, 0.0, 1.0),
            bone_weights: Vector4::new(0.25, 0.25, 0.25, 0.25),
            bone_indices: Vector4::new(1, 2, 3, 4),
        },
    ];

    fn test_view_original_equal<T: VertexReadTrait>(view: T, original: &Vertex) {
        assert_eq!(
            view.read_3_f32(VertexAttributeKind::Position).unwrap(),
            original.position
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeKind::TexCoord0).unwrap(),
            original.tex_coord
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeKind::TexCoord1).unwrap(),
            original.second_tex_coord
        );
        assert_eq!(
            view.read_3_f32(VertexAttributeKind::Normal).unwrap(),
            original.normal
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeKind::Tangent).unwrap(),
            original.tangent
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeKind::BoneWeight).unwrap(),
            original.bone_weights
        );
        assert_eq!(
            view.read_4_u8(VertexAttributeKind::BoneIndices).unwrap(),
            original.bone_indices
        );
    }

    fn create_test_buffer() -> VertexBuffer {
        VertexBuffer::new(VERTICES.len(), Vertex::layout(), VERTICES.to_vec()).unwrap()
    }

    #[test]
    fn test_iter() {
        let buffer = create_test_buffer();

        for (view, original) in buffer.iter().zip(VERTICES.iter()) {
            test_view_original_equal(view, original);
        }
    }

    #[test]
    fn test_iter_mut() {
        let mut buffer = create_test_buffer();

        for (view, original) in buffer.iter_mut().zip(VERTICES.iter()) {
            test_view_original_equal(view, original);
        }
    }

    #[test]
    fn test_vertex_duplication() {
        let mut buffer = create_test_buffer();

        buffer.duplicate(0);

        assert_eq!(buffer.vertex_count(), 4);
        assert_eq!(buffer.get(0).unwrap(), buffer.get(3).unwrap())
    }

    #[test]
    fn test_pop_vertex() {
        let mut buffer = create_test_buffer();

        let vertex = buffer.pop_vertex::<Vertex>().unwrap();

        assert_eq!(buffer.vertex_count(), 2);
        assert_eq!(vertex, VERTICES[2]);
    }

    #[test]
    fn test_remove_last_vertex() {
        let mut buffer = create_test_buffer();

        buffer.remove_last_vertex();

        assert_eq!(buffer.vertex_count(), 2);
    }

    #[test]
    fn test_add_attribute() {
        let mut buffer = create_test_buffer();

        let fill = Vector2::new(0.25, 0.75);
        let test_index = 1;

        buffer
            .add_attribute(
                VertexAttributeDescriptor {
                    kind: VertexAttributeKind::TexCoord2,
                    component_type: VertexAttributeDataKind::F32,
                    size: 2,
                    divisor: 0,
                },
                fill,
            )
            .unwrap();

        #[derive(Clone, Copy, PartialEq, Debug)]
        struct ExtendedVertex {
            position: Vector3<f32>,
            tex_coord: Vector2<f32>,
            second_tex_coord: Vector2<f32>,
            normal: Vector3<f32>,
            tangent: Vector4<f32>,
            bone_weights: Vector4<f32>,
            bone_indices: Vector4<u8>,
            third_tex_coord: Vector2<f32>, // NEW
        }

        let new_1 = ExtendedVertex {
            position: VERTICES[test_index].position,
            tex_coord: VERTICES[test_index].tex_coord,
            second_tex_coord: VERTICES[test_index].second_tex_coord,
            normal: VERTICES[test_index].normal,
            tangent: VERTICES[test_index].tangent,
            bone_weights: VERTICES[test_index].bone_weights,
            bone_indices: VERTICES[test_index].bone_indices,
            third_tex_coord: fill,
        };

        assert_eq!(
            buffer.vertex_size,
            std::mem::size_of::<ExtendedVertex>() as u8
        );
        let view = buffer.get(test_index).unwrap();
        assert_eq!(
            view.read_3_f32(VertexAttributeKind::Position).unwrap(),
            new_1.position
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeKind::TexCoord0).unwrap(),
            new_1.tex_coord
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeKind::TexCoord1).unwrap(),
            new_1.second_tex_coord
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeKind::TexCoord2).unwrap(),
            new_1.third_tex_coord
        );
        assert_eq!(
            view.read_3_f32(VertexAttributeKind::Normal).unwrap(),
            new_1.normal
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeKind::Tangent).unwrap(),
            new_1.tangent
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeKind::BoneWeight).unwrap(),
            new_1.bone_weights
        );
        assert_eq!(
            view.read_4_u8(VertexAttributeKind::BoneIndices).unwrap(),
            new_1.bone_indices
        );
    }
}
