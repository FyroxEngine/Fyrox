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

use crate::visitor::VisitorVersion;
use crate::{
    pool::{Handle, Pool},
    visitor::{
        blackboard::Blackboard,
        error::VisitError,
        field::{Field, FieldKind},
        reader::Reader,
        VisitResult, Visitor, VisitorFlags, VisitorNode,
    },
};
use base64::Engine;
use byteorder::ReadBytesExt;
use nalgebra::{
    Complex, Quaternion, SMatrix, SVector, Scalar, UnitComplex, UnitQuaternion, Vector2, Vector4,
};
use std::{io::Read, str::FromStr};
use uuid::Uuid;

struct ByteReader<'a> {
    peeked: Option<u8>,
    src: &'a mut dyn Read,
}

impl<'a> ByteReader<'a> {
    #[allow(dead_code)]
    fn new(src: &'a mut dyn Read) -> Self {
        Self { peeked: None, src }
    }

    fn next(&mut self) -> Result<u8, VisitError> {
        match self.peeked.take() {
            Some(b) => Ok(b),
            None => Ok(self.src.read_u8()?),
        }
    }

    fn peek(&mut self) -> Result<u8, VisitError> {
        match self.peeked {
            Some(b) => Ok(b),
            None => {
                let b = self.src.read_u8()?;
                self.peeked = Some(b);
                Ok(b)
            }
        }
    }

    fn skip_until<F>(&mut self, mut f: F) -> VisitResult
    where
        F: FnMut(u8) -> bool,
    {
        loop {
            let ch = self.peek()?;
            if f(ch) {
                self.next()?;
            } else {
                return Ok(());
            }
        }
    }

    fn skip_ws(&mut self) -> VisitResult {
        self.skip_until(|ch| ch.is_ascii_whitespace())
    }

    fn skip_n(&mut self, n: usize) -> VisitResult {
        if n == 0 {
            return Ok(());
        }
        let mut counter = 0;
        loop {
            self.next()?;
            counter += 1;
            if counter == n {
                return Ok(());
            }
        }
    }

    fn read_until<F>(&mut self, buf: &mut Vec<u8>, mut f: F) -> VisitResult
    where
        F: FnMut(u8) -> bool,
    {
        loop {
            let ch = self.peek()?;
            if f(ch) {
                buf.push(ch);
                self.next()?;
            } else {
                return Ok(());
            }
        }
    }

    fn read_str_until<F>(&mut self, f: F) -> Result<String, VisitError>
    where
        F: FnMut(u8) -> bool,
    {
        let mut ascii_str = Vec::with_capacity(16);
        self.read_until(&mut ascii_str, f)?;
        Ok(String::from_utf8(ascii_str)?)
    }

    fn read_str_until_skip_ws(&mut self, stop: u8) -> Result<String, VisitError> {
        self.skip_ws()?;
        let str = self.read_str_until(|ch| ch != stop && !ch.is_ascii_whitespace())?;
        if self.peek()?.is_ascii_whitespace() {
            self.skip_until(|ch| ch != stop)?;
        }
        if self.peek()? == stop {
            self.skip_n(1)?;
        }
        Ok(str)
    }

    fn read_num_until<T, E>(&mut self, stop: u8) -> Result<T, VisitError>
    where
        T: Scalar + FromStr<Err = E>,
        VisitError: From<E>,
    {
        Ok(self.read_str_until_skip_ws(stop)?.parse::<T>()?)
    }

    fn read_num<T, E>(&mut self) -> Result<T, VisitError>
    where
        T: Scalar + FromStr<Err = E>,
        VisitError: From<E>,
    {
        self.read_num_until(b'>')
    }

    fn read_vec_n<T, E, const N: usize>(&mut self) -> Result<SVector<T, N>, VisitError>
    where
        T: Scalar + Default + FromStr<Err = E>,
        VisitError: From<E>,
        [T; N]: Default,
    {
        let mut vec = SVector::default();
        for i in 0..(N - 1) {
            vec[i] = self.read_str_until_skip_ws(b';')?.parse()?;
        }
        vec[N - 1] = self.read_str_until_skip_ws(b'>')?.parse()?;
        Ok(vec)
    }

