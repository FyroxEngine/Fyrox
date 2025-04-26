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

use crate::visitor::{
    field::{Field, FieldKind},
    writer::Writer,
    VisitResult, VisitableElementaryField, Visitor, VisitorNode,
};
use byteorder::{LittleEndian, WriteBytesExt};
use nalgebra::SVector;
use std::io::Write;

#[derive(Default)]
pub struct BinaryWriter {}

fn write_vec_n<T, const N: usize>(
    dest: &mut dyn Write,
    type_id: u8,
    vec: &SVector<T, N>,
) -> VisitResult
where
    T: VisitableElementaryField,
{
    dest.write_u8(type_id)?;
    for v in vec.iter() {
        v.write(dest)?;
    }
    Ok(())
}

impl Writer for BinaryWriter {
    fn write_field(&self, field: &Field, dest: &mut dyn Write) -> VisitResult {
        let name = field.name.as_bytes();
        dest.write_u32::<LittleEndian>(name.len() as u32)?;
        dest.write_all(name)?;
        match &field.kind {
            FieldKind::U8(data) => {
                dest.write_u8(1)?;
                dest.write_u8(*data)?;
            }
            FieldKind::I8(data) => {
                dest.write_i8(2)?;
                dest.write_i8(*data)?;
            }
            FieldKind::U16(data) => {
                dest.write_u8(3)?;
                dest.write_u16::<LittleEndian>(*data)?;
            }
            FieldKind::I16(data) => {
                dest.write_u8(4)?;
                dest.write_i16::<LittleEndian>(*data)?;
            }
            FieldKind::U32(data) => {
                dest.write_u8(5)?;
                dest.write_u32::<LittleEndian>(*data)?;
            }
            FieldKind::I32(data) => {
                dest.write_u8(6)?;
                dest.write_i32::<LittleEndian>(*data)?;
            }
            FieldKind::U64(data) => {
                dest.write_u8(7)?;
                dest.write_u64::<LittleEndian>(*data)?;
            }
            FieldKind::I64(data) => {
                dest.write_u8(8)?;
                dest.write_i64::<LittleEndian>(*data)?;
            }
            FieldKind::F32(data) => {
                dest.write_u8(9)?;
                dest.write_f32::<LittleEndian>(*data)?;
            }
            FieldKind::F64(data) => {
                dest.write_u8(10)?;
                dest.write_f64::<LittleEndian>(*data)?;
            }
            FieldKind::Vector3F32(data) => {
                write_vec_n(dest, 11, data)?;
            }
            FieldKind::UnitQuaternion(data) => {
                dest.write_u8(12)?;
                dest.write_f32::<LittleEndian>(data.i)?;
                dest.write_f32::<LittleEndian>(data.j)?;
                dest.write_f32::<LittleEndian>(data.k)?;
                dest.write_f32::<LittleEndian>(data.w)?;
            }
            FieldKind::Matrix4(data) => {
                dest.write_u8(13)?;
                for f in data.iter() {
                    dest.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::BinaryBlob(data) => {
                dest.write_u8(14)?;
                dest.write_u32::<LittleEndian>(data.len() as u32)?;
                dest.write_all(data.as_slice())?;
            }
            FieldKind::Bool(data) => {
                dest.write_u8(15)?;
                dest.write_u8(u8::from(*data))?;
            }
            FieldKind::Matrix3(data) => {
                dest.write_u8(16)?;
                for f in data.iter() {
                    dest.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::Vector2F32(data) => {
                write_vec_n(dest, 17, data)?;
            }
            FieldKind::Vector4F32(data) => {
                write_vec_n(dest, 18, data)?;
            }
            FieldKind::Uuid(uuid) => {
                dest.write_u8(19)?;
                dest.write_all(uuid.as_bytes())?;
            }
            FieldKind::UnitComplex(c) => {
                dest.write_u8(20)?;
                dest.write_f32::<LittleEndian>(c.re)?;
                dest.write_f32::<LittleEndian>(c.im)?;
            }
            FieldKind::PodArray {
                type_id,
                element_size,
                bytes,
            } => {
                dest.write_u8(21)?;
                dest.write_u8(*type_id)?;
                dest.write_u32::<LittleEndian>(*element_size)?;
                dest.write_u64::<LittleEndian>(bytes.len() as u64)?;
                dest.write_all(bytes)?;
            }
            FieldKind::Matrix2(data) => {
                dest.write_u8(22)?;
                for f in data.iter() {
                    dest.write_f32::<LittleEndian>(*f)?;
                }
            }
            FieldKind::Vector2F64(data) => {
                write_vec_n(dest, 23, data)?;
            }
            FieldKind::Vector3F64(data) => {
                write_vec_n(dest, 24, data)?;
            }
            FieldKind::Vector4F64(data) => {
                write_vec_n(dest, 25, data)?;
            }

            FieldKind::Vector2I8(data) => {
                write_vec_n(dest, 26, data)?;
            }
            FieldKind::Vector3I8(data) => {
                write_vec_n(dest, 27, data)?;
            }
            FieldKind::Vector4I8(data) => {
                write_vec_n(dest, 28, data)?;
            }

            FieldKind::Vector2U8(data) => {
                write_vec_n(dest, 29, data)?;
            }
            FieldKind::Vector3U8(data) => {
                write_vec_n(dest, 30, data)?;
            }
            FieldKind::Vector4U8(data) => {
                write_vec_n(dest, 31, data)?;
            }

            FieldKind::Vector2I16(data) => {
                write_vec_n(dest, 32, data)?;
            }
            FieldKind::Vector3I16(data) => {
                write_vec_n(dest, 33, data)?;
            }
            FieldKind::Vector4I16(data) => {
                write_vec_n(dest, 34, data)?;
            }

            FieldKind::Vector2U16(data) => {
                write_vec_n(dest, 35, data)?;
            }
            FieldKind::Vector3U16(data) => {
                write_vec_n(dest, 36, data)?;
            }
            FieldKind::Vector4U16(data) => {
                write_vec_n(dest, 37, data)?;
            }

            FieldKind::Vector2I32(data) => {
                write_vec_n(dest, 38, data)?;
            }
            FieldKind::Vector3I32(data) => {
                write_vec_n(dest, 39, data)?;
            }
            FieldKind::Vector4I32(data) => {
                write_vec_n(dest, 40, data)?;
            }

            FieldKind::Vector2U32(data) => {
                write_vec_n(dest, 41, data)?;
            }
            FieldKind::Vector3U32(data) => {
                write_vec_n(dest, 42, data)?;
            }
            FieldKind::Vector4U32(data) => {
                write_vec_n(dest, 43, data)?;
            }

            FieldKind::Vector2I64(data) => {
                write_vec_n(dest, 44, data)?;
            }
            FieldKind::Vector3I64(data) => {
                write_vec_n(dest, 45, data)?;
            }
            FieldKind::Vector4I64(data) => {
                write_vec_n(dest, 46, data)?;
            }

            FieldKind::Vector2U64(data) => {
                write_vec_n(dest, 47, data)?;
            }
            FieldKind::Vector3U64(data) => {
                write_vec_n(dest, 48, data)?;
            }
            FieldKind::Vector4U64(data) => {
                write_vec_n(dest, 49, data)?;
            }
            FieldKind::String(str) => {
                dest.write_u8(50)?;
                dest.write_u32::<LittleEndian>(str.len() as u32)?;
                dest.write_all(str.as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_node(
        &self,
        _visitor: &Visitor,
        node: &VisitorNode,
        _hierarchy_level: usize,
        dest: &mut dyn Write,
    ) -> VisitResult {
        let name = node.name.as_bytes();
        dest.write_u32::<LittleEndian>(name.len() as u32)?;
        dest.write_all(name)?;

        dest.write_u32::<LittleEndian>(node.fields.len() as u32)?;
        for field in node.fields.iter() {
            self.write_field(field, dest)?;
        }

        dest.write_u32::<LittleEndian>(node.children.len() as u32)?;
        Ok(())
    }

    fn write(&self, visitor: &Visitor, dest: &mut dyn Write) -> VisitResult {
        dest.write_all(Visitor::MAGIC_BINARY_CURRENT.as_bytes())?;
        dest.write_u32::<LittleEndian>(visitor.version)?;
        let mut stack = vec![visitor.root];
        while let Some(node_handle) = stack.pop() {
            let node = visitor.nodes.borrow(node_handle);
            self.write_node(visitor, node, 0, dest)?;
            stack.extend_from_slice(&node.children);
        }
        Ok(())
    }
}
