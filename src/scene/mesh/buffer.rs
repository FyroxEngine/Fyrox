//! Vertex buffer with dynamic layout. See [`VertexBuffer`] docs for more info and usage examples.

use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        arrayvec::ArrayVec,
        byteorder::{ByteOrder, LittleEndian},
        futures::io::Error,
        math::TriangleDefinition,
        visitor::prelude::*,
    },
    utils::value_as_u8_slice,
};
use fxhash::FxHasher;
use std::{
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Index, IndexMut},
};

/// A common trait for all vertex types. **IMPORTANT:** Implementors **must** use `#[repr(C)]` attribute, otherwise the compiler
/// is free to reorder fields and you might get weird results, because definition order will be different from memory order! See
/// examples in [`VertexBuffer`] docs.
pub trait VertexTrait: Copy {
    /// Returns memory layout of the vertex. It basically tells a GPU how to interpret every byte range
    /// of your vertex type; which kind of information it holds.
    fn layout() -> &'static [VertexAttributeDescriptor];
}

/// Data type for a vertex attribute component.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Visit, Debug)]
#[repr(u8)]
pub enum VertexAttributeDataType {
    /// 32-bit floating-point.
    F32,
    /// 32-bit unsigned integer.
    U32,
    /// 16-bit unsigned integer.
    U16,
    /// 8-bit unsigned integer.
    U8,
}

impl Default for VertexAttributeDataType {
    fn default() -> Self {
        Self::F32
    }
}

impl VertexAttributeDataType {
    /// Returns size of data in bytes.
    pub fn size(self) -> u8 {
        match self {
            VertexAttributeDataType::F32 | VertexAttributeDataType::U32 => 4,
            VertexAttributeDataType::U16 => 2,
            VertexAttributeDataType::U8 => 1,
        }
    }
}

/// An usage for vertex attribute. It is a fixed set, but there are plenty
/// room for any custom data - it may be fit into `TexCoordN` attributes.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash, Visit, Debug)]
#[repr(u32)]
pub enum VertexAttributeUsage {
    /// Vertex position. Usually `Vector2<f32>` or `Vector3<f32>`.
    Position = 0,
    /// Vertex normal. Usually `Vector3<f32>`, more rare `Vector3<u16>` (F16).
    Normal = 1,
    /// Vertex tangent. Usually `Vector3<f32>`.
    Tangent = 2,
    /// First texture coordinates. Usually `Vector2<f32>`.
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
    /// Bone weights. Usually `Vector4<f32>`.
    BoneWeight = 11,
    /// Bone indices. Usually `Vector4<u8>`.
    BoneIndices = 12,
    /// Maximum amount of attribute kinds.
    Count,
}

impl Default for VertexAttributeUsage {
    fn default() -> Self {
        Self::Position
    }
}

/// Input vertex attribute descriptor used to construct layouts and feed vertex buffer.
#[derive(Debug)]
pub struct VertexAttributeDescriptor {
    /// Claimed usage of the attribute. It could be Position, Normal, etc.
    pub usage: VertexAttributeUsage,
    /// Data type of every component of the attribute. It could be F32, U32, U16, etc.
    pub data_type: VertexAttributeDataType,
    /// Size of attribute expressed in components. For example, for `Position` it could
    /// be 3 - which means there are 3 components in attribute of `data_type`.
    pub size: u8,
    /// Sets a "fetch rate" for vertex shader at which it will read vertex attribute:
    ///  0 - per vertex (default)
    ///  1 - per instance
    ///  2 - per 2 instances and so on.
    pub divisor: u8,
    /// Defines location of the attribute in a shader (`layout(location = x) attrib;`)
    pub shader_location: u8,
}

/// Vertex attribute is a simple "bridge" between raw data and its interpretation. In
/// other words it defines how to treat raw data in vertex shader.
#[derive(Visit, Copy, Clone, Default, Debug, Hash)]
pub struct VertexAttribute {
    /// Claimed usage of the attribute. It could be Position, Normal, etc.
    pub usage: VertexAttributeUsage,
    /// Data type of every component of the attribute. It could be F32, U32, U16, etc.
    pub data_type: VertexAttributeDataType,
    /// Size of attribute expressed in components. For example, for `Position` it could
    /// be 3 - which means there are 3 components in attribute of `data_type`.
    pub size: u8,
    /// Sets a "fetch rate" for vertex shader at which it will read vertex attribute:
    ///  0 - per vertex (default)
    ///  1 - per instance
    ///  2 - per 2 instances and so on.
    pub divisor: u8,
    /// Offset in bytes from beginning of the vertex.
    pub offset: u8,
    /// Defines location of the attribute in a shader (`layout(location = x) attrib;`)
    pub shader_location: u8,
}