    fn read_mat_n<T, E, const N: usize>(&mut self) -> Result<SMatrix<T, N, N>, VisitError>
    where
        T: Scalar + Default + FromStr<Err = E>,
        VisitError: From<E>,
        [[T; N]; N]: Default,
    {
        let mut vec = SMatrix::default();
        for i in 0..(N * N - 1) {
            vec[i] = self.read_str_until_skip_ws(b';')?.parse()?;
        }
        vec[N * N - 1] = self.read_str_until_skip_ws(b'>')?.parse()?;
        Ok(vec)
    }
}

macro_rules! err {
    ($($arg:tt)*) => {{
        let str = format!($($arg)*);
        Err(VisitError::User(str))
    }};
}

pub struct AsciiReader<'a> {
    src: ByteReader<'a>,
}

impl<'a> AsciiReader<'a> {
    #[allow(dead_code)]
    pub fn new(src: &'a mut dyn Read) -> Self {
        Self {
            src: ByteReader { peeked: None, src },
        }
    }
}

impl Reader for AsciiReader<'_> {
    fn read_field(&mut self) -> Result<Field, VisitError> {
        let src = &mut self.src;

        let name = src.read_str_until_skip_ws(b'<')?;
        let ty = src.read_str_until_skip_ws(b':')?;

        let kind = match ty.as_ref() {
            "bool" => {
                let value = src.read_str_until_skip_ws(b'>')?;
                if value == "true" {
                    FieldKind::Bool(true)
                } else if value == "false" {
                    FieldKind::Bool(false)
                } else {
                    return err!("bool must be either true or false, got {}", value);
                }
            }
            "u8" => FieldKind::U8(src.read_num()?),
            "i8" => FieldKind::I8(src.read_num()?),
            "u16" => FieldKind::U16(src.read_num()?),
            "i16" => FieldKind::I16(src.read_num()?),
            "u32" => FieldKind::U32(src.read_num()?),
            "i32" => FieldKind::I32(src.read_num()?),
            "u64" => FieldKind::U64(src.read_num()?),
            "i64" => FieldKind::I64(src.read_num()?),
            "f32" => FieldKind::F32(src.read_num()?),
            "f64" => FieldKind::F64(src.read_num()?),
            "vec2f32" => FieldKind::Vector2F32(src.read_vec_n()?),
            "vec3f32" => FieldKind::Vector3F32(src.read_vec_n()?),
            "vec4f32" => FieldKind::Vector4F32(src.read_vec_n()?),
            "vec2f64" => FieldKind::Vector2F64(src.read_vec_n()?),
            "vec3f64" => FieldKind::Vector3F64(src.read_vec_n()?),
            "vec4f64" => FieldKind::Vector4F64(src.read_vec_n()?),
            "vec2i8" => FieldKind::Vector2I8(src.read_vec_n()?),
            "vec3i8" => FieldKind::Vector3I8(src.read_vec_n()?),
            "vec4i8" => FieldKind::Vector4I8(src.read_vec_n()?),
            "vec2u8" => FieldKind::Vector2U8(src.read_vec_n()?),
            "vec3u8" => FieldKind::Vector3U8(src.read_vec_n()?),
            "vec4u8" => FieldKind::Vector4U8(src.read_vec_n()?),
            "vec2i16" => FieldKind::Vector2I16(src.read_vec_n()?),
            "vec3i16" => FieldKind::Vector3I16(src.read_vec_n()?),
            "vec4i16" => FieldKind::Vector4I16(src.read_vec_n()?),
            "vec2u16" => FieldKind::Vector2U16(src.read_vec_n()?),
            "vec3u16" => FieldKind::Vector3U16(src.read_vec_n()?),
            "vec4u16" => FieldKind::Vector4U16(src.read_vec_n()?),
            "vec2i32" => FieldKind::Vector2I32(src.read_vec_n()?),
            "vec3i32" => FieldKind::Vector3I32(src.read_vec_n()?),
            "vec4i32" => FieldKind::Vector4I32(src.read_vec_n()?),
            "vec2u32" => FieldKind::Vector2U32(src.read_vec_n()?),
            "vec3u32" => FieldKind::Vector3U32(src.read_vec_n()?),
            "vec4u32" => FieldKind::Vector4U32(src.read_vec_n()?),
            "vec2i64" => FieldKind::Vector2I64(src.read_vec_n()?),
            "vec3i64" => FieldKind::Vector3I64(src.read_vec_n()?),
            "vec4i64" => FieldKind::Vector4I64(src.read_vec_n()?),
            "vec2u64" => FieldKind::Vector2U64(src.read_vec_n()?),
            "vec3u64" => FieldKind::Vector3U64(src.read_vec_n()?),
            "vec4u64" => FieldKind::Vector4U64(src.read_vec_n()?),
            "quat" => {
                let v: Vector4<f32> = src.read_vec_n()?;
                FieldKind::UnitQuaternion(UnitQuaternion::new_normalize(Quaternion::new(
                    v.w, v.x, v.y, v.z,
                )))
            }
            "mat4" => FieldKind::Matrix4(src.read_mat_n()?),
            "data" => {
                let base64 = src.read_str_until_skip_ws(b'>')?;
                FieldKind::BinaryBlob(base64::engine::general_purpose::STANDARD.decode(base64)?)
            }
            "mat3" => FieldKind::Matrix3(src.read_mat_n()?),
            "uuid" => {
                let uuid = src.read_str_until_skip_ws(b'>')?;
                FieldKind::Uuid(Uuid::from_str(&uuid)?)
            }
            "complex" => {
                let v: Vector2<f32> = src.read_vec_n()?;
                FieldKind::UnitComplex(UnitComplex::from_complex(Complex::new(v.x, v.y)))
            }
            "podarray" => {
                let type_id: u8 = src.read_num_until(b';')?;
                let element_size: u32 = src.read_num_until(b';')?;
                let base64 = src.read_str_until_skip_ws(b'>')?;
                FieldKind::PodArray {
                    type_id,
                    element_size,
                    bytes: base64::engine::general_purpose::STANDARD.decode(base64)?,
                }
            }
            "mat2" => FieldKind::Matrix2(src.read_mat_n()?),
            "str" => {
                src.skip_until(|ch| ch != b'\"')?;
                src.skip_n(1)?;

                let mut bytes = Vec::new();

                loop {
                    let ch = src.next()?;
                    if ch == b'\"' {
                        break;
                    }
                    let next_ch = src.peek()?;
                    if ch == b'\\' && next_ch == b'\"' {
                        // Special case for quotes.
                        bytes.push(next_ch);
                        src.skip_n(1)?;
                    } else if ch == b'\\' && next_ch == b'n' {
                        // Special case for new line.
                        bytes.push(b'\n');
                        src.skip_n(1)?;
                    } else {
                        bytes.push(ch);
                    }
                }

                src.skip_until(|ch| ch != b'>')?;
                src.skip_n(1)?;

                FieldKind::String(String::from_utf8(bytes)?)
            }
            _ => return err!("unexpected field type: {}", ty),
        };

        Ok(Field { name, kind })
    }

    fn read_node(&mut self, visitor: &mut Visitor) -> Result<Handle<VisitorNode>, VisitError> {
        let src = &mut self.src;
        let name = src.read_str_until_skip_ws(b'[')?;

        let mut node = VisitorNode {
            name,
            ..VisitorNode::default()
        };

        let field_count: usize = src.read_num_until(b':')?;
        for _ in 0..field_count {
            node.fields.push(self.read_field()?);
        }

        let src = &mut self.src;

        src.skip_until(|ch| ch != b']')?;
        src.skip_n(1)?;

        src.skip_until(|ch| ch != b'{')?;
        src.skip_n(1)?;

        let child_count: usize = src.read_num_until(b':')?;
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            children.push(self.read_node(visitor)?);
        }

        let src = &mut self.src;

        src.skip_until(|ch| ch != b'}')?;
        src.skip_n(1)?;

        node.children.clone_from(&children);

        let handle = visitor.nodes.spawn(node);
        for child_handle in children.iter() {
            let child = visitor.nodes.borrow_mut(*child_handle);
            child.parent = handle;
        }

        Ok(handle)
    }

    fn read(&mut self) -> Result<Visitor, VisitError> {
        let src = &mut self.src;

        let magic: [u8; 4] = [src.next()?, src.next()?, src.next()?, src.next()?];

        let version = if magic.eq(Visitor::MAGIC_ASCII_OLD.as_bytes()) {
            VisitorVersion::Legacy as u32
        } else if magic.eq(Visitor::MAGIC_ASCII_CURRENT.as_bytes()) {
            src.skip_until(|ch| ch != b':')?;
            src.skip_n(1)?;
            src.read_num_until::<u32, _>(b';')?
        } else {
            return Err(VisitError::NotSupportedFormat);
        };

        let mut visitor = Visitor {
            nodes: Pool::new(),
            unique_id_counter: 1,
            rc_map: Default::default(),
            arc_map: Default::default(),
            reading: true,
            current_node: Handle::NONE,
            root: Handle::NONE,
            version,
            blackboard: Blackboard::new(),
            flags: VisitorFlags::NONE,
        };
        visitor.root = self.read_node(&mut visitor)?;
        visitor.current_node = visitor.root;
        Ok(visitor)
    }
}

