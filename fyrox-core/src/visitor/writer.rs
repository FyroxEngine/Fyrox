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
    VisitResult, VisitableElementaryField, Visitor, VisitorNode,
};
use base64::Engine;
use byteorder::{LittleEndian, WriteBytesExt};
use nalgebra::SVector;
use std::fmt::Display;
use std::io::Write;

pub trait Writer {
    fn write_field(&self, field: &Field, dest: &mut dyn Write) -> VisitResult;
    fn write_node(
        &self,
        visitor: &Visitor,
        node: &VisitorNode,
        hierarchy_level: usize,
        dest: &mut dyn Write,
    ) -> VisitResult;
    fn write(&self, visitor: &Visitor, dest: &mut dyn Write) -> VisitResult;
}

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
        dest.write_all(Visitor::MAGIC_BINARY.as_bytes())?;
        let mut stack = vec![visitor.root];
        while let Some(node_handle) = stack.pop() {
            let node = visitor.nodes.borrow(node_handle);
            self.write_node(visitor, node, 0, dest)?;
            stack.extend_from_slice(&node.children);
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct AsciiWriter {}

impl Writer for AsciiWriter {
    fn write_field(&self, field: &Field, dest: &mut dyn Write) -> VisitResult {
        fn write_array(
            dest: &mut dyn Write,
            iter: impl Iterator<Item = impl Display>,
        ) -> VisitResult {
            for (i, f) in iter.enumerate() {
                if i != 0 {
                    write!(dest, "; ")?;
                }
                write!(dest, "{f}")?;
            }
            Ok(())
        }

        write!(dest, "{}", field.name)?;
        match field.kind {
            FieldKind::Bool(data) => write!(dest, "<bool:{data}>")?,
            FieldKind::U8(data) => write!(dest, "<u8:{data}>")?,
            FieldKind::I8(data) => write!(dest, "<i8:{data}>")?,
            FieldKind::U16(data) => write!(dest, "<u16:{data}>")?,
            FieldKind::I16(data) => write!(dest, "<i16:{data}>")?,
            FieldKind::U32(data) => write!(dest, "<u32:{data}>")?,
            FieldKind::I32(data) => write!(dest, "<i32:{data}>")?,
            FieldKind::U64(data) => write!(dest, "<u64:{data}>")?,
            FieldKind::I64(data) => write!(dest, "<i64:{data}>")?,
            FieldKind::F32(data) => write!(dest, "<f32:{data}>")?,
            FieldKind::F64(data) => write!(dest, "<f64:{data}>")?,
            FieldKind::Vector2F32(data) => write!(dest, "<vec2f32:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3F32(data) => {
                write!(dest, "<vec3f32:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4F32(data) => write!(
                dest,
                "<vec4f32:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2F64(data) => write!(dest, "<vec2f64:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3F64(data) => {
                write!(dest, "<vec3f64:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4F64(data) => write!(
                dest,
                "<vec4f64:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2I8(data) => write!(dest, "<vec2i8:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3I8(data) => {
                write!(dest, "<vec3i8:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4I8(data) => write!(
                dest,
                "<vec4i8:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2U8(data) => write!(dest, "<vec2u8:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3U8(data) => {
                write!(dest, "<vec3u8:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4U8(data) => write!(
                dest,
                "<vec4u8:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,

            FieldKind::Vector2I16(data) => write!(dest, "<vec2i16:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3I16(data) => {
                write!(dest, "<vec3i16:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4I16(data) => write!(
                dest,
                "<vec4i16:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2U16(data) => write!(dest, "<vec2u16:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3U16(data) => {
                write!(dest, "<vec3u16:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4U16(data) => write!(
                dest,
                "<vec4u16:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2I32(data) => write!(dest, "<vec2i32:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3I32(data) => {
                write!(dest, "<vec3i32:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4I32(data) => write!(
                dest,
                "<vec4i32:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2U32(data) => write!(dest, "<vec2u32:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3U32(data) => {
                write!(dest, "<vec3u32:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4U32(data) => write!(
                dest,
                "<vec4u32:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2I64(data) => write!(dest, "<vec2i64:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3I64(data) => {
                write!(dest, "<vec3i64:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4I64(data) => write!(
                dest,
                "<vec4i64:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::Vector2U64(data) => write!(dest, "<vec2u64:{}; {}>", data.x, data.y)?,
            FieldKind::Vector3U64(data) => {
                write!(dest, "<vec3u64:{}; {}; {}>", data.x, data.y, data.z)?
            }
            FieldKind::Vector4U64(data) => write!(
                dest,
                "<vec4u64:{}; {}; {}; {}>",
                data.x, data.y, data.z, data.w
            )?,
            FieldKind::UnitQuaternion(data) => write!(
                dest,
                "<quat:{}; {}; {}; {}>",
                data.i, data.j, data.k, data.w
            )?,
            FieldKind::Matrix4(data) => {
                write!(dest, "<mat4:")?;
                write_array(dest, data.iter())?;
                write!(dest, ">")?;
            }
            FieldKind::BinaryBlob(ref data) => write!(
                dest,
                "<data:{}>",
                base64::engine::general_purpose::STANDARD.encode(data)
            )?,
            FieldKind::Matrix3(data) => {
                write!(dest, "<mat3:")?;
                write_array(dest, data.iter())?;
                write!(dest, ">")?;
            }
            FieldKind::Uuid(uuid) => write!(dest, "<uuid:{uuid}>")?,
            FieldKind::UnitComplex(data) => write!(dest, "<complex:{}; {}>", data.re, data.im)?,
            FieldKind::PodArray {
                type_id,
                element_size,
                ref bytes,
            } => write!(
                dest,
                "<podarray:{type_id}; {element_size}; {}>",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )?,
            FieldKind::Matrix2(data) => {
                write!(dest, "<mat2:")?;
                write_array(dest, data.iter())?;
                write!(dest, ">")?;
            }
        }

        Ok(())
    }

    fn write_node(
        &self,
        visitor: &Visitor,
        node: &VisitorNode,
        hierarchy_level: usize,
        dest: &mut dyn Write,
    ) -> VisitResult {
        fn align(count: usize, dest: &mut dyn Write) -> VisitResult {
            for _ in 0..count {
                write!(dest, "\t")?;
            }
            Ok(())
        }

        align(hierarchy_level, dest)?;
        writeln!(dest, "{}", node.name)?;

        align(hierarchy_level, dest)?;
        write!(dest, "[{}:", node.fields.len())?;

        for field in node.fields.iter() {
            writeln!(dest)?;
            align(hierarchy_level + 1, dest)?;
            self.write_field(field, dest)?;
        }

        writeln!(dest)?;

        align(hierarchy_level, dest)?;
        writeln!(dest, "]")?;

        align(hierarchy_level, dest)?;
        writeln!(dest, "{{{}:", node.children.len())?;

        for child_handle in node.children.iter() {
            let child = visitor.nodes.borrow(*child_handle);
            self.write_node(visitor, child, hierarchy_level + 1, dest)?;
        }

        align(hierarchy_level, dest)?;
        writeln!(dest, "}}")?;

        Ok(())
    }

    fn write(&self, visitor: &Visitor, dest: &mut dyn Write) -> VisitResult {
        writeln!(dest, "{}", Visitor::MAGIC_ASCII)?;
        self.write_node(visitor, &visitor.nodes[visitor.root], 0, dest)
    }
}

#[cfg(test)]
mod test {
    use crate::visitor::prelude::*;
    use std::io::Cursor;

    #[derive(Visit)]
    struct MyOtherObject {
        data: f32,
        array: Vec<u32>,
    }

    #[derive(Visit)]
    struct MyObject {
        foo: String,
        object: MyOtherObject,
        bar: usize,
    }

    #[test]
    fn test_write_ascii() {
        let mut object = MyObject {
            foo: "Some String".to_string(),
            bar: 123,
            object: MyOtherObject {
                data: 321.123,
                array: vec![1, 2, 3, 3, 2, 1],
            },
        };

        let mut visitor = Visitor::new();
        object.visit("MyObject", &mut visitor).unwrap();

        let mut cursor = Cursor::<Vec<u8>>::default();
        visitor.save_ascii_to_memory(&mut cursor).unwrap();

        print!("{}", String::from_utf8(cursor.into_inner()).unwrap());
    }
}