/// Vertex buffer with dynamic layout. It is used to store multiple vertices of a single type, that implements [`VertexTrait`].
/// Different vertex types used to for efficient memory usage. For example, you could have a simple vertex with only position
/// expressed as Vector3 and it will be enough for simple cases, when only position is required. However, if you want to draw
/// a mesh with skeletal animation, that also supports texturing, lighting, you need to provide a lot more data (bone indices,
/// bone weights, normals, tangents, texture coordinates).
///
/// ## Examples
///
/// ```rust
/// # use fyrox::{
/// #     core::algebra::Vector3,
/// #     scene::mesh::buffer::{
/// #         VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage, VertexBuffer,
/// #         VertexTrait,
/// #     },
/// # };
/// #
/// #[derive(Copy, Clone)]
/// #[repr(C)]
/// struct MyVertex {
///     position: Vector3<f32>,
/// }
///
/// impl VertexTrait for MyVertex {
///     fn layout() -> &'static [VertexAttributeDescriptor] {
///         &[VertexAttributeDescriptor {
///             usage: VertexAttributeUsage::Position,
///             data_type: VertexAttributeDataType::F32,
///             size: 3,
///             divisor: 0,
///             shader_location: 0,
///         }]
///     }
/// }
///
/// fn create_triangle_vertex_buffer() -> VertexBuffer {
///     VertexBuffer::new(
///         3,
///         vec![
///             MyVertex {
///                 position: Vector3::new(0.0, 0.0, 0.0),
///             },
///             MyVertex {
///                 position: Vector3::new(0.0, 1.0, 0.0),
///             },
///             MyVertex {
///                 position: Vector3::new(1.0, 1.0, 0.0),
///             },
///         ],
///     )
///     .unwrap()
/// }  
/// ```
///
/// This example creates a simple vertex buffer that contains a single triangle with custom vertex format. The most important
/// part here is [`VertexTrait::layout`] implementation - it describes each "attribute" of your vertex, if your layout does not
/// match the actual content of the vertex (in terms of size in bytes), then vertex buffer cannot be created and [`VertexBuffer::new`]
/// will return [`None`].
///
/// The second, but not least important is `#[repr(C)]` attribute - it is mandatory for every vertex type, it forbids fields
/// reordering of you vertex structure and guarantees that they will have the same layout in memory as their declaration order.
///
/// ## Limitations
///
/// Vertex size cannot be more than 256 bytes, this limitation shouldn't be a problem because almost every GPU supports up to
/// 16 vertex attributes with 16 bytes of size each, which gives exactly 256 bytes.
#[derive(Clone, Visit, Default, Debug)]
pub struct VertexBuffer {
    dense_layout: Vec<VertexAttribute>,
    sparse_layout: [Option<VertexAttribute>; 13],
    vertex_size: u8,
    vertex_count: u32,
    data: Vec<u8>,
    data_hash: u64,
    #[visit(optional)]
    layout_hash: u64,
}

fn calculate_layout_hash(layout: &[VertexAttribute]) -> u64 {
    let mut hasher = FxHasher::default();
    layout.hash(&mut hasher);
    hasher.finish()
}

fn calculate_data_hash(data: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

/// See VertexBuffer::modify for more info.
pub struct VertexBufferRefMut<'a> {
    vertex_buffer: &'a mut VertexBuffer,
}

impl<'a> Drop for VertexBufferRefMut<'a> {
    fn drop(&mut self) {
        // Recalculate data hash.
        self.vertex_buffer.data_hash = calculate_data_hash(&self.vertex_buffer.data);
    }
}

impl<'a> Deref for VertexBufferRefMut<'a> {
    type Target = VertexBuffer;

    fn deref(&self) -> &Self::Target {
        self.vertex_buffer
    }
}

impl<'a> DerefMut for VertexBufferRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.vertex_buffer
    }
}

impl<'a> VertexBufferRefMut<'a> {
    /// Tries to append a vertex to the buffer.
    ///
    /// # Safety and validation
    ///
    /// This method accepts any type that has appropriate size, the size must be equal
    /// with the size defined by layout. The Copy trait bound is required to ensure that
    /// the type does not have any custom destructors.
    pub fn push_vertex<T>(&mut self, vertex: &T) -> Result<(), ValidationError>
    where
        T: VertexTrait,
    {
        if std::mem::size_of::<T>() == self.vertex_buffer.vertex_size as usize {
            self.vertex_buffer
                .data
                .extend_from_slice(value_as_u8_slice(vertex));
            self.vertex_buffer.vertex_count += 1;
            Ok(())
        } else {
            Err(ValidationError::InvalidVertexSize {
                expected: self.vertex_buffer.vertex_size,
                actual: std::mem::size_of::<T>() as u8,
            })
        }
    }

    /// Removes last vertex from the buffer.
    pub fn remove_last_vertex(&mut self) {
        self.vertex_buffer
            .data
            .drain((self.vertex_buffer.data.len() - self.vertex_buffer.vertex_size as usize)..);
        self.vertex_buffer.vertex_count -= 1;
    }