#[cfg(test)]
mod test {
    use crate::visitor::{
        field::{Field, FieldKind},
        reader::{
            ascii::{AsciiReader, ByteReader},
            Reader,
        },
    };
    use nalgebra::{
        Complex, Matrix2, Matrix3, Matrix4, Quaternion, UnitComplex, UnitQuaternion, Vector2,
        Vector3, Vector4,
    };
    use std::io::Cursor;
    use uuid::uuid;

    #[test]
    fn test_byte_reader_methods() {
        let input = "Name 123 \t 456  789 0 \t this_is_string : other_string; ;";
        let mut cursor = Cursor::new(input);
        let mut reader = ByteReader::new(&mut cursor);
        assert_eq!(reader.peek().unwrap(), b'N');
        assert_eq!(reader.peek().unwrap(), b'N');
        assert_eq!(reader.next().unwrap(), b'N');
        assert_eq!(reader.next().unwrap(), b'a');
        assert_eq!(reader.next().unwrap(), b'm');
        assert_eq!(reader.next().unwrap(), b'e');

        reader.skip_n(1).unwrap();

        assert_eq!(reader.next().unwrap(), b'1');
        assert_eq!(reader.next().unwrap(), b'2');
        assert_eq!(reader.next().unwrap(), b'3');

        reader.skip_ws().unwrap();

        assert_eq!(reader.next().unwrap(), b'4');
        assert_eq!(reader.next().unwrap(), b'5');
        assert_eq!(reader.next().unwrap(), b'6');

        reader.skip_until(|ch| ch != b'7').unwrap();

        assert_eq!(reader.next().unwrap(), b'7');

        reader.skip_until(|ch| ch != b'8').unwrap();
        reader.skip_n(1).unwrap();

        assert_eq!(reader.next().unwrap(), b'9');

        reader.skip_ws().unwrap();
        reader.skip_ws().unwrap();

        assert_eq!(reader.next().unwrap(), b'0');

        assert_eq!(
            reader.read_str_until_skip_ws(b':').unwrap(),
            "this_is_string"
        );

        assert_eq!(reader.peek().unwrap(), b' ');

        assert_eq!(reader.read_str_until_skip_ws(b';').unwrap(), "other_string");

        assert_eq!(reader.next().unwrap(), b' ');
        assert_eq!(reader.peek().unwrap(), b';');
        reader.skip_n(1).unwrap();
        assert!(reader.next().is_err());
    }

