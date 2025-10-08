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

//! Data types that can be serialized as-is, by dumping the memory into a file.

use crate::visitor::{
    error::VisitError,
    field::{Field, FieldKind},
    Visit, VisitResult, Visitor,
};

const POD_TYPES: &[&str] = &[
    "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "f32", "f64",
];

fn type_id_to_str(type_id: u8) -> &'static str {
    POD_TYPES
        .get(type_id as usize)
        .copied()
        .unwrap_or("Invalid Type")
}

/// Trait for datatypes that can be converted directly into bytes.
/// This is required for the type to be used in the Vec of a [PodVecView].
pub trait Pod: Copy {
    /// A number which distinguishes each Pod type. Two distinct Pod types must not share the same `type_id` byte.
    /// The `type_id` is stored with the data when a [PodVecView] is visited and used to confirm that the stored
    /// data matches the expected type when reading. Otherwise garbage data could be read by interpreting an
    /// array of i8 as an array of f32 or any other mismatched combination.
    fn type_id() -> u8;
}

impl Pod for u8 {
    fn type_id() -> u8 {
        0
    }
}

impl Pod for i8 {
    fn type_id() -> u8 {
        1
    }
}

impl Pod for u16 {
    fn type_id() -> u8 {
        2
    }
}

impl Pod for i16 {
    fn type_id() -> u8 {
        3
    }
}

impl Pod for u32 {
    fn type_id() -> u8 {
        4
    }
}

impl Pod for i32 {
    fn type_id() -> u8 {
        5
    }
}

impl Pod for u64 {
    fn type_id() -> u8 {
        6
    }
}

impl Pod for i64 {
    fn type_id() -> u8 {
        7
    }
}

impl Pod for f32 {
    fn type_id() -> u8 {
        8
    }
}

impl Pod for f64 {
    fn type_id() -> u8 {
        9
    }
}

/// A [Visit] type for storing a whole Vec of [Pod] values as a single field within a Visitor.
/// The Vec is reinterpreted as a Vec of bytes, with no consideration given for whether the bytes
/// are in big-endian or little-endian order by using [std::ptr::copy_nonoverlapping].
pub struct PodVecView<'a, T: Pod> {
    type_id: u8,
    vec: &'a mut Vec<T>,
}

impl<'a, T: Pod> PodVecView<'a, T> {
    /// Creates a view from the given vector.
    pub fn from_pod_vec(vec: &'a mut Vec<T>) -> Self {
        Self {
            type_id: T::type_id(),
            vec,
        }
    }
}

impl<T: Pod> Visit for PodVecView<'_, T> {
    #[allow(clippy::uninit_vec)]
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.reading {
            if let Some(field) = visitor.find_field(name) {
                match &field.kind {
                    FieldKind::PodArray {
                        type_id,
                        element_size,
                        bytes,
                    } => {
                        if *type_id == self.type_id {
                            let len = bytes.len() / *element_size as usize;
                            let mut data = Vec::<T>::with_capacity(len);
                            unsafe {
                                data.set_len(len);
                                std::ptr::copy_nonoverlapping(
                                    bytes.as_ptr(),
                                    data.as_mut_ptr() as *mut u8,
                                    bytes.len(),
                                );
                            }
                            *self.vec = data;
                            Ok(())
                        } else {
                            Err(VisitError::TypeMismatch {
                                expected: type_id_to_str(self.type_id),
                                actual: type_id_to_str(*type_id),
                            })
                        }
                    }
                    _ => Err(VisitError::FieldTypeDoesNotMatch {
                        expected: stringify!(FieldKind::PodArray),
                        actual: format!("{:?}", field.kind),
                    }),
                }
            } else {
                Err(VisitError::field_does_not_exist(name, visitor))
            }
        } else if visitor.find_field(name).is_some() {
            Err(VisitError::FieldAlreadyExists(name.to_owned()))
        } else {
            let node = visitor.current_node();
            node.fields.push(Field::new(
                name,
                FieldKind::PodArray {
                    type_id: T::type_id(),
                    element_size: std::mem::size_of::<T>() as u32,
                    bytes: unsafe {
                        let mut data = self.vec.clone();
                        let bytes = Vec::from_raw_parts(
                            data.as_mut_ptr() as *mut u8,
                            data.len() * std::mem::size_of::<T>(),
                            data.capacity() * std::mem::size_of::<T>(),
                        );
                        std::mem::forget(data);
                        bytes
                    },
                },
            ));
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::visitor::pod::PodVecView;

    #[test]
    fn pod_vec_view_from_pod_vec() {
        // Pod for u8
        let mut v = Vec::<u8>::new();
        let mut v2 = v.clone();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 0_u8);
        assert_eq!(p.vec, &mut v2);

        // Pod for i8
        let mut v = Vec::<i8>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 1_u8);

        // Pod for u16
        let mut v = Vec::<u16>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 2_u8);

        // Pod for i16
        let mut v = Vec::<i16>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 3_u8);

        // Pod for u32
        let mut v = Vec::<u32>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 4_u8);

        // Pod for i32
        let mut v = Vec::<i32>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 5_u8);

        // Pod for u64
        let mut v = Vec::<u64>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 6_u8);

        // Pod for i64
        let mut v = Vec::<i64>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 7_u8);

        // Pod for f32
        let mut v = Vec::<f32>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 8_u8);

        // Pod for f64
        let mut v = Vec::<f64>::new();
        let p = PodVecView::from_pod_vec(&mut v);
        assert_eq!(p.type_id, 9_u8);
    }
}