    /// Copies data of last vertex from the buffer to an instance of variable of a type.
    ///
    /// # Safety and validation
    ///
    /// This method accepts any type that has appropriate size, the size must be equal
    /// with the size defined by layout. The Copy trait bound is required to ensure that
    /// the type does not have any custom destructors.
    pub fn pop_vertex<T>(&mut self) -> Result<T, ValidationError>
    where
        T: VertexTrait,
    {
        if std::mem::size_of::<T>() == self.vertex_buffer.vertex_size as usize
            && self.vertex_buffer.data.len() >= self.vertex_buffer.vertex_size as usize
        {
            unsafe {
                let mut v = MaybeUninit::<T>::uninit();
                std::ptr::copy_nonoverlapping(
                    self.vertex_buffer.data.as_ptr().add(
                        self.vertex_buffer.data.len() - self.vertex_buffer.vertex_size as usize,
                    ),
                    v.as_mut_ptr() as *mut u8,
                    self.vertex_buffer.vertex_size as usize,
                );
                self.vertex_buffer.data.drain(
                    (self.vertex_buffer.data.len() - self.vertex_buffer.vertex_size as usize)..,
                );
                self.vertex_buffer.vertex_count -= 1;
                Ok(v.assume_init())
            }
        } else {
            Err(ValidationError::InvalidVertexSize {
                expected: self.vertex_buffer.vertex_size,
                actual: std::mem::size_of::<T>() as u8,
            })
        }
    }

    /// Tries to cast internal data buffer to a slice of given type. It may fail if
    /// size of type is not equal with claimed size (which is set by the layout).
    pub fn cast_data_mut<T>(&mut self) -> Result<&mut [T], ValidationError>
    where
        T: VertexTrait,
    {
        if std::mem::size_of::<T>() == self.vertex_buffer.vertex_size as usize {
            Ok(unsafe {
                std::slice::from_raw_parts_mut(
                    self.vertex_buffer.data.as_mut_ptr() as *const T as *mut T,
                    self.vertex_buffer.data.len() / std::mem::size_of::<T>(),
                )
            })
        } else {
            Err(ValidationError::InvalidVertexSize {
                expected: self.vertex_buffer.vertex_size,
                actual: std::mem::size_of::<T>() as u8,
            })
        }
    }

    /// Creates iterator that emits read/write accessors for vertices.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = VertexViewMut<'_>> + '_ {
        unsafe {
            VertexViewMutIterator {
                ptr: self.vertex_buffer.data.as_mut_ptr(),
                end: self.data.as_mut_ptr().add(
                    self.vertex_buffer.vertex_size as usize
                        * self.vertex_buffer.vertex_count as usize,
                ),
                vertex_size: self.vertex_buffer.vertex_size,
                sparse_layout: &self.vertex_buffer.sparse_layout,
                marker: PhantomData,
            }
        }
    }

    /// Returns a read/write accessor of n-th vertex.
    pub fn get_mut(&mut self, n: usize) -> Option<VertexViewMut<'_>> {
        let offset = n * self.vertex_buffer.vertex_size as usize;
        if offset < self.vertex_buffer.data.len() {
            Some(VertexViewMut {
                vertex_data: &mut self.vertex_buffer.data
                    [offset..(offset + self.vertex_buffer.vertex_size as usize)],
                sparse_layout: &self.vertex_buffer.sparse_layout,
            })
        } else {
            None
        }
    }

    /// Duplicates n-th vertex and puts it at the back of the buffer.
    pub fn duplicate(&mut self, n: usize) {
        // Vertex cannot be larger than 256 bytes, so having temporary array of
        // such size is ok.
        let mut temp = ArrayVec::<u8, 256>::new();
        temp.try_extend_from_slice(
            &self.vertex_buffer.data[(n * self.vertex_buffer.vertex_size as usize)
                ..((n + 1) * self.vertex_buffer.vertex_size as usize)],
        )
        .unwrap();
        self.vertex_buffer.data.extend_from_slice(temp.as_slice());
        self.vertex_buffer.vertex_count += 1;
    }

    /// Adds new attribute at the end of layout, reorganizes internal data storage to be
    /// able to contain new attribute. Default value of the new attribute in the buffer
    /// becomes `fill_value`. Graphically this could be represented like so:
    ///
    /// Add secondary texture coordinates:
    ///  Before: P1_N1_TC1_P2_N2_TC2...
    ///  After: P1_N1_TC1_TC2(fill_value)_P2_N2_TC2_TC2(fill_value)...
    pub fn add_attribute<T>(
        &mut self,
        descriptor: VertexAttributeDescriptor,
        fill_value: T,
    ) -> Result<(), ValidationError>
    where
        T: Copy,
    {
        if self.vertex_buffer.sparse_layout[descriptor.usage as usize].is_some() {
            Err(ValidationError::DuplicatedAttributeDescriptor)
        } else {
            let vertex_attribute = VertexAttribute {
                usage: descriptor.usage,
                data_type: descriptor.data_type,
                size: descriptor.size,
                divisor: descriptor.divisor,
                offset: self.vertex_buffer.vertex_size,
                shader_location: descriptor.shader_location,
            };
            self.vertex_buffer.sparse_layout[descriptor.usage as usize] = Some(vertex_attribute);
            self.vertex_buffer.dense_layout.push(vertex_attribute);

            self.layout_hash = calculate_layout_hash(&self.vertex_buffer.dense_layout);

            let mut new_data = Vec::new();

            for chunk in self
                .vertex_buffer
                .data
                .chunks_exact(self.vertex_buffer.vertex_size as usize)
            {
                let mut temp = ArrayVec::<u8, 256>::new();
                temp.try_extend_from_slice(chunk).unwrap();
                temp.try_extend_from_slice(value_as_u8_slice(&fill_value))
                    .unwrap();
                new_data.extend_from_slice(&temp);
            }

            self.vertex_buffer.data = new_data;

            self.vertex_buffer.vertex_size += std::mem::size_of::<T>() as u8;

            Ok(())
        }
    }

    /// Clears the buffer making it empty.
    pub fn clear(&mut self) {
        self.data.clear();
        self.vertex_count = 0;
    }
}