    #[test]
    fn test_byte_reader_complex() {
        let input = "\t\t  Name<u32: \t 123 > OtherField<  f64  \t   :  123.321  >";
        let mut cursor = Cursor::new(input);
        let mut reader = ByteReader::new(&mut cursor);
        assert_eq!(
            reader.read_str_until_skip_ws(b'<').unwrap().as_str(),
            "Name"
        );
        assert_eq!(reader.read_str_until_skip_ws(b':').unwrap().as_str(), "u32");
        assert_eq!(reader.read_num_until::<u32, _>(b'>').unwrap(), 123);

        assert_eq!(
            reader.read_str_until_skip_ws(b'<').unwrap().as_str(),
            "OtherField"
        );
        assert_eq!(reader.read_str_until_skip_ws(b':').unwrap().as_str(), "f64");
        assert_eq!(reader.read_num_until::<f64, _>(b'>').unwrap(), 123.321);
    }

    #[test]
    fn test_parse_field() {
        let input = r#"
        U8<u8:123>
        I8<i8:-123>
        U16<u16:123>
        I16<i16:-123>
        U32<u32:123>
        I32<i32:-123>
        U64<u64:123>
        I64<i64:-123>
        F32<f32:123.321>
        F64<f64:123.321>
        Vec2F32<vec2f32:1.1; 2.2>
        Vec3F32<vec3f32:1.1; 2.2; 3.3>
        Vec4F32<vec4f32:1.1; 2.2; 3.3; 4.4>
        Vec2F64<vec2f64:1.1; 2.2>
        Vec3F64<vec3f64:1.1; 2.2; 3.3>
        Vec4F64<vec4f64:1.1; 2.2; 3.3; 4.4>
        Vec2I8<vec2i8:-1; -2>
        Vec2U8<vec2u8:1; 2>
        Vec2I16<vec2i16:-1; -2>
        Vec2U16<vec2u16:1; 2>
        Vec2I32<vec2i32:-1; -2>
        Vec2U32<vec2u32:1; 2>
        Vec2I64<vec2i64:-1; -2>
        Vec2U64<vec2u64:1; 2>
        Quat<quat:1.1; 2.2; 3.3; 4.4>
        Mat4<mat4:1.0;2.0;3.0;4.0;5.0;6.0;7.0;8.0;9.0;10.0;11.0;12.0;13.0;14.0;15.0;16.0>
        Mat3<mat3:1.0;2.0;3.0;4.0;5.0;6.0;7.0;8.0;9.0>
        Mat2<mat2:1.0;2.0;3.0;4.0>
        Data<data:SGVsbG8h>
        Uuid<uuid:ecdaa0b6-1867-424f-b86d-86818b9514c6>
        Complex<complex:-1.0; 2.0>
        PodArray<podarray:0; 1; SGVsbG8h>
        "#;
        let mut cursor = Cursor::new(input);
        let mut reader = AsciiReader::new(&mut cursor);

        macro_rules! read_next {
            ($name:expr => $value:expr) => {
                assert_eq!(reader.read_field().unwrap(), Field::new($name, $value));
            };
        }

        read_next!("U8" => FieldKind::U8(123));
        read_next!("I8" => FieldKind::I8(-123));
        read_next!("U16" => FieldKind::U16(123));
        read_next!("I16" => FieldKind::I16(-123));
        read_next!("U32" => FieldKind::U32(123));
        read_next!("I32" => FieldKind::I32(-123));
        read_next!("U64" => FieldKind::U64(123));
        read_next!("I64" => FieldKind::I64(-123));
        read_next!("F32" => FieldKind::F32(123.321));
        read_next!("F64" => FieldKind::F64(123.321));
        read_next!("Vec2F32" => FieldKind::Vector2F32(Vector2::new(1.1,2.2)));
        read_next!("Vec3F32" => FieldKind::Vector3F32(Vector3::new(1.1,2.2,3.3)));
        read_next!("Vec4F32" => FieldKind::Vector4F32(Vector4::new(1.1,2.2,3.3,4.4)));
        read_next!("Vec2F64" => FieldKind::Vector2F64(Vector2::new(1.1,2.2)));
        read_next!("Vec3F64" => FieldKind::Vector3F64(Vector3::new(1.1,2.2,3.3)));
        read_next!("Vec4F64" => FieldKind::Vector4F64(Vector4::new(1.1,2.2,3.3,4.4)));
        read_next!("Vec2I8" => FieldKind::Vector2I8(Vector2::new(-1,-2)));
        read_next!("Vec2U8" => FieldKind::Vector2U8(Vector2::new(1,2)));
        read_next!("Vec2I16" => FieldKind::Vector2I16(Vector2::new(-1,-2)));
        read_next!("Vec2U16" => FieldKind::Vector2U16(Vector2::new(1,2)));
        read_next!("Vec2I32" => FieldKind::Vector2I32(Vector2::new(-1,-2)));
        read_next!("Vec2U32" => FieldKind::Vector2U32(Vector2::new(1,2)));
        read_next!("Vec2I64" => FieldKind::Vector2I64(Vector2::new(-1,-2)));
        read_next!("Vec2U64" => FieldKind::Vector2U64(Vector2::new(1,2)));
        read_next!("Quat" => FieldKind::UnitQuaternion(
            // w, i, j, k
            UnitQuaternion::new_normalize(Quaternion::new(4.4, 1.1, 2.2, 3.3,)))
        );
        read_next!("Mat4" => FieldKind::Matrix4(
            // column major constructor
            Matrix4::new(1.0, 5.0, 9.0, 13.0,
                         2.0, 6.0, 10.0, 14.0,
                         3.0, 7.0, 11.0, 15.0,
                         4.0, 8.0, 12.0, 16.0)
        ));
        read_next!("Mat3" => FieldKind::Matrix3(
            // column major constructor
            Matrix3::new(1.0, 4.0, 7.0,
                         2.0, 5.0, 8.0,
                         3.0, 6.0, 9.0)
        ));
        read_next!("Mat2" => FieldKind::Matrix2(
            // column major constructor
            Matrix2::new(1.0, 3.0,
                         2.0, 4.0)
        ));
        read_next!("Data" => FieldKind::BinaryBlob(vec![b'H', b'e', b'l', b'l', b'o', b'!']));
        read_next!("Uuid" => FieldKind::Uuid(uuid!("ecdaa0b6-1867-424f-b86d-86818b9514c6")));
        read_next!("Complex" => FieldKind::UnitComplex(UnitComplex::new_normalize(Complex::new(-1.0, 2.0))));
        read_next!("PodArray" => FieldKind::PodArray{
            type_id: 0,
            element_size: 1,
            bytes: vec![b'H', b'e', b'l', b'l', b'o', b'!']
        });
    }

