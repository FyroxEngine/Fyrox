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
    VisitResult, Visitor, VisitorNode, LATEST_VERSION,
};
use base64::Engine;
use byteorder::WriteBytesExt;
use std::{fmt::Display, io::Write};

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
            FieldKind::String(ref str) => {
                write!(dest, "<str:\"")?;
                for &ascii_chr in str.as_bytes() {
                    if ascii_chr == b'\"' {
                        // Escape the quotes.
                        write!(dest, "\\\"")?;
                    } else if ascii_chr == b'\n' {
                        // Escape the new line. This is needed to prevent breaking the structured
                        // output and to reduce merge conflicts.
                        write!(dest, "\\n")?;
                    } else {
                        // The rest can be written as-is.
                        dest.write_u8(ascii_chr)?;
                    }
                }
                write!(dest, "\">")?
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
        write!(dest, "{}", node.name)?;

        write!(dest, "[{}:", node.fields.len())?;
        for field in node.fields.iter() {
            self.write_field(field, dest)?;
        }

        if node.children.is_empty() {
            writeln!(dest, "]{{0:}}")?;
        } else {
            writeln!(dest, "]")?;

            align(hierarchy_level, dest)?;
            writeln!(dest, "{{{}:", node.children.len())?;

            for child_handle in node.children.iter() {
                let child = visitor.nodes.borrow(*child_handle);
                self.write_node(visitor, child, hierarchy_level + 1, dest)?;
            }

            align(hierarchy_level, dest)?;
            writeln!(dest, "}}")?;
        }

        Ok(())
    }

    fn write(&self, visitor: &Visitor, dest: &mut dyn Write) -> VisitResult {
        writeln!(dest, "{}:{};", Visitor::MAGIC_ASCII_CURRENT, LATEST_VERSION)?;
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
