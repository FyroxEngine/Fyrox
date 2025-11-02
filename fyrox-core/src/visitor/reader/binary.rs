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

use crate::{
    pool::{Handle, Pool},
    visitor::{
        blackboard::Blackboard,
        error::VisitError,
        field::{Field, FieldKind},
        reader::Reader,
        VisitableElementaryField, Visitor, VisitorFlags, VisitorNode,
    },
};
use byteorder::{LittleEndian, ReadBytesExt};
use nalgebra::{
    Complex, Const, Matrix, Matrix2, Matrix3, Matrix4, Quaternion, RawStorage, RawStorageMut,
    Scalar, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4, U1,
};
use std::io::Read;
use uuid::Uuid;

pub struct BinaryReader<'a> {
    src: &'a mut dyn Read,
}

impl<'a> BinaryReader<'a> {
    pub fn new(src: &'a mut dyn Read) -> Self {
        Self { src }
    }
}

fn read_vec_n<T, S, const N: usize>(
    src: &mut dyn Read,
) -> Result<Matrix<T, Const<N>, U1, S>, VisitError>
where
    T: VisitableElementaryField + Scalar + Default,
    S: RawStorage<T, Const<N>> + RawStorageMut<T, Const<N>> + Default,
{
    let mut vec = Matrix::<T, Const<N>, U1, S>::default();
    for v in vec.iter_mut() {
        v.read(src)?;
    }
    Ok(vec)
}