/// An error that may occur during input data and layout validation.
#[derive(Debug)]
pub enum ValidationError {
    /// Attribute size must be either 1, 2, 3 or 4.
    InvalidAttributeSize(usize),

    /// Data size is not correct.
    InvalidDataSize {
        /// Expected data size in bytes.
        expected: usize,
        /// Actual data size in bytes.
        actual: usize,
    },

    /// Trying to add vertex of incorrect size.
    InvalidVertexSize {
        /// Expected vertex size.
        expected: u8,
        /// Actual vertex size.
        actual: u8,
    },

    /// A duplicate of a descriptor was found.
    DuplicatedAttributeDescriptor,

    /// Duplicate shader locations were found.
    ConflictingShaderLocations(usize),
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidAttributeSize(v) => {
                write!(f, "Invalid attribute size {v}. Must be either 1, 2, 3 or 4")
            }
            ValidationError::InvalidDataSize { expected, actual } => {
                write!(f, "Invalid data size. Expected {expected}, got {actual}.")
            }
            ValidationError::InvalidVertexSize { expected, actual } => {
                write!(f, "Invalid vertex size. Expected {expected}, got {actual}.",)
            }
            ValidationError::DuplicatedAttributeDescriptor => {
                write!(f, "A duplicate of a descriptor was found.")
            }
            ValidationError::ConflictingShaderLocations(v) => {
                write!(f, "Duplicate shader locations were found {v}.")
            }
        }
    }
}

