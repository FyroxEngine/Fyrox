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

use crate::visitor::{error::VisitError, VisitResult, VisitableElementaryField};
use base64::Engine;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use nalgebra::{
    Complex, Const, Matrix, Matrix2, Matrix3, Matrix4, Quaternion, RawStorage, RawStorageMut,
    SVector, Scalar, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4, U1,
};
use std::io::{Read, Write};
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

impl FieldKind {
    pub fn as_string(&self) -> String {
        match self {
            Self::Bool(data) => format!("<bool = {data}>, "),
            Self::U8(data) => format!("<u8 = {data}>, "),
            Self::I8(data) => format!("<i8 = {data}>, "),
            Self::U16(data) => format!("<u16 = {data}>, "),
            Self::I16(data) => format!("<i16 = {data}>, "),
            Self::U32(data) => format!("<u32 = {data}>, "),
            Self::I32(data) => format!("<i32 = {data}>, "),
            Self::U64(data) => format!("<u64 = {data}>, "),
            Self::I64(data) => format!("<i64 = {data}>, "),
            Self::F32(data) => format!("<f32 = {data}>, "),
            Self::F64(data) => format!("<f64 = {data}>, "),
            Self::Vector2F32(data) => format!("<vec2f32 = {}; {}>, ", data.x, data.y),
            Self::Vector3F32(data) => format!("<vec3f32 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4F32(data) => {
                format!(
                    "<vec4f32 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }
            Self::Vector2F64(data) => format!("<vec2f64 = {}; {}>, ", data.x, data.y),
            Self::Vector3F64(data) => format!("<vec3f64 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4F64(data) => {
                format!(
                    "<vec4f64 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }
            Self::Vector2I8(data) => format!("<vec2i8 = {}; {}>, ", data.x, data.y),
            Self::Vector3I8(data) => format!("<vec3i8 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4I8(data) => {
                format!(
                    "<vec4i8 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }
            Self::Vector2U8(data) => format!("<vec2u8 = {}; {}>, ", data.x, data.y),
            Self::Vector3U8(data) => format!("<vec3u8 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4U8(data) => {
                format!(
                    "<vec4u8 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }

            Self::Vector2I16(data) => format!("<vec2i16 = {}; {}>, ", data.x, data.y),
            Self::Vector3I16(data) => format!("<vec3i16 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4I16(data) => {
                format!(
                    "<vec4i16 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }
            Self::Vector2U16(data) => format!("<vec2u16 = {}; {}>, ", data.x, data.y),
            Self::Vector3U16(data) => format!("<vec3u16 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4U16(data) => {
                format!(
                    "<vec4u16 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }

            Self::Vector2I32(data) => format!("<vec2i32 = {}; {}>, ", data.x, data.y),
            Self::Vector3I32(data) => format!("<vec3i32 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4I32(data) => {
                format!(
                    "<vec4i32 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }
            Self::Vector2U32(data) => format!("<vec2u32 = {}; {}>, ", data.x, data.y),
            Self::Vector3U32(data) => format!("<vec3u32 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4U32(data) => {
                format!(
                    "<vec4u32 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }

            Self::Vector2I64(data) => format!("<vec2i64 = {}; {}>, ", data.x, data.y),
            Self::Vector3I64(data) => format!("<vec3i64 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4I64(data) => {
                format!(
                    "<vec4i64 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }
            Self::Vector2U64(data) => format!("<vec2u64 = {}; {}>, ", data.x, data.y),
            Self::Vector3U64(data) => format!("<vec3u64 = {}; {}; {}>, ", data.x, data.y, data.z),
            Self::Vector4U64(data) => {
                format!(
                    "<vec4u64 = {}; {}; {}; {}>, ",
                    data.x, data.y, data.z, data.w
                )
            }

            Self::UnitQuaternion(data) => {
                format!("<quat = {}; {}; {}; {}>, ", data.i, data.j, data.k, data.w)
            }
            Self::Matrix4(data) => {
                let mut out = String::from("<mat4 = ");
                for f in data.iter() {
                    out += format!("{f}; ").as_str();
                }
                out
            }
            Self::BinaryBlob(data) => {
                let out = match String::from_utf8(data.clone()) {
                    Ok(s) => s,
                    Err(_) => base64::engine::general_purpose::STANDARD.encode(data),
                };
                format!("<data = {out}>, ")
            }
            Self::Matrix3(data) => {
                let mut out = String::from("<mat3 = ");
                for f in data.iter() {
                    out += format!("{f}; ").as_str();
                }
                out
            }
            Self::Uuid(uuid) => {
                format!("<uuid = {uuid}")
            }
            Self::UnitComplex(data) => {
                format!("<complex = {}; {}>, ", data.re, data.im)
            }
            FieldKind::PodArray {
                type_id,
                element_size,
                bytes,
            } => {
                let base64_encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                format!("<podarray = {type_id}; {element_size}; [{base64_encoded}]>")
            }
            Self::Matrix2(data) => {
                let mut out = String::from("<mat2 = ");
                for f in data.iter() {
                    out += format!("{f}; ").as_str();
                }
                out
            }
        }
    }
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

    pub fn save(field: &Field, file: &mut dyn Write) -> VisitResult {
        fn write_vec_n<T, const N: usize>(
            file: &mut dyn Write,
            type_id: u8,
            vec: &SVector<T, N>,
        ) -> VisitResult
        where
            T: VisitableElementaryField,
        {
            file.write_u8(type_id)?;
            for v in vec.iter() {
                v.write(file)?;
            }
            Ok(())
        }

        let name = field.name.as_bytes();
        file.write_u32::<LittleEndian>(name.len() as u32)?;
        file.write_all(name)?;
        match &field.kind {
            FieldKind::U8(data) => {
                file.write_u8(1)?;
                file.write_u8(*data)?;
            }
            FieldKind::I8(data) => {
                file.write_i8(2)?;
                file.write_i8(*data)?;
            }
            FieldKind::U16(data) => {
                file.write_u8(3)?;
                file.write_u16::<LittleEndian>(*data)?;
            }
            FieldKind::I16(data) => {
                file.write_u8(4)?;
                file.write_i16::<LittleEndian>(*data)?;
            }
            FieldKind::U32(data) => {
                file.write_u8(5)?;
                file.write_u32::<LittleEndian>(*data)?;
            }
            FieldKind::I32(data) => {
                file.write_u8(6)?;
                file.write_i32::<LittleEndian>(*data)?;
            }
            FieldKind::U64(data) => {
                file.write_u8(7)?;
                file.write_u64::<LittleEndian>(*data)?;
            }
            FieldKind::I64(data) => {
                file.write_u8(8)?;
                file.write_i64::<LittleEndian>(*data)?;
            }
            FieldKind::F32(data) => {
                file.write_u8(9)?;
                file.write_f32::<LittleEndian>(*data)?;
            }
            FieldKind::F64(data) => {
                file.write_u8(10)?;
                file.write_f64::<LittleEndian>(*data)?;
            }
            FieldKind::Vector3F32(data) => {
                write_vec_n(file, 11, data)?;
            }
            FieldKind::UnitQuaternion(data) => {
                file.write_u8(12)?;
                file.write_f32::<LittleEndian>(data.i)?;
                file.write_f32::<LittleEndian>(data.j)?;
                file.write_f32::<LittleEndian>(data.k)?;
                file.write_f32::<LittleEndian>(data.w)?;
            }
            FieldKind::Matrix4(data) => {
                file.write_u8(13)?;
                for f in data.iter() {
                    file.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::BinaryBlob(data) => {
                file.write_u8(14)?;
                file.write_u32::<LittleEndian>(data.len() as u32)?;
                file.write_all(data.as_slice())?;
            }
            FieldKind::Bool(data) => {
                file.write_u8(15)?;
                file.write_u8(u8::from(*data))?;
            }
            FieldKind::Matrix3(data) => {
                file.write_u8(16)?;
                for f in data.iter() {
                    file.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::Vector2F32(data) => {
                write_vec_n(file, 17, data)?;
            }
            FieldKind::Vector4F32(data) => {
                write_vec_n(file, 18, data)?;
            }
            FieldKind::Uuid(uuid) => {
                file.write_u8(19)?;
                file.write_all(uuid.as_bytes())?;
            }
            FieldKind::UnitComplex(c) => {
                file.write_u8(20)?;
                file.write_f32::<LittleEndian>(c.re)?;
                file.write_f32::<LittleEndian>(c.im)?;
            }
            FieldKind::PodArray {
                type_id,
                element_size,
                bytes,
            } => {
                file.write_u8(21)?;
                file.write_u8(*type_id)?;
                file.write_u32::<LittleEndian>(*element_size)?;
                file.write_u64::<LittleEndian>(bytes.len() as u64)?;
                file.write_all(bytes)?;
            }
            FieldKind::Matrix2(data) => {
                file.write_u8(22)?;
                for f in data.iter() {
                    file.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::Vector2F64(data) => {
                write_vec_n(file, 23, data)?;
            }
            FieldKind::Vector3F64(data) => {
                write_vec_n(file, 24, data)?;
            }
            FieldKind::Vector4F64(data) => {
                write_vec_n(file, 25, data)?;
            }

            FieldKind::Vector2I8(data) => {
                write_vec_n(file, 26, data)?;
            }
            FieldKind::Vector3I8(data) => {
                write_vec_n(file, 27, data)?;
            }
            FieldKind::Vector4I8(data) => {
                write_vec_n(file, 28, data)?;
            }

            FieldKind::Vector2U8(data) => {
                write_vec_n(file, 29, data)?;
            }
            FieldKind::Vector3U8(data) => {
                write_vec_n(file, 30, data)?;
            }
            FieldKind::Vector4U8(data) => {
                write_vec_n(file, 31, data)?;
            }

            FieldKind::Vector2I16(data) => {
                write_vec_n(file, 32, data)?;
            }
            FieldKind::Vector3I16(data) => {
                write_vec_n(file, 33, data)?;
            }
            FieldKind::Vector4I16(data) => {
                write_vec_n(file, 34, data)?;
            }

            FieldKind::Vector2U16(data) => {
                write_vec_n(file, 35, data)?;
            }
            FieldKind::Vector3U16(data) => {
                write_vec_n(file, 36, data)?;
            }
            FieldKind::Vector4U16(data) => {
                write_vec_n(file, 37, data)?;
            }

            FieldKind::Vector2I32(data) => {
                write_vec_n(file, 38, data)?;
            }
            FieldKind::Vector3I32(data) => {
                write_vec_n(file, 39, data)?;
            }
            FieldKind::Vector4I32(data) => {
                write_vec_n(file, 40, data)?;
            }

            FieldKind::Vector2U32(data) => {
                write_vec_n(file, 41, data)?;
            }
            FieldKind::Vector3U32(data) => {
                write_vec_n(file, 42, data)?;
            }
            FieldKind::Vector4U32(data) => {
                write_vec_n(file, 43, data)?;
            }

            FieldKind::Vector2I64(data) => {
                write_vec_n(file, 44, data)?;
            }
            FieldKind::Vector3I64(data) => {
                write_vec_n(file, 45, data)?;
            }
            FieldKind::Vector4I64(data) => {
                write_vec_n(file, 46, data)?;
            }

            FieldKind::Vector2U64(data) => {
                write_vec_n(file, 47, data)?;
            }
            FieldKind::Vector3U64(data) => {
                write_vec_n(file, 48, data)?;
            }
            FieldKind::Vector4U64(data) => {
                write_vec_n(file, 49, data)?;
            }
        }
        Ok(())
    }

    pub fn load(file: &mut dyn Read) -> Result<Field, VisitError> {
        fn read_vec_n<T, S, const N: usize>(
            file: &mut dyn Read,
        ) -> Result<Matrix<T, Const<N>, U1, S>, VisitError>
        where
            T: VisitableElementaryField + Scalar + Default,
            S: RawStorage<T, Const<N>> + RawStorageMut<T, Const<N>> + Default,
        {
            let mut vec = Matrix::<T, Const<N>, U1, S>::default();
            for v in vec.iter_mut() {
                v.read(file)?;
            }
            Ok(vec)
        }

        let name_len = file.read_u32::<LittleEndian>()? as usize;
        let mut raw_name = vec![Default::default(); name_len];
        file.read_exact(raw_name.as_mut_slice())?;
        let id = file.read_u8()?;
        Ok(Field::new(
            String::from_utf8(raw_name)?.as_str(),
            match id {
                1 => FieldKind::U8(file.read_u8()?),
                2 => FieldKind::I8(file.read_i8()?),
                3 => FieldKind::U16(file.read_u16::<LittleEndian>()?),
                4 => FieldKind::I16(file.read_i16::<LittleEndian>()?),
                5 => FieldKind::U32(file.read_u32::<LittleEndian>()?),
                6 => FieldKind::I32(file.read_i32::<LittleEndian>()?),
                7 => FieldKind::U64(file.read_u64::<LittleEndian>()?),
                8 => FieldKind::I64(file.read_i64::<LittleEndian>()?),
                9 => FieldKind::F32(file.read_f32::<LittleEndian>()?),
                10 => FieldKind::F64(file.read_f64::<LittleEndian>()?),
                11 => FieldKind::Vector3F32({
                    let x = file.read_f32::<LittleEndian>()?;
                    let y = file.read_f32::<LittleEndian>()?;
                    let z = file.read_f32::<LittleEndian>()?;
                    Vector3::new(x, y, z)
                }),
                12 => FieldKind::UnitQuaternion({
                    let x = file.read_f32::<LittleEndian>()?;
                    let y = file.read_f32::<LittleEndian>()?;
                    let z = file.read_f32::<LittleEndian>()?;
                    let w = file.read_f32::<LittleEndian>()?;
                    UnitQuaternion::new_normalize(Quaternion::new(w, x, y, z))
                }),
                13 => FieldKind::Matrix4({
                    let mut f = [0.0f32; 16];
                    for n in &mut f {
                        *n = file.read_f32::<LittleEndian>()?;
                    }
                    Matrix4::from_row_slice(&f)
                }),
                14 => FieldKind::BinaryBlob({
                    let len = file.read_u32::<LittleEndian>()? as usize;
                    let mut vec = vec![Default::default(); len];
                    file.read_exact(vec.as_mut_slice())?;
                    vec
                }),
                15 => FieldKind::Bool(file.read_u8()? != 0),
                16 => FieldKind::Matrix3({
                    let mut f = [0.0f32; 9];
                    for n in &mut f {
                        *n = file.read_f32::<LittleEndian>()?;
                    }
                    Matrix3::from_row_slice(&f)
                }),
                17 => FieldKind::Vector2F32({
                    let x = file.read_f32::<LittleEndian>()?;
                    let y = file.read_f32::<LittleEndian>()?;
                    Vector2::new(x, y)
                }),
                18 => FieldKind::Vector4F32({
                    let x = file.read_f32::<LittleEndian>()?;
                    let y = file.read_f32::<LittleEndian>()?;
                    let z = file.read_f32::<LittleEndian>()?;
                    let w = file.read_f32::<LittleEndian>()?;
                    Vector4::new(x, y, z, w)
                }),
                19 => FieldKind::Uuid({
                    let mut bytes = uuid::Bytes::default();
                    file.read_exact(&mut bytes)?;
                    Uuid::from_bytes(bytes)
                }),
                20 => FieldKind::UnitComplex({
                    let re = file.read_f32::<LittleEndian>()?;
                    let im = file.read_f32::<LittleEndian>()?;
                    UnitComplex::from_complex(Complex::new(re, im))
                }),
                21 => {
                    let type_id = file.read_u8()?;
                    let element_size = file.read_u32::<LittleEndian>()?;
                    let data_size = file.read_u64::<LittleEndian>()?;
                    let mut bytes = vec![0; data_size as usize];
                    file.read_exact(&mut bytes)?;
                    FieldKind::PodArray {
                        type_id,
                        element_size,
                        bytes,
                    }
                }
                22 => FieldKind::Matrix2({
                    let mut f = [0.0f32; 3];
                    for n in &mut f {
                        *n = file.read_f32::<LittleEndian>()?;
                    }
                    Matrix2::from_row_slice(&f)
                }),
                23 => FieldKind::Vector2F64(read_vec_n(file)?),
                24 => FieldKind::Vector3F64(read_vec_n(file)?),
                25 => FieldKind::Vector4F64(read_vec_n(file)?),

                26 => FieldKind::Vector2I8(read_vec_n(file)?),
                27 => FieldKind::Vector3I8(read_vec_n(file)?),
                28 => FieldKind::Vector4I8(read_vec_n(file)?),

                29 => FieldKind::Vector2U8(read_vec_n(file)?),
                30 => FieldKind::Vector3U8(read_vec_n(file)?),
                31 => FieldKind::Vector4U8(read_vec_n(file)?),

                32 => FieldKind::Vector2I16(read_vec_n(file)?),
                33 => FieldKind::Vector3I16(read_vec_n(file)?),
                34 => FieldKind::Vector4I16(read_vec_n(file)?),

                35 => FieldKind::Vector2U16(read_vec_n(file)?),
                36 => FieldKind::Vector3U16(read_vec_n(file)?),
                37 => FieldKind::Vector4U16(read_vec_n(file)?),

                38 => FieldKind::Vector2I32(read_vec_n(file)?),
                39 => FieldKind::Vector3I32(read_vec_n(file)?),
                40 => FieldKind::Vector4I32(read_vec_n(file)?),

                41 => FieldKind::Vector2U32(read_vec_n(file)?),
                42 => FieldKind::Vector3U32(read_vec_n(file)?),
                43 => FieldKind::Vector4U32(read_vec_n(file)?),

                44 => FieldKind::Vector2I64(read_vec_n(file)?),
                45 => FieldKind::Vector3I64(read_vec_n(file)?),
                46 => FieldKind::Vector4I64(read_vec_n(file)?),

                47 => FieldKind::Vector2U64(read_vec_n(file)?),
                48 => FieldKind::Vector3U64(read_vec_n(file)?),
                49 => FieldKind::Vector4U64(read_vec_n(file)?),

                _ => return Err(VisitError::UnknownFieldType(id)),
            },
        ))
    }

    pub fn as_string(&self) -> String {
        format!("{}{}", self.name, self.kind.as_string())
    }
}
