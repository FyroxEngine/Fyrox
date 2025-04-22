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

//! A set fundamental data types.

use nalgebra::{Matrix2, Matrix3, Matrix4, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4};
use uuid::Uuid;

/// The internal data format of [`crate::visitor::Visitor`]. Fields are limited to being one of these types.
/// This means that all [`crate::visitor::Visit`] values must be built from some assortment
/// of these types.
/// Fields can be accessed from a visitor using [`crate::visitor::Visit::visit`] on a variable with the
/// same type as the field.
#[derive(PartialEq, Debug)]
pub enum FieldKind {
    /// Boolean value.
    Bool(bool),
    /// `u8` value.
    U8(u8),
    /// `i8` value.
    I8(i8),
    /// `u16` value.
    U16(u16),
    /// `i16` value.
    I16(i16),
    /// `u32` value.
    U32(u32),
    /// `i32` value.
    I32(i32),
    /// `u64` value.
    U64(u64),
    /// `i64` value.
    I64(i64),
    /// `f32` value.
    F32(f32),
    /// `f64` value.
    F64(f64),
    /// Unit quaternion.
    UnitQuaternion(UnitQuaternion<f32>),
    /// 4x4 f32 matrix.
    Matrix4(Matrix4<f32>),
    /// A representation of some `Vec<T>` where `T` must be [Copy].
    /// It is mostly used to store the bytes of string types.
    BinaryBlob(Vec<u8>),
    /// 3x3 f32 matrix.
    Matrix3(Matrix3<f32>),
    /// Unique id.
    Uuid(Uuid),
    /// A complex number.
    UnitComplex(UnitComplex<f32>),
    /// A representation for arrays of [`crate::visitor::pod::Pod`] types as a `Vec<u8>`.
    PodArray {
        /// A code to identify the Pod type of the elements of the array.
        /// Taken from [`crate::visitor::pod::Pod::type_id`].
        type_id: u8,
        /// The number of bytes in each array element.
        element_size: u32,
        /// The bytes that store the data, with unspecified endianness.
        bytes: Vec<u8>,
    },
    /// 2x2 f32 matrix.
    Matrix2(Matrix2<f32>),
    /// 2D f32 vector.
    Vector2F32(Vector2<f32>),
    /// 3D f32 vector.
    Vector3F32(Vector3<f32>),
    /// 4D f32 vector.
    Vector4F32(Vector4<f32>),
    /// 2D f64 vector.
    Vector2F64(Vector2<f64>),
    /// 3D f64 vector.
    Vector3F64(Vector3<f64>),
    /// 4D f64 vector.
    Vector4F64(Vector4<f64>),
    /// 2D u8 vector.
    Vector2U8(Vector2<u8>),
    /// 3D u8 vector.
    Vector3U8(Vector3<u8>),
    /// 4D u8 vector.
    Vector4U8(Vector4<u8>),
    /// 2D i8 vector.
    Vector2I8(Vector2<i8>),
    /// 3D i8 vector.
    Vector3I8(Vector3<i8>),
    /// 4D i8 vector.
    Vector4I8(Vector4<i8>),
    /// 2D u16 vector.
    Vector2U16(Vector2<u16>),
    /// 3D u16 vector.
    Vector3U16(Vector3<u16>),
    /// 4D u16 vector.
    Vector4U16(Vector4<u16>),
    /// 2D i16 vector.
    Vector2I16(Vector2<i16>),
    /// 3D i16 vector.
    Vector3I16(Vector3<i16>),
    /// 4D i16 vector.
    Vector4I16(Vector4<i16>),
    /// 2D u32 vector.
    Vector2U32(Vector2<u32>),
    /// 3D u32 vector.
    Vector3U32(Vector3<u32>),
    /// 4D u32 vector.
    Vector4U32(Vector4<u32>),
    /// 2D i32 vector.
    Vector2I32(Vector2<i32>),
    /// 3D i32 vector.
    Vector3I32(Vector3<i32>),
    /// 4D i32 vector.
    Vector4I32(Vector4<i32>),
    /// 2D u64 vector.
    Vector2U64(Vector2<u64>),
    /// 3D u64 vector.
    Vector3U64(Vector3<u64>),
    /// 4D u64 vector.
    Vector4U64(Vector4<u64>),
    /// 2D i64 vector.
    Vector2I64(Vector2<i64>),
    /// 3D i64 vector.
    Vector3I64(Vector3<i64>),
    /// 4D i64 vector.
    Vector4I64(Vector4<i64>),
    /// A unicode string.
    String(String),
}

/// Values within a visitor are constructed from Fields. Each Field has a name and a value. The name
/// is used as a key to access the value within the visitor using the [`crate::Visit::visit`] method,
/// so each field within a value must have a unique name.
#[derive(PartialEq, Debug)]
pub struct Field {
    /// The key string that allows access to the field.
    pub name: String,
    /// The data stored in the visitor for this field.
    pub kind: FieldKind,
}

impl Field {
    /// Creates a new field from the given name and the given field kind.
    pub fn new(name: &str, kind: FieldKind) -> Self {
        Self {
            name: name.to_owned(),
            kind,
        }
    }
}