impl VertexBuffer {
    /// Creates new vertex buffer from provided data and with the given layout of the vertex type `T`.
    pub fn new<T>(vertex_count: usize, data: Vec<T>) -> Result<Self, ValidationError>
    where
        T: VertexTrait,
    {
        let mut data = std::mem::ManuallyDrop::new(data);
        let length = data.len() * std::mem::size_of::<T>();
        let capacity = data.capacity() * std::mem::size_of::<T>();

        let bytes = unsafe { Vec::<u8>::from_raw_parts(data.as_mut_ptr() as *mut u8, length, capacity) };

        let layout = T::layout();

        // Validate for duplicates and invalid layout.
        for descriptor in layout {
            for other_descriptor in layout {
                if !std::ptr::eq(descriptor, other_descriptor) {
                    if descriptor.usage == other_descriptor.usage {
                        return Err(ValidationError::DuplicatedAttributeDescriptor);
                    } else if descriptor.shader_location == other_descriptor.shader_location {
                        return Err(ValidationError::ConflictingShaderLocations(
                            descriptor.shader_location as usize,
                        ));
                    }
                }
            }
        }

        let mut dense_layout = Vec::new();

        // Validate everything as much as possible and calculate vertex size.
        let mut sparse_layout = [None; VertexAttributeUsage::Count as usize];
        let mut vertex_size_bytes = 0u8;
        for attribute in layout.iter() {
            if attribute.size < 1 || attribute.size > 4 {
                return Err(ValidationError::InvalidAttributeSize(
                    attribute.size as usize,
                ));
            }

            let vertex_attribute = VertexAttribute {
                usage: attribute.usage,
                data_type: attribute.data_type,
                size: attribute.size,
                divisor: attribute.divisor,
                offset: vertex_size_bytes,
                shader_location: attribute.shader_location,
            };

            dense_layout.push(vertex_attribute);

            // Map dense to sparse layout to increase performance.
            sparse_layout[attribute.usage as usize] = Some(vertex_attribute);

            vertex_size_bytes += attribute.size * attribute.data_type.size();
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
            data_hash: calculate_data_hash(&bytes),
            data: bytes,
            layout_hash: calculate_layout_hash(&dense_layout),
            sparse_layout,
            dense_layout,
        })
    }

    /// Returns a reference to underlying data buffer slice.
    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }

    /// Returns true if buffer does not contain any vertex, false - otherwise.
    pub fn is_empty(&self) -> bool {
        self.vertex_count == 0
    }

    /// Returns cached data hash. Cached value is guaranteed to be in actual state.
    pub fn data_hash(&self) -> u64 {
        self.data_hash
    }

    /// Returns hash of vertex buffer layout. Cached value is guaranteed to be in actual state.
    /// The hash could be used to check if the layout has changed.
    pub fn layout_hash(&self) -> u64 {
        self.layout_hash
    }

    /// Provides mutable access to content of the buffer.
    ///
    /// # Performance
    ///
    /// This method returns special structure which has custom destructor that
    /// calculates hash of the data once modification is over. You **must** hold
    /// this structure as long as possible while modifying contents of the buffer.
    /// Do **not** even try to do this:
    ///
    /// ```no_run
    /// use fyrox::{
    ///     scene::mesh::buffer::{VertexBuffer, VertexWriteTrait, VertexAttributeUsage},
    ///     core::algebra::Vector3
    /// };
    /// fn do_something(buffer: &mut VertexBuffer) {
    ///     for i in 0..buffer.vertex_count() {
    ///         buffer
    ///             .modify() // Doing this in a loop will cause HUGE performance issues!
    ///             .get_mut(i as usize)
    ///             .unwrap()
    ///             .write_3_f32(VertexAttributeUsage::Position, Vector3::<f32>::default())
    ///             .unwrap();
    ///     }
    /// }
    /// ```
    ///
    /// Instead do this:
    ///
    /// ```no_run
    /// use fyrox::{
    ///     scene::mesh::buffer::{VertexBuffer, VertexWriteTrait, VertexAttributeUsage},
    ///     core::algebra::Vector3
    /// };
    /// fn do_something(buffer: &mut VertexBuffer) {
    ///     let mut buffer_modifier = buffer.modify();
    ///     for mut vertex in buffer_modifier.iter_mut() {
    ///         vertex
    ///             .write_3_f32(VertexAttributeUsage::Position, Vector3::<f32>::default())
    ///             .unwrap();
    ///     }
    /// }
    /// ```
    ///
    /// Why do we even need such complications? It is used for lazy hash calculation which is
    /// used for automatic upload of contents to GPU in case if content has changed.
    pub fn modify(&mut self) -> VertexBufferRefMut<'_> {
        VertexBufferRefMut {
            vertex_buffer: self,
        }
    }

    /// Checks if an attribute of `usage` exists.
    pub fn has_attribute(&self, usage: VertexAttributeUsage) -> bool {
        self.sparse_layout[usage as usize].is_some()
    }

    /// Returns vertex buffer layout.
    pub fn layout(&self) -> &[VertexAttribute] {
        &self.dense_layout
    }

    /// Tries to cast internal data buffer to a slice of given type. It may fail if
    /// size of type is not equal with claimed size (which is set by the layout).
    pub fn cast_data_ref<T>(&self) -> Result<&[T], ValidationError>
    where
        T: VertexTrait,
    {
        if std::mem::size_of::<T>() == self.vertex_size as usize {
            Ok(unsafe {
                std::slice::from_raw_parts(
                    self.data.as_ptr() as *const T,
                    self.data.len() / std::mem::size_of::<T>(),
                )
            })
        } else {
            Err(ValidationError::InvalidVertexSize {
                expected: self.vertex_size,
                actual: std::mem::size_of::<T>() as u8,
            })
        }
    }

    /// Creates iterator that emits read accessors for vertices.
    pub fn iter(&self) -> impl Iterator<Item = VertexViewRef<'_>> + '_ {
        VertexViewRefIterator {
            data: &self.data,
            offset: 0,
            end: self.vertex_size as usize * self.vertex_count as usize,
            vertex_size: self.vertex_size,
            sparse_layout: &self.sparse_layout,
        }
    }

    /// Returns a read accessor of n-th vertex.
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

    /// Returns exact amount of vertices in the buffer.
    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    /// Return vertex size of the buffer.
    pub fn vertex_size(&self) -> u8 {
        self.vertex_size
    }

    /// Finds free location for an attribute in the layout.
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

/// Read accessor for a vertex with some layout.
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

/// Read/write accessor for a vertex with some layout.
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

/// An error that may occur during fetching using vertex read/write accessor.
#[derive(Debug)]
pub enum VertexFetchError {
    /// Trying to read/write non-existent attribute.
    NoSuchAttribute(VertexAttributeUsage),
    /// IO error.
    Io(std::io::Error),
}