    #[test]
    fn test_parse_visitor() {
        let input = r#"FTAX:1;
            SomeNode
            [2:
                U8<u8:123>
                I8<i8:-123>
            ]
            {2:
                NestedNode1
                [1:
                    F32<f32:123.1>
                ]
                {0:
                }
                NestedNode2
                [1:
                    F32<f32:123.1>
                ]
                {0:
                }
            }
        "#;
        let mut cursor = Cursor::new(input);
        let mut reader = AsciiReader::new(&mut cursor);

        let visitor = reader.read().unwrap();

        assert_eq!(visitor.version, 1);

        let some_node = visitor.find_node("SomeNode").unwrap();
        assert_eq!(some_node.fields.len(), 2);
        assert_eq!(
            some_node.fields,
            vec![
                Field::new("U8", FieldKind::U8(123)),
                Field::new("I8", FieldKind::I8(-123))
            ]
        );
        assert_eq!(some_node.children.len(), 2);

        let nested_node_1 = visitor.find_node("NestedNode1").unwrap();
        assert_eq!(nested_node_1.fields.len(), 1);
        assert_eq!(
            nested_node_1.fields,
            vec![Field::new("F32", FieldKind::F32(123.1)),]
        );
        assert_eq!(nested_node_1.children.len(), 0);

        let nested_node_2 = visitor.find_node("NestedNode2").unwrap();
        assert_eq!(nested_node_2.fields.len(), 1);
        assert_eq!(
            nested_node_2.fields,
            vec![Field::new("F32", FieldKind::F32(123.1)),]
        );
        assert_eq!(nested_node_2.children.len(), 0);
    }
}
