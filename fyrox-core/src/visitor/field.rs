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

use nalgebra::{Matrix2, Matrix3, Matrix4, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4};
use uuid::Uuid;

/// The internal data format of [Visitor]. Fields are limited to being one of these types.
/// This means that all [Visit] values must be built from some assortment
/// of these types.
/// Fields can be accessed from a visitor using [Visit::visit] on a variable with the
/// same type as the field.
pub enum FieldKind {
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    UnitQuaternion(UnitQuaternion<f32>),
    Matrix4(Matrix4<f32>),
    /// A representation of some `Vec<T>` where `T` must be [Copy].
    /// It is mostly used to store the bytes of string types.
    BinaryBlob(Vec<u8>),
    Matrix3(Matrix3<f32>),
    Uuid(Uuid),
    UnitComplex(UnitComplex<f32>),
    /// A representation for arrays of [Pod] types as a `Vec<u8>`.
    PodArray {
        /// A code to identify the Pod type of the elements of the array.
        /// Taken from [Pod::type_id].
        type_id: u8,
        /// The number of bytes in each array element.
        element_size: u32,
        /// The bytes that store the data, with unspecified endianness.
        bytes: Vec<u8>,
    },
    Matrix2(Matrix2<f32>),

    Vector2F32(Vector2<f32>),
    Vector3F32(Vector3<f32>),
    Vector4F32(Vector4<f32>),

    Vector2F64(Vector2<f64>),
    Vector3F64(Vector3<f64>),
    Vector4F64(Vector4<f64>),

    Vector2U8(Vector2<u8>),
    Vector3U8(Vector3<u8>),
    Vector4U8(Vector4<u8>),

    Vector2I8(Vector2<i8>),
    Vector3I8(Vector3<i8>),
    Vector4I8(Vector4<i8>),

    Vector2U16(Vector2<u16>),
    Vector3U16(Vector3<u16>),
    Vector4U16(Vector4<u16>),

    Vector2I16(Vector2<i16>),
    Vector3I16(Vector3<i16>),
    Vector4I16(Vector4<i16>),

    Vector2U32(Vector2<u32>),
    Vector3U32(Vector3<u32>),
    Vector4U32(Vector4<u32>),

    Vector2I32(Vector2<i32>),
    Vector3I32(Vector3<i32>),
    Vector4I32(Vector4<i32>),

    Vector2U64(Vector2<u64>),
    Vector3U64(Vector3<u64>),
    Vector4U64(Vector4<u64>),

    Vector2I64(Vector2<i64>),
    Vector3I64(Vector3<i64>),
    Vector4I64(Vector4<i64>),
}

/// Values within a visitor are constructed from Fields.
/// Each Field has a name and a value. The name is used as a key to access the value
/// within the visitor using the [Visit::visit] method, so each field within a value
/// must have a unique name.
pub struct Field {
    /// The key string that allows access to the field.
    pub name: String,
    /// The data stored in the visitor for this field.
    pub kind: FieldKind,
}

impl Field {
    pub fn new(name: &str, kind: FieldKind) -> Self {
        Self {
            name: name.to_owned(),
            kind,
        }
    }
}