impl std::error::Error for VertexFetchError {}

impl Display for VertexFetchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VertexFetchError::NoSuchAttribute(v) => {
                write!(f, "No attribute with such usage: {v:?}")
            }
            VertexFetchError::Io(v) => {
                write!(f, "An i/o error has occurred {v:?}")
            }
        }
    }
}

impl From<std::io::Error> for VertexFetchError {
    fn from(e: Error) -> Self {
        Self::Io(e)
    }
}

/// A trait for read-only vertex data accessor.
pub trait VertexReadTrait {
    #[doc(hidden)]
    fn data_layout_ref(&self) -> (&[u8], &[Option<VertexAttribute>]);

    /// Tries to read an attribute with given usage as a pair of two f32.
    #[inline(always)]
    fn read_2_f32(&self, usage: VertexAttributeUsage) -> Result<Vector2<f32>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            let x = LittleEndian::read_f32(&data[(attribute.offset as usize)..]);
            let y = LittleEndian::read_f32(&data[(attribute.offset as usize + 4)..]);
            Ok(Vector2::new(x, y))
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }

    /// Tries to read an attribute with given usage as a pair of three f32.
    #[inline(always)]
    fn read_3_f32(&self, usage: VertexAttributeUsage) -> Result<Vector3<f32>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            let x = LittleEndian::read_f32(&data[(attribute.offset as usize)..]);
            let y = LittleEndian::read_f32(&data[(attribute.offset as usize + 4)..]);
            let z = LittleEndian::read_f32(&data[(attribute.offset as usize + 8)..]);
            Ok(Vector3::new(x, y, z))
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }

    /// Tries to read an attribute with given usage as a pair of four f32.
    #[inline(always)]
    fn read_4_f32(&self, usage: VertexAttributeUsage) -> Result<Vector4<f32>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            let x = LittleEndian::read_f32(&data[(attribute.offset as usize)..]);
            let y = LittleEndian::read_f32(&data[(attribute.offset as usize + 4)..]);
            let z = LittleEndian::read_f32(&data[(attribute.offset as usize + 8)..]);
            let w = LittleEndian::read_f32(&data[(attribute.offset as usize + 12)..]);
            Ok(Vector4::new(x, y, z, w))
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }

    /// Tries to read an attribute with given usage as a pair of four u8.
    #[inline(always)]
    fn read_4_u8(&self, usage: VertexAttributeUsage) -> Result<Vector4<u8>, VertexFetchError> {
        let (data, layout) = self.data_layout_ref();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            let offset = attribute.offset as usize;
            let x = data[offset];
            let y = data[offset + 1];
            let z = data[offset + 2];
            let w = data[offset + 3];
            Ok(Vector4::new(x, y, z, w))
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }
}

impl<'a> VertexReadTrait for VertexViewRef<'a> {
    fn data_layout_ref(&self) -> (&[u8], &[Option<VertexAttribute>]) {
        (self.vertex_data, self.sparse_layout)
    }
}

/// A trait for read/write vertex data accessor.
pub trait VertexWriteTrait: VertexReadTrait {
    #[doc(hidden)]
    fn data_layout_mut(&mut self) -> (&mut [u8], &[Option<VertexAttribute>]);

    /// Tries to write an attribute with given usage as a pair of two f32.
    fn write_2_f32(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector2<f32>,
    ) -> Result<(), VertexFetchError>;

    /// Tries to write an attribute with given usage as a pair of three f32.
    fn write_3_f32(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector3<f32>,
    ) -> Result<(), VertexFetchError>;

    /// Tries to write an attribute with given usage as a pair of four f32.
    fn write_4_f32(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector4<f32>,
    ) -> Result<(), VertexFetchError>;

    /// Tries to write an attribute with given usage as a pair of four u8.
    fn write_4_u8(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector4<u8>,
    ) -> Result<(), VertexFetchError>;
}

impl<'a> VertexReadTrait for VertexViewMut<'a> {
    fn data_layout_ref(&self) -> (&[u8], &[Option<VertexAttribute>]) {
        (self.vertex_data, self.sparse_layout)
    }
}

impl<'a> VertexWriteTrait for VertexViewMut<'a> {
    #[inline(always)]
    fn data_layout_mut(&mut self) -> (&mut [u8], &[Option<VertexAttribute>]) {
        (self.vertex_data, self.sparse_layout)
    }

    #[inline(always)]
    fn write_2_f32(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector2<f32>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            LittleEndian::write_f32(&mut data[(attribute.offset as usize)..], value.x);
            LittleEndian::write_f32(&mut data[(attribute.offset as usize + 4)..], value.y);
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }

    #[inline(always)]
    fn write_3_f32(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector3<f32>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            LittleEndian::write_f32(&mut data[(attribute.offset as usize)..], value.x);
            LittleEndian::write_f32(&mut data[(attribute.offset as usize + 4)..], value.y);
            LittleEndian::write_f32(&mut data[(attribute.offset as usize + 8)..], value.z);
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }

    #[inline(always)]
    fn write_4_f32(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector4<f32>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            LittleEndian::write_f32(&mut data[(attribute.offset as usize)..], value.x);
            LittleEndian::write_f32(&mut data[(attribute.offset as usize + 4)..], value.y);
            LittleEndian::write_f32(&mut data[(attribute.offset as usize + 8)..], value.z);
            LittleEndian::write_f32(&mut data[(attribute.offset as usize + 12)..], value.w);
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }

    #[inline(always)]
    fn write_4_u8(
        &mut self,
        usage: VertexAttributeUsage,
        value: Vector4<u8>,
    ) -> Result<(), VertexFetchError> {
        let (data, layout) = self.data_layout_mut();
        if let Some(attribute) = layout.get(usage as usize).unwrap() {
            data[attribute.offset as usize] = value.x;
            data[(attribute.offset + 1) as usize] = value.y;
            data[(attribute.offset + 2) as usize] = value.z;
            data[(attribute.offset + 3) as usize] = value.w;
            Ok(())
        } else {
            Err(VertexFetchError::NoSuchAttribute(usage))
        }
    }
}

/// A buffer for data that defines connections between vertices.
#[derive(Visit, Default, Clone, Debug)]
pub struct TriangleBuffer {
    triangles: Vec<TriangleDefinition>,
    data_hash: u64,
}

fn calculate_triangle_buffer_hash(triangles: &[TriangleDefinition]) -> u64 {
    let mut hasher = FxHasher::default();
    triangles.hash(&mut hasher);
    hasher.finish()
}

impl TriangleBuffer {
    /// Creates new triangle buffer with given set of triangles.
    pub fn new(triangles: Vec<TriangleDefinition>) -> Self {
        let hash = calculate_triangle_buffer_hash(&triangles);

        Self {
            triangles,
            data_hash: hash,
        }
    }

    /// Creates new ref iterator.
    pub fn iter(&self) -> impl Iterator<Item = &TriangleDefinition> {
        self.triangles.iter()
    }

    /// Returns a ref to inner data with triangles.
    pub fn triangles_ref(&self) -> &[TriangleDefinition] {
        &self.triangles
    }

    /// Sets a new set of triangles.
    pub fn set_triangles(&mut self, triangles: Vec<TriangleDefinition>) {
        self.data_hash = calculate_triangle_buffer_hash(&triangles);
        self.triangles = triangles;
    }

    /// Returns amount of triangles in the buffer.
    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    /// Returns true if the buffer is empty, false - otherwise.
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Returns cached data hash. Cached value is guaranteed to be in actual state.
    pub fn data_hash(&self) -> u64 {
        self.data_hash
    }

    /// See VertexBuffer::modify for more info.
    pub fn modify(&mut self) -> TriangleBufferRefMut<'_> {
        TriangleBufferRefMut {
            triangle_buffer: self,
        }
    }
}

impl Index<usize> for TriangleBuffer {
    type Output = TriangleDefinition;

    fn index(&self, index: usize) -> &Self::Output {
        &self.triangles[index]
    }
}

/// See VertexBuffer::modify for more info.
pub struct TriangleBufferRefMut<'a> {
    triangle_buffer: &'a mut TriangleBuffer,
}

impl<'a> Deref for TriangleBufferRefMut<'a> {
    type Target = TriangleBuffer;

    fn deref(&self) -> &Self::Target {
        self.triangle_buffer
    }
}

impl<'a> DerefMut for TriangleBufferRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.triangle_buffer
    }
}

impl<'a> Drop for TriangleBufferRefMut<'a> {
    fn drop(&mut self) {
        self.triangle_buffer.data_hash =
            calculate_triangle_buffer_hash(&self.triangle_buffer.triangles);
    }
}

impl<'a> TriangleBufferRefMut<'a> {
    /// Returns mutable iterator.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TriangleDefinition> {
        self.triangles.iter_mut()
    }

    /// Adds new triangle in the buffer.
    pub fn push(&mut self, triangle: TriangleDefinition) {
        self.triangles.push(triangle)
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        self.triangles.clear();
    }
}

impl<'a> Index<usize> for TriangleBufferRefMut<'a> {
    type Output = TriangleDefinition;

    fn index(&self, index: usize) -> &Self::Output {
        &self.triangle_buffer.triangles[index]
    }
}

impl<'a> IndexMut<usize> for TriangleBufferRefMut<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.triangle_buffer.triangles[index]
    }
}

#[cfg(test)]
mod test {
    use crate::scene::mesh::buffer::VertexTrait;
    use crate::{
        core::algebra::{Vector2, Vector3, Vector4},
        scene::mesh::buffer::{
            VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage, VertexBuffer,
            VertexReadTrait,
        },
    };