impl Reader for BinaryReader<'_> {
    fn read_field(&mut self) -> Result<Field, VisitError> {
        let src = &mut *self.src;
        let name_len = src.read_u32::<LittleEndian>()? as usize;
        let mut raw_name = vec![Default::default(); name_len];
        src.read_exact(raw_name.as_mut_slice())?;
        let id = src.read_u8()?;
        Ok(Field::new(
            String::from_utf8(raw_name)?.as_str(),
            match id {
                1 => FieldKind::U8(src.read_u8()?),
                2 => FieldKind::I8(src.read_i8()?),
                3 => FieldKind::U16(src.read_u16::<LittleEndian>()?),
                4 => FieldKind::I16(src.read_i16::<LittleEndian>()?),
                5 => FieldKind::U32(src.read_u32::<LittleEndian>()?),
                6 => FieldKind::I32(src.read_i32::<LittleEndian>()?),
                7 => FieldKind::U64(src.read_u64::<LittleEndian>()?),
                8 => FieldKind::I64(src.read_i64::<LittleEndian>()?),
                9 => FieldKind::F32(src.read_f32::<LittleEndian>()?),
                10 => FieldKind::F64(src.read_f64::<LittleEndian>()?),
                11 => FieldKind::Vector3F32({
                    let x = src.read_f32::<LittleEndian>()?;
                    let y = src.read_f32::<LittleEndian>()?;
                    let z = src.read_f32::<LittleEndian>()?;
                    Vector3::new(x, y, z)
                }),
                12 => FieldKind::UnitQuaternion({
                    let x = src.read_f32::<LittleEndian>()?;
                    let y = src.read_f32::<LittleEndian>()?;
                    let z = src.read_f32::<LittleEndian>()?;
                    let w = src.read_f32::<LittleEndian>()?;
                    UnitQuaternion::new_normalize(Quaternion::new(w, x, y, z))
                }),
                13 => FieldKind::Matrix4({
                    let mut f = [0.0f32; 16];
                    for n in &mut f {
                        *n = src.read_f32::<LittleEndian>()?;
                    }
                    Matrix4::from_row_slice(&f)
                }),
                14 => FieldKind::BinaryBlob({
                    let len = src.read_u32::<LittleEndian>()? as usize;
                    let mut vec = vec![Default::default(); len];
                    src.read_exact(vec.as_mut_slice())?;
                    vec
                }),
                15 => FieldKind::Bool(src.read_u8()? != 0),
                16 => FieldKind::Matrix3({
                    let mut f = [0.0f32; 9];
                    for n in &mut f {
                        *n = src.read_f32::<LittleEndian>()?;
                    }
                    Matrix3::from_row_slice(&f)
                }),
                17 => FieldKind::Vector2F32({
                    let x = src.read_f32::<LittleEndian>()?;
                    let y = src.read_f32::<LittleEndian>()?;
                    Vector2::new(x, y)
                }),
                18 => FieldKind::Vector4F32({
                    let x = src.read_f32::<LittleEndian>()?;
                    let y = src.read_f32::<LittleEndian>()?;
                    let z = src.read_f32::<LittleEndian>()?;
                    let w = src.read_f32::<LittleEndian>()?;
                    Vector4::new(x, y, z, w)
                }),
                19 => FieldKind::Uuid({
                    let mut bytes = uuid::Bytes::default();
                    src.read_exact(&mut bytes)?;
                    Uuid::from_bytes(bytes)
                }),
                20 => FieldKind::UnitComplex({
                    let re = src.read_f32::<LittleEndian>()?;
                    let im = src.read_f32::<LittleEndian>()?;
                    UnitComplex::from_complex(Complex::new(re, im))
                }),
                21 => {
                    let type_id = src.read_u8()?;
                    let element_size = src.read_u32::<LittleEndian>()?;
                    let data_size = src.read_u64::<LittleEndian>()?;
                    let mut bytes = vec![0; data_size as usize];
                    src.read_exact(&mut bytes)?;
                    FieldKind::PodArray {
                        type_id,
                        element_size,
                        bytes,
                    }
                }
                22 => FieldKind::Matrix2({
                    let mut f = [0.0f32; 4];
                    for n in &mut f {
                        *n = src.read_f32::<LittleEndian>()?;
                    }
                    Matrix2::from_row_slice(&f)
                }),
                23 => FieldKind::Vector2F64(read_vec_n(src)?),
                24 => FieldKind::Vector3F64(read_vec_n(src)?),
                25 => FieldKind::Vector4F64(read_vec_n(src)?),

                26 => FieldKind::Vector2I8(read_vec_n(src)?),
                27 => FieldKind::Vector3I8(read_vec_n(src)?),
                28 => FieldKind::Vector4I8(read_vec_n(src)?),

                29 => FieldKind::Vector2U8(read_vec_n(src)?),
                30 => FieldKind::Vector3U8(read_vec_n(src)?),
                31 => FieldKind::Vector4U8(read_vec_n(src)?),

                32 => FieldKind::Vector2I16(read_vec_n(src)?),
                33 => FieldKind::Vector3I16(read_vec_n(src)?),
                34 => FieldKind::Vector4I16(read_vec_n(src)?),

                35 => FieldKind::Vector2U16(read_vec_n(src)?),
                36 => FieldKind::Vector3U16(read_vec_n(src)?),
                37 => FieldKind::Vector4U16(read_vec_n(src)?),

                38 => FieldKind::Vector2I32(read_vec_n(src)?),
                39 => FieldKind::Vector3I32(read_vec_n(src)?),
                40 => FieldKind::Vector4I32(read_vec_n(src)?),

                41 => FieldKind::Vector2U32(read_vec_n(src)?),
                42 => FieldKind::Vector3U32(read_vec_n(src)?),
                43 => FieldKind::Vector4U32(read_vec_n(src)?),

                44 => FieldKind::Vector2I64(read_vec_n(src)?),
                45 => FieldKind::Vector3I64(read_vec_n(src)?),
                46 => FieldKind::Vector4I64(read_vec_n(src)?),

                47 => FieldKind::Vector2U64(read_vec_n(src)?),
                48 => FieldKind::Vector3U64(read_vec_n(src)?),
                49 => FieldKind::Vector4U64(read_vec_n(src)?),
                50 => FieldKind::String({
                    let len = src.read_u32::<LittleEndian>()? as usize;
                    let mut vec = vec![Default::default(); len];
                    src.read_exact(vec.as_mut_slice())?;
                    String::from_utf8(vec)?
                }),

                _ => return Err(VisitError::UnknownFieldType(id)),
            },
        ))
    }

    fn read_node(&mut self, visitor: &mut Visitor) -> Result<Handle<VisitorNode>, VisitError> {
        let src = &mut *self.src;

        let name_len = src.read_u32::<LittleEndian>()? as usize;
        let mut raw_name = vec![Default::default(); name_len];
        src.read_exact(raw_name.as_mut_slice())?;

        let mut node = VisitorNode {
            name: String::from_utf8(raw_name)?,
            ..VisitorNode::default()
        };

        let field_count = src.read_u32::<LittleEndian>()? as usize;
        for _ in 0..field_count {
            let field = self.read_field()?;
            node.fields.push(field);
        }

        let src = &mut *self.src;

        let child_count = src.read_u32::<LittleEndian>()? as usize;
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            children.push(self.read_node(visitor)?);
        }

        node.children.clone_from(&children);

        let handle = visitor.nodes.spawn(node);
        for child_handle in children.iter() {
            let child = visitor.nodes.borrow_mut(*child_handle);
            child.parent = handle;
        }

        Ok(handle)
    }

    fn read(&mut self) -> Result<Visitor, VisitError> {
        let src = &mut *self.src;

        let mut magic: [u8; 4] = Default::default();
        src.read_exact(&mut magic)?;

        let version = if magic.eq(Visitor::MAGIC_BINARY_CURRENT.as_bytes()) {
            src.read_u32::<LittleEndian>()?
        } else {
            return Err(VisitError::NotSupportedFormat);
        };

        let mut visitor = Visitor {
            nodes: Pool::new(),
            unique_id_counter: 1,
            type_name_map: Default::default(),
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