    #[derive(Clone, Copy, PartialEq, Debug)]
    #[repr(C)]
    struct Vertex {
        position: Vector3<f32>,
        tex_coord: Vector2<f32>,
        second_tex_coord: Vector2<f32>,
        normal: Vector3<f32>,
        tangent: Vector4<f32>,
        bone_weights: Vector4<f32>,
        bone_indices: Vector4<u8>,
    }

    impl VertexTrait for Vertex {
        fn layout() -> &'static [VertexAttributeDescriptor] {
            static LAYOUT: [VertexAttributeDescriptor; 7] = [
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::Position,
                    data_type: VertexAttributeDataType::F32,
                    size: 3,
                    divisor: 0,
                    shader_location: 0,
                },
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::TexCoord0,
                    data_type: VertexAttributeDataType::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 1,
                },
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::TexCoord1,
                    data_type: VertexAttributeDataType::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 2,
                },
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::Normal,
                    data_type: VertexAttributeDataType::F32,
                    size: 3,
                    divisor: 0,
                    shader_location: 3,
                },
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::Tangent,
                    data_type: VertexAttributeDataType::F32,
                    size: 4,
                    divisor: 0,
                    shader_location: 4,
                },
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::BoneWeight,
                    data_type: VertexAttributeDataType::F32,
                    size: 4,
                    divisor: 0,
                    shader_location: 5,
                },
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::BoneIndices,
                    data_type: VertexAttributeDataType::U8,
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
            view.read_3_f32(VertexAttributeUsage::Position).unwrap(),
            original.position
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeUsage::TexCoord0).unwrap(),
            original.tex_coord
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeUsage::TexCoord1).unwrap(),
            original.second_tex_coord
        );
        assert_eq!(
            view.read_3_f32(VertexAttributeUsage::Normal).unwrap(),
            original.normal
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeUsage::Tangent).unwrap(),
            original.tangent
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeUsage::BoneWeight).unwrap(),
            original.bone_weights
        );
        assert_eq!(
            view.read_4_u8(VertexAttributeUsage::BoneIndices).unwrap(),
            original.bone_indices
        );
    }

    fn create_test_buffer() -> VertexBuffer {
        VertexBuffer::new(VERTICES.len(), VERTICES.to_vec()).unwrap()
    }

    #[test]
    fn test_empty() {
        VertexBuffer::new::<Vertex>(0, vec![]).unwrap();
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

        for (view, original) in buffer.modify().iter_mut().zip(VERTICES.iter()) {
            test_view_original_equal(view, original);
        }
    }

    #[test]
    fn test_vertex_duplication() {
        let mut buffer = create_test_buffer();

        buffer.modify().duplicate(0);

        assert_eq!(buffer.vertex_count(), 4);
        assert_eq!(buffer.get(0).unwrap(), buffer.get(3).unwrap())
    }

    #[test]
    fn test_pop_vertex() {
        let mut buffer = create_test_buffer();

        let vertex = buffer.modify().pop_vertex::<Vertex>().unwrap();

        assert_eq!(buffer.vertex_count(), 2);
        assert_eq!(vertex, VERTICES[2]);
    }

    #[test]
    fn test_remove_last_vertex() {
        let mut buffer = create_test_buffer();

        buffer.modify().remove_last_vertex();

        assert_eq!(buffer.vertex_count(), 2);
    }

    #[test]
    fn test_add_attribute() {
        let mut buffer = create_test_buffer();

        let fill = Vector2::new(0.25, 0.75);
        let test_index = 1;

        buffer
            .modify()
            .add_attribute(
                VertexAttributeDescriptor {
                    usage: VertexAttributeUsage::TexCoord2,
                    data_type: VertexAttributeDataType::F32,
                    size: 2,
                    divisor: 0,
                    shader_location: 7,
                },
                fill,
            )
            .unwrap();

        #[derive(Clone, Copy, PartialEq, Debug)]
        #[repr(C)]
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
            view.read_3_f32(VertexAttributeUsage::Position).unwrap(),
            new_1.position
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeUsage::TexCoord0).unwrap(),
            new_1.tex_coord
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeUsage::TexCoord1).unwrap(),
            new_1.second_tex_coord
        );
        assert_eq!(
            view.read_2_f32(VertexAttributeUsage::TexCoord2).unwrap(),
            new_1.third_tex_coord
        );
        assert_eq!(
            view.read_3_f32(VertexAttributeUsage::Normal).unwrap(),
            new_1.normal
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeUsage::Tangent).unwrap(),
            new_1.tangent
        );
        assert_eq!(
            view.read_4_f32(VertexAttributeUsage::BoneWeight).unwrap(),
            new_1.bone_weights
        );
        assert_eq!(
            view.read_4_u8(VertexAttributeUsage::BoneIndices).unwrap(),
            new_1.bone_indices
        );
    }
}
