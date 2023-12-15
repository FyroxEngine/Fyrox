//! Visitor is a tree-based serializer/deserializer.
//!
//! # Overview
//!
//! Visitor uses tree to create structured storage of data. Basic unit is a *node* - it is a container
//! for data fields. Each node has name, handle to parent, set of handles to children nodes and some
//! container for data fields. Data field is tuple of name and value, value can be any of simple Rust
//! types and some of basic structures of the crate. Main criteria of what could be the field and what
//! not is the ability to be represented as set of bytes without any aliasing issues.

pub use fyrox_core_derive::Visit;

pub mod prelude {
    //! Types to use `#[derive(Visit)]`
    pub use super::{Visit, VisitError, VisitResult, Visitor};
}

use crate::{
    algebra::{
        Complex, Const, Matrix, Matrix2, Matrix3, Matrix4, Quaternion, RawStorage, RawStorageMut,
        SVector, Scalar, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4, U1,
    },
    io::{self, FileLoadError},
    pool::{Handle, Pool},
    replace_slashes,
};

use base64::Engine;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fxhash::FxHashMap;
use std::any::TypeId;
use std::error::Error;
use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::{Display, Formatter},
    fs::File,
    hash::{BuildHasher, Hash},
    io::{BufWriter, Cursor, Read, Write},
    ops::{Deref, DerefMut, Range},
    path::{Path, PathBuf},
    rc::Rc,
    string::FromUtf8Error,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use uuid::Uuid;

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
    BinaryBlob(Vec<u8>),
    Matrix3(Matrix3<f32>),
    Uuid(Uuid),
    UnitComplex(UnitComplex<f32>),
    PodArray {
        type_id: u8,
        element_size: u32,
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

pub trait Pod: Copy {
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

pub struct PodVecView<'a, T: Pod> {
    type_id: u8,
    vec: &'a mut Vec<T>,
}

impl<'a, T: Pod> PodVecView<'a, T> {
    pub fn from_pod_vec(vec: &'a mut Vec<T>) -> Self {
        Self {
            type_id: T::type_id(),
            vec,
        }
    }
}

impl<'a, T: Pod> Visit for PodVecView<'a, T> {
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
                            Err(VisitError::TypeMismatch)
                        }
                    }
                    _ => Err(VisitError::FieldTypeDoesNotMatch),
                }
            } else {
                Err(VisitError::FieldDoesNotExist(name.to_owned()))
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

impl FieldKind {
    fn as_string(&self) -> String {
        match self {
            Self::Bool(data) => format!("<bool = {}>, ", data),
            Self::U8(data) => format!("<u8 = {}>, ", data),
            Self::I8(data) => format!("<i8 = {}>, ", data),
            Self::U16(data) => format!("<u16 = {}>, ", data),
            Self::I16(data) => format!("<i16 = {}>, ", data),
            Self::U32(data) => format!("<u32 = {}>, ", data),
            Self::I32(data) => format!("<i32 = {}>, ", data),
            Self::U64(data) => format!("<u64 = {}>, ", data),
            Self::I64(data) => format!("<i64 = {}>, ", data),
            Self::F32(data) => format!("<f32 = {}>, ", data),
            Self::F64(data) => format!("<f64 = {}>, ", data),
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
                    out += format!("{}; ", f).as_str();
                }
                out
            }
            Self::BinaryBlob(data) => {
                let out = match String::from_utf8(data.clone()) {
                    Ok(s) => s,
                    Err(_) => base64::engine::general_purpose::STANDARD.encode(data),
                };
                format!("<data = {}>, ", out)
            }
            Self::Matrix3(data) => {
                let mut out = String::from("<mat3 = ");
                for f in data.iter() {
                    out += format!("{}; ", f).as_str();
                }
                out
            }
            Self::Uuid(uuid) => {
                format!("<uuid = {}", uuid)
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
                format!(
                    "<podarray = {}; {}; [{}]>",
                    type_id, element_size, base64_encoded
                )
            }
            Self::Matrix2(data) => {
                let mut out = String::from("<mat2 = ");
                for f in data.iter() {
                    out += format!("{}; ", f).as_str();
                }
                out
            }
        }
    }
}

macro_rules! impl_field_data {
    ($type_name:ty, $($kind:tt)*) => {
        impl Visit for $type_name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                if visitor.reading {
                    if let Some(field) = visitor.find_field(name) {
                        match field.kind {
                            $($kind)*(data) => {
                                *self = data.clone();
                                Ok(())
                            },
                            _ => Err(VisitError::FieldTypeDoesNotMatch)
                        }
                    } else {
                        Err(VisitError::FieldDoesNotExist(name.to_owned()))
                    }
                } else if visitor.find_field(name).is_some() {
                    Err(VisitError::FieldAlreadyExists(name.to_owned()))
                } else {
                    let node = visitor.current_node();
                    node.fields.push(Field::new(name, $($kind)*(self.clone())));
                    Ok(())
                }
            }
        }
    };
}

/// Proxy struct for plain data, we can't use `Vec<u8>` directly,
/// because it will serialize each byte as separate node.
pub struct BinaryBlob<'a, T>
where
    T: Copy,
{
    pub vec: &'a mut Vec<T>,
}

impl_field_data!(u64, FieldKind::U64);
impl_field_data!(i64, FieldKind::I64);
impl_field_data!(u32, FieldKind::U32);
impl_field_data!(i32, FieldKind::I32);
impl_field_data!(u16, FieldKind::U16);
impl_field_data!(i16, FieldKind::I16);
impl_field_data!(u8, FieldKind::U8);
impl_field_data!(i8, FieldKind::I8);
impl_field_data!(f32, FieldKind::F32);
impl_field_data!(f64, FieldKind::F64);
impl_field_data!(UnitQuaternion<f32>, FieldKind::UnitQuaternion);
impl_field_data!(Matrix4<f32>, FieldKind::Matrix4);
impl_field_data!(bool, FieldKind::Bool);
impl_field_data!(Matrix3<f32>, FieldKind::Matrix3);
impl_field_data!(Uuid, FieldKind::Uuid);
impl_field_data!(UnitComplex<f32>, FieldKind::UnitComplex);
impl_field_data!(Matrix2<f32>, FieldKind::Matrix2);

impl_field_data!(Vector2<f32>, FieldKind::Vector2F32);
impl_field_data!(Vector3<f32>, FieldKind::Vector3F32);
impl_field_data!(Vector4<f32>, FieldKind::Vector4F32);

impl_field_data!(Vector2<f64>, FieldKind::Vector2F64);
impl_field_data!(Vector3<f64>, FieldKind::Vector3F64);
impl_field_data!(Vector4<f64>, FieldKind::Vector4F64);

impl_field_data!(Vector2<i8>, FieldKind::Vector2I8);
impl_field_data!(Vector3<i8>, FieldKind::Vector3I8);
impl_field_data!(Vector4<i8>, FieldKind::Vector4I8);

impl_field_data!(Vector2<u8>, FieldKind::Vector2U8);
impl_field_data!(Vector3<u8>, FieldKind::Vector3U8);
impl_field_data!(Vector4<u8>, FieldKind::Vector4U8);

impl_field_data!(Vector2<i16>, FieldKind::Vector2I16);
impl_field_data!(Vector3<i16>, FieldKind::Vector3I16);
impl_field_data!(Vector4<i16>, FieldKind::Vector4I16);

impl_field_data!(Vector2<u16>, FieldKind::Vector2U16);
impl_field_data!(Vector3<u16>, FieldKind::Vector3U16);
impl_field_data!(Vector4<u16>, FieldKind::Vector4U16);

impl_field_data!(Vector2<i32>, FieldKind::Vector2I32);
impl_field_data!(Vector3<i32>, FieldKind::Vector3I32);
impl_field_data!(Vector4<i32>, FieldKind::Vector4I32);

impl_field_data!(Vector2<u32>, FieldKind::Vector2U32);
impl_field_data!(Vector3<u32>, FieldKind::Vector3U32);
impl_field_data!(Vector4<u32>, FieldKind::Vector4U32);

impl_field_data!(Vector2<i64>, FieldKind::Vector2I64);
impl_field_data!(Vector3<i64>, FieldKind::Vector3I64);
impl_field_data!(Vector4<i64>, FieldKind::Vector4I64);

impl_field_data!(Vector2<u64>, FieldKind::Vector2U64);
impl_field_data!(Vector3<u64>, FieldKind::Vector3U64);
impl_field_data!(Vector4<u64>, FieldKind::Vector4U64);

impl<'a, T> Visit for BinaryBlob<'a, T>
where
    T: Copy,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.reading {
            if let Some(field) = visitor.find_field(name) {
                match &field.kind {
                    FieldKind::BinaryBlob(data) => {
                        let mut bytes = std::mem::ManuallyDrop::new(data.clone());

                        // SAFETY: This is kinda safe, but may cause portability issues because of various byte order.
                        // However it seems to be fine, since big-endian is pretty much dead and unused nowadays.
                        *self.vec = unsafe {
                            Vec::from_raw_parts(
                                bytes.as_mut_ptr() as *mut T,
                                bytes.len() / std::mem::size_of::<T>(),
                                bytes.capacity() / std::mem::size_of::<T>(),
                            )
                        };

                        Ok(())
                    }
                    _ => Err(VisitError::FieldTypeDoesNotMatch),
                }
            } else {
                Err(VisitError::FieldDoesNotExist(name.to_owned()))
            }
        } else if visitor.find_field(name).is_some() {
            Err(VisitError::FieldAlreadyExists(name.to_owned()))
        } else {
            let node = visitor.current_node();

            let len_bytes = self.vec.len() * std::mem::size_of::<T>();
            let mut bytes = Vec::<u8>::with_capacity(len_bytes);
            bytes.extend_from_slice(unsafe {
                std::slice::from_raw_parts(self.vec.as_ptr() as *const u8, len_bytes)
            });

            node.fields
                .push(Field::new(name, FieldKind::BinaryBlob(bytes)));

            Ok(())
        }
    }
}

pub struct Field {
    name: String,
    kind: FieldKind,
}

#[derive(Debug)]
pub enum VisitError {
    Io(std::io::Error),
    UnknownFieldType(u8),
    FieldDoesNotExist(String),
    FieldAlreadyExists(String),
    RegionAlreadyExists(String),
    InvalidCurrentNode,
    FieldTypeDoesNotMatch,
    RegionDoesNotExist(String),
    NoActiveNode,
    NotSupportedFormat,
    InvalidName,
    TypeMismatch,
    RefCellAlreadyMutableBorrowed,
    User(String),
    UnexpectedRcNullIndex,
    PoisonedMutex,
    FileLoadError(FileLoadError),
}

impl Error for VisitError {}

impl Display for VisitError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::Io(io) => write!(f, "io error: {}", io),
            Self::UnknownFieldType(type_index) => write!(f, "unknown field type {}", type_index),
            Self::FieldDoesNotExist(name) => write!(f, "field does not exists {}", name),
            Self::FieldAlreadyExists(name) => write!(f, "field already exists {}", name),
            Self::RegionAlreadyExists(name) => write!(f, "region already exists {}", name),
            Self::InvalidCurrentNode => write!(f, "invalid current node"),
            Self::FieldTypeDoesNotMatch => write!(f, "field type does not match"),
            Self::RegionDoesNotExist(name) => write!(f, "region does not exists {}", name),
            Self::NoActiveNode => write!(f, "no active node"),
            Self::NotSupportedFormat => write!(f, "not supported format"),
            Self::InvalidName => write!(f, "invalid name"),
            Self::TypeMismatch => write!(f, "type mismatch"),
            Self::RefCellAlreadyMutableBorrowed => write!(f, "ref cell already mutable borrowed"),
            Self::User(msg) => write!(f, "user defined error: {}", msg),
            Self::UnexpectedRcNullIndex => write!(f, "unexpected rc null index"),
            Self::PoisonedMutex => write!(f, "attempt to lock poisoned mutex"),
            Self::FileLoadError(e) => write!(f, "file load error: {:?}", e),
        }
    }
}

impl<T> From<std::sync::PoisonError<std::sync::MutexGuard<'_, T>>> for VisitError {
    fn from(_: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> Self {
        Self::PoisonedMutex
    }
}

impl<T> From<std::sync::PoisonError<&mut T>> for VisitError {
    fn from(_: std::sync::PoisonError<&mut T>) -> Self {
        Self::PoisonedMutex
    }
}

impl<T> From<std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, T>>> for VisitError {
    fn from(_: std::sync::PoisonError<std::sync::RwLockWriteGuard<'_, T>>) -> Self {
        Self::PoisonedMutex
    }
}

impl From<std::io::Error> for VisitError {
    fn from(io_err: std::io::Error) -> Self {
        Self::Io(io_err)
    }
}

impl From<FromUtf8Error> for VisitError {
    fn from(_: FromUtf8Error) -> Self {
        Self::InvalidName
    }
}

impl From<String> for VisitError {
    fn from(s: String) -> Self {
        Self::User(s)
    }
}

impl From<FileLoadError> for VisitError {
    fn from(e: FileLoadError) -> Self {
        Self::FileLoadError(e)
    }
}

pub type VisitResult = Result<(), VisitError>;

trait VisitableElementaryField {
    fn write(&self, file: &mut dyn Write) -> VisitResult;
    fn read(&mut self, file: &mut dyn Read) -> VisitResult;
}

macro_rules! impl_visitable_elementary_field {
    ($ty:ty, $write:ident, $read:ident $(, $endian:ident)*) => {
        impl VisitableElementaryField for $ty {
            fn write(&self, file: &mut dyn Write) -> VisitResult {
                file.$write::<$($endian)*>(*self)?;
                Ok(())
            }

            fn read(&mut self, file: &mut dyn Read) -> VisitResult {
                *self = file.$read::<$($endian)*>()?;
                Ok(())
            }
        }
    };
}
impl_visitable_elementary_field!(f64, write_f64, read_f64, LittleEndian);
impl_visitable_elementary_field!(f32, write_f32, read_f32, LittleEndian);
impl_visitable_elementary_field!(u8, write_u8, read_u8);
impl_visitable_elementary_field!(i8, write_i8, read_i8);
impl_visitable_elementary_field!(u16, write_u16, read_u16, LittleEndian);
impl_visitable_elementary_field!(i16, write_i16, read_i16, LittleEndian);
impl_visitable_elementary_field!(u32, write_u32, read_u32, LittleEndian);
impl_visitable_elementary_field!(i32, write_i32, read_i32, LittleEndian);
impl_visitable_elementary_field!(u64, write_u64, read_u64, LittleEndian);
impl_visitable_elementary_field!(i64, write_i64, read_i64, LittleEndian);

impl Field {
    pub fn new(name: &str, kind: FieldKind) -> Self {
        Self {
            name: name.to_owned(),
            kind,
        }
    }

    fn save(field: &Field, file: &mut dyn Write) -> VisitResult {
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

    fn load(file: &mut dyn Read) -> Result<Field, VisitError> {
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

    fn as_string(&self) -> String {
        format!("{}{}", self.name, self.kind.as_string())
    }
}

pub struct VisitorNode {
    name: String,
    fields: Vec<Field>,
    parent: Handle<VisitorNode>,
    children: Vec<Handle<VisitorNode>>,
}

impl VisitorNode {
    fn new(name: &str, parent: Handle<VisitorNode>) -> Self {
        Self {
            name: name.to_owned(),
            fields: Vec::new(),
            parent,
            children: Vec::new(),
        }
    }
}

impl Default for VisitorNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            fields: Vec::new(),
            parent: Handle::NONE,
            children: Vec::new(),
        }
    }
}

#[must_use = "the guard must be used"]
pub struct RegionGuard<'a>(&'a mut Visitor);

impl<'a> Deref for RegionGuard<'a> {
    type Target = Visitor;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for RegionGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a> Drop for RegionGuard<'a> {
    fn drop(&mut self) {
        // If we acquired RegionGuard instance, then it is safe to assert that
        // `leave_region` was successful.
        self.0.leave_region().unwrap();
    }
}

#[derive(Default)]
pub struct Blackboard {
    items: FxHashMap<TypeId, Arc<dyn Any>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self {
            items: Default::default(),
        }
    }

    pub fn register<T: Any>(&mut self, value: Arc<T>) {
        self.items.insert(TypeId::of::<T>(), value);
    }

    pub fn get<T: Any>(&self) -> Option<&T> {
        self.items
            .get(&TypeId::of::<T>())
            .and_then(|v| (**v).downcast_ref::<T>())
    }

    pub fn inner(&self) -> &FxHashMap<TypeId, Arc<dyn Any>> {
        &self.items
    }

    pub fn inner_mut(&mut self) -> &mut FxHashMap<TypeId, Arc<dyn Any>> {
        &mut self.items
    }
}

pub struct Visitor {
    nodes: Pool<VisitorNode>,
    rc_map: FxHashMap<u64, Rc<dyn Any>>,
    arc_map: FxHashMap<u64, Arc<dyn Any + Send + Sync>>,
    reading: bool,
    current_node: Handle<VisitorNode>,
    root: Handle<VisitorNode>,
    pub blackboard: Blackboard,
}

pub trait Visit {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult;
}

impl Default for Visitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor {
    pub const MAGIC: &'static str = "RG3D";

    pub fn new() -> Self {
        let mut nodes = Pool::new();
        let root = nodes.spawn(VisitorNode::new("__ROOT__", Handle::NONE));
        Self {
            nodes,
            rc_map: FxHashMap::default(),
            arc_map: FxHashMap::default(),
            reading: false,
            current_node: root,
            root,
            blackboard: Blackboard::new(),
        }
    }

    fn find_field(&mut self, name: &str) -> Option<&mut Field> {
        self.nodes
            .borrow_mut(self.current_node)
            .fields
            .iter_mut()
            .find(|field| field.name == name)
    }

    pub fn is_reading(&self) -> bool {
        self.reading
    }

    fn current_node(&mut self) -> &mut VisitorNode {
        self.nodes.borrow_mut(self.current_node)
    }

    pub fn enter_region(&mut self, name: &str) -> Result<RegionGuard, VisitError> {
        let node = self.nodes.borrow(self.current_node);
        if self.reading {
            let mut region = Handle::NONE;
            for child_handle in node.children.iter() {
                let child = self.nodes.borrow(*child_handle);
                if child.name == name {
                    region = *child_handle;
                    break;
                }
            }
            if region.is_some() {
                self.current_node = region;
                Ok(RegionGuard(self))
            } else {
                Err(VisitError::RegionDoesNotExist(name.to_owned()))
            }
        } else {
            // Make sure that node does not exists already.
            for child_handle in node.children.iter() {
                let child = self.nodes.borrow(*child_handle);
                if child.name == name {
                    return Err(VisitError::RegionAlreadyExists(name.to_owned()));
                }
            }

            let node_handle = self.nodes.spawn(VisitorNode::new(name, self.current_node));
            self.nodes
                .borrow_mut(self.current_node)
                .children
                .push(node_handle);
            self.current_node = node_handle;

            Ok(RegionGuard(self))
        }
    }

    pub fn current_region(&self) -> Option<&str> {
        self.nodes
            .try_borrow(self.current_node)
            .map(|n| n.name.as_str())
    }

    fn leave_region(&mut self) -> VisitResult {
        self.current_node = self.nodes.borrow(self.current_node).parent;
        if self.current_node.is_none() {
            Err(VisitError::NoActiveNode)
        } else {
            Ok(())
        }
    }

    fn print_node(
        &self,
        node_handle: Handle<VisitorNode>,
        nesting: usize,
        out_string: &mut String,
    ) {
        let offset = (0..nesting).map(|_| "\t").collect::<String>();
        let node = self.nodes.borrow(node_handle);
        *out_string += format!(
            "{}{}[Fields={}, Children={}]: ",
            offset,
            node.name,
            node.fields.len(),
            node.children.len()
        )
        .as_str();
        for field in node.fields.iter() {
            *out_string += field.as_string().as_str();
        }

        *out_string += "\n";

        for child_handle in node.children.iter() {
            self.print_node(*child_handle, nesting + 1, out_string);
        }
    }

    pub fn save_text(&self) -> String {
        let mut out_string = String::new();
        self.print_node(self.root, 0, &mut out_string);
        out_string
    }

    pub fn save_binary_to_memory<W: Write>(&self, mut writer: W) -> VisitResult {
        writer.write_all(Self::MAGIC.as_bytes())?;
        let mut stack = vec![self.root];
        while let Some(node_handle) = stack.pop() {
            let node = self.nodes.borrow(node_handle);
            let name = node.name.as_bytes();
            writer.write_u32::<LittleEndian>(name.len() as u32)?;
            writer.write_all(name)?;

            writer.write_u32::<LittleEndian>(node.fields.len() as u32)?;
            for field in node.fields.iter() {
                Field::save(field, &mut writer)?
            }

            writer.write_u32::<LittleEndian>(node.children.len() as u32)?;
            stack.extend_from_slice(&node.children);
        }
        Ok(())
    }

    pub fn save_binary_to_vec(&self) -> Result<Vec<u8>, VisitError> {
        let mut writer = Cursor::new(Vec::new());
        self.save_binary_to_memory(&mut writer)?;
        Ok(writer.into_inner())
    }

    pub fn save_binary<P: AsRef<Path>>(&self, path: P) -> VisitResult {
        let writer = BufWriter::new(File::create(path)?);
        self.save_binary_to_memory(writer)
    }

    fn load_node_binary(&mut self, file: &mut dyn Read) -> Result<Handle<VisitorNode>, VisitError> {
        let name_len = file.read_u32::<LittleEndian>()? as usize;
        let mut raw_name = vec![Default::default(); name_len];
        file.read_exact(raw_name.as_mut_slice())?;

        let mut node = VisitorNode {
            name: String::from_utf8(raw_name)?,
            ..VisitorNode::default()
        };

        let field_count = file.read_u32::<LittleEndian>()? as usize;
        for _ in 0..field_count {
            let field = Field::load(file)?;
            node.fields.push(field);
        }

        let child_count = file.read_u32::<LittleEndian>()? as usize;
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
            children.push(self.load_node_binary(file)?);
        }

        node.children = children.clone();

        let handle = self.nodes.spawn(node);
        for child_handle in children.iter() {
            let child = self.nodes.borrow_mut(*child_handle);
            child.parent = handle;
        }

        Ok(handle)
    }

    pub async fn load_binary<P: AsRef<Path>>(path: P) -> Result<Self, VisitError> {
        Self::load_from_memory(&io::load_file(path).await?)
    }

    pub fn load_from_memory(data: &[u8]) -> Result<Self, VisitError> {
        let mut reader = Cursor::new(data);
        let mut magic: [u8; 4] = Default::default();
        reader.read_exact(&mut magic)?;
        if !magic.eq(Self::MAGIC.as_bytes()) {
            return Err(VisitError::NotSupportedFormat);
        }
        let mut visitor = Self {
            nodes: Pool::new(),
            rc_map: Default::default(),
            arc_map: Default::default(),
            reading: true,
            current_node: Handle::NONE,
            root: Handle::NONE,
            blackboard: Blackboard::new(),
        };
        visitor.root = visitor.load_node_binary(&mut reader)?;
        visitor.current_node = visitor.root;
        Ok(visitor)
    }
}

impl<T> Visit for RefCell<T>
where
    T: Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if let Ok(mut data) = self.try_borrow_mut() {
            data.visit(name, visitor)
        } else {
            Err(VisitError::RefCellAlreadyMutableBorrowed)
        }
    }
}

impl<T> Visit for Vec<T>
where
    T: Default + Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut len = self.len() as u32;
        len.visit("Length", &mut region)?;

        if region.reading {
            self.clear();
            for index in 0..len {
                let region_name = format!("Item{}", index);
                let mut region = region.enter_region(region_name.as_str())?;
                let mut object = T::default();
                object.visit("ItemData", &mut region)?;
                self.push(object);
            }
        } else {
            for (index, item) in self.iter_mut().enumerate() {
                let region_name = format!("Item{}", index);
                let mut region = region.enter_region(region_name.as_str())?;
                item.visit("ItemData", &mut region)?;
            }
        }

        Ok(())
    }
}

impl<T> Visit for Option<T>
where
    T: Default + Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut is_some = u8::from(self.is_some());
        is_some.visit("IsSome", &mut region)?;

        if is_some != 0 {
            if region.reading {
                let mut value = T::default();
                value.visit("Data", &mut region)?;
                *self = Some(value);
            } else {
                self.as_mut().unwrap().visit("Data", &mut region)?;
            }
        }

        Ok(())
    }
}

impl Visit for String {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut len = self.as_bytes().len() as u32;
        len.visit("Length", &mut region)?;

        let mut data = if region.reading {
            Vec::new()
        } else {
            Vec::from(self.as_bytes())
        };

        let mut proxy = BinaryBlob { vec: &mut data };
        proxy.visit("Data", &mut region)?;

        if region.reading {
            *self = String::from_utf8(data)?;
        }
        Ok(())
    }
}

impl Visit for PathBuf {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        // We have to replace Windows back slashes \ to forward / to make paths portable
        // across all OSes.
        let portable_path = replace_slashes(&self);

        let bytes = if let Some(path_str) = portable_path.as_os_str().to_str() {
            path_str.as_bytes()
        } else {
            return Err(VisitError::InvalidName);
        };

        let mut len = bytes.len() as u32;
        len.visit("Length", &mut region)?;

        let mut data = if region.reading {
            Vec::new()
        } else {
            Vec::from(bytes)
        };

        let mut proxy = BinaryBlob { vec: &mut data };
        proxy.visit("Data", &mut region)?;

        if region.reading {
            *self = PathBuf::from(String::from_utf8(data)?);
        }

        Ok(())
    }
}

impl<T> Visit for Cell<T>
where
    T: Copy + Clone + Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut value = self.get();
        value.visit(name, visitor)?;
        if visitor.is_reading() {
            self.set(value);
        }
        Ok(())
    }
}

impl<T> Visit for Rc<T>
where
    T: Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.reading {
            let mut raw = 0u64;
            raw.visit("Id", &mut region)?;
            if raw == 0 {
                return Err(VisitError::UnexpectedRcNullIndex);
            }
            if let Some(ptr) = region.rc_map.get(&raw) {
                if let Ok(res) = Rc::downcast::<T>(ptr.clone()) {
                    *self = res;
                } else {
                    return Err(VisitError::TypeMismatch);
                }
            } else {
                // Remember that we already visited data Rc store.
                region.rc_map.insert(raw, self.clone());

                let raw = rc_to_raw(self);
                unsafe { &mut *raw }.visit("RcData", &mut region)?;
            }
        } else {
            // Take raw pointer to inner data.
            let raw = rc_to_raw(self);

            // Save it as id.
            let mut index = raw as u64;
            index.visit("Id", &mut region)?;

            if let Entry::Vacant(entry) = region.rc_map.entry(index) {
                entry.insert(self.clone());
                unsafe { &mut *raw }.visit("RcData", &mut region)?;
            }
        }

        Ok(())
    }
}

impl<T> Visit for Mutex<T>
where
    T: Visit + Send,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.get_mut()?.visit(name, visitor)
    }
}

impl<T> Visit for parking_lot::Mutex<T>
where
    T: Visit + Send,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.get_mut().visit(name, visitor)
    }
}

impl<T> Visit for Box<T>
where
    T: Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.deref_mut().visit(name, visitor)
    }
}

impl<T> Visit for RwLock<T>
where
    T: Visit + Send,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.write()?.visit(name, visitor)
    }
}

fn arc_to_raw<T>(arc: &Arc<T>) -> *mut T {
    &**arc as *const T as *mut T
}

fn rc_to_raw<T>(rc: &Rc<T>) -> *mut T {
    &**rc as *const T as *mut T
}

impl<T> Visit for Arc<T>
where
    T: Visit + Send + Sync + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.reading {
            let mut raw = 0u64;
            raw.visit("Id", &mut region)?;
            if raw == 0 {
                return Err(VisitError::UnexpectedRcNullIndex);
            }
            if let Some(ptr) = &mut region.arc_map.get(&raw) {
                if let Ok(res) = Arc::downcast::<T>(ptr.clone()) {
                    *self = res;
                } else {
                    return Err(VisitError::TypeMismatch);
                }
            } else {
                // Remember that we already visited data Rc store.
                region.arc_map.insert(raw, self.clone());

                let raw = arc_to_raw(self);
                unsafe { &mut *raw }.visit("ArcData", &mut region)?;
            }
        } else {
            // Take raw pointer to inner data.
            let raw = arc_to_raw(self);

            // Save it as id.
            let mut index = raw as u64;
            index.visit("Id", &mut region)?;

            if let Entry::Vacant(entry) = region.arc_map.entry(index) {
                entry.insert(self.clone());
                unsafe { &mut *raw }.visit("ArcData", &mut region)?;
            }
        }

        Ok(())
    }
}

impl<T> Visit for std::rc::Weak<T>
where
    T: Default + Visit + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.reading {
            let mut raw = 0u64;
            raw.visit("Id", &mut region)?;

            if raw != 0 {
                if let Some(ptr) = &mut region.rc_map.get(&raw) {
                    if let Ok(res) = Rc::downcast::<T>(ptr.clone()) {
                        *self = Rc::downgrade(&res);
                    } else {
                        return Err(VisitError::TypeMismatch);
                    }
                } else {
                    // Create new value wrapped into Rc and deserialize it.
                    let rc = Rc::new(T::default());
                    region.rc_map.insert(raw, rc.clone());

                    let raw = rc_to_raw(&rc);
                    unsafe { &mut *raw }.visit("RcData", &mut region)?;

                    *self = Rc::downgrade(&rc);
                }
            }
        } else if let Some(rc) = std::rc::Weak::upgrade(self) {
            // Take raw pointer to inner data.
            let raw = rc_to_raw(&rc);

            // Save it as id.
            let mut index = raw as u64;
            index.visit("Id", &mut region)?;

            if let Entry::Vacant(entry) = region.rc_map.entry(index) {
                entry.insert(rc);
                unsafe { &mut *raw }.visit("RcData", &mut region)?;
            }
        } else {
            let mut index = 0u64;
            index.visit("Id", &mut region)?;
        }

        Ok(())
    }
}

impl<T> Visit for std::sync::Weak<T>
where
    T: Default + Visit + Send + Sync + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.reading {
            let mut raw = 0u64;
            raw.visit("Id", &mut region)?;

            if raw != 0 {
                if let Some(ptr) = region.arc_map.get(&raw) {
                    if let Ok(res) = Arc::downcast::<T>(ptr.clone()) {
                        *self = Arc::downgrade(&res);
                    } else {
                        return Err(VisitError::TypeMismatch);
                    }
                } else {
                    // Create new value wrapped into Arc and deserialize it.
                    let arc = Arc::new(T::default());
                    region.arc_map.insert(raw, arc.clone());

                    let raw = arc_to_raw(&arc);
                    unsafe { &mut *raw }.visit("ArcData", &mut region)?;

                    *self = Arc::downgrade(&arc);
                }
            }
        } else if let Some(arc) = std::sync::Weak::upgrade(self) {
            // Take raw pointer to inner data.
            let raw = arc_to_raw(&arc);

            // Save it as id.
            let mut index = raw as u64;
            index.visit("Id", &mut region)?;

            if let Entry::Vacant(entry) = region.arc_map.entry(index) {
                entry.insert(arc);
                unsafe { &mut *raw }.visit("ArcData", &mut region)?;
            }
        } else {
            let mut index = 0u64;
            index.visit("Id", &mut region)?;
        }

        Ok(())
    }
}

impl<K, V, S> Visit for HashMap<K, V, S>
where
    K: Visit + Default + Clone + Hash + Eq,
    V: Visit + Default,
    S: BuildHasher + Clone,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut count = self.len() as u32;
        count.visit("Count", &mut region)?;

        if region.is_reading() {
            self.clear();
            for i in 0..(count as usize) {
                let name = format!("Item{}", i);

                let mut region = region.enter_region(name.as_str())?;

                let mut key = K::default();
                key.visit("Key", &mut region)?;

                let mut value = V::default();
                value.visit("Value", &mut region)?;

                self.insert(key, value);
            }
        } else {
            for (i, (key, value)) in self.iter_mut().enumerate() {
                let name = format!("Item{}", i);

                let mut region = region.enter_region(name.as_str())?;

                let mut key = key.clone();
                key.visit("Key", &mut region)?;

                value.visit("Value", &mut region)?;
            }
        }

        Ok(())
    }
}

impl<K, S> Visit for HashSet<K, S>
where
    K: Visit + Default + Clone + Hash + Eq,
    S: BuildHasher + Clone,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut count = self.len() as u32;
        count.visit("Count", &mut region)?;

        if region.is_reading() {
            self.clear();
            for i in 0..(count as usize) {
                let name = format!("Item{}", i);

                let mut region = region.enter_region(name.as_str())?;

                let mut key = K::default();
                key.visit("Key", &mut region)?;

                self.insert(key);
            }
        } else {
            for (i, mut key) in self.clone().into_iter().enumerate() {
                let name = format!("Item{}", i);

                let mut region = region.enter_region(name.as_str())?;

                key.visit("Key", &mut region)?;
            }
        }

        Ok(())
    }
}

impl<T: Default + Visit, const SIZE: usize> Visit for [T; SIZE] {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut len = SIZE as u32;
        len.visit("Length", &mut region)?;

        if region.reading {
            if len > SIZE as u32 {
                return VisitResult::Err(VisitError::User(format!(
                    "Not enough space in static array, got {}, needed {}!",
                    len, SIZE
                )));
            }

            for index in 0..len {
                let region_name = format!("Item{}", index);
                let mut region = region.enter_region(region_name.as_str())?;
                let mut object = T::default();
                object.visit("ItemData", &mut region)?;
                self[index as usize] = object;
            }
        } else {
            for (index, item) in self.iter_mut().enumerate() {
                let region_name = format!("Item{}", index);
                let mut region = region.enter_region(region_name.as_str())?;
                item.visit("ItemData", &mut region)?;
            }
        }

        Ok(())
    }
}

impl Visit for Duration {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut secs: u64 = self.as_secs();
        let mut nanos: u32 = self.subsec_nanos();

        secs.visit("Secs", &mut region)?;
        nanos.visit("Nanos", &mut region)?;

        if region.is_reading() {
            *self = Duration::new(secs, nanos);
        }

        Ok(())
    }
}

impl Visit for char {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut bytes = *self as u32;
        bytes.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = char::from_u32(bytes).unwrap();
        }
        Ok(())
    }
}

impl<T: Visit> Visit for Range<T> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.start.visit("Start", &mut region)?;
        self.end.visit("End", &mut region)?;

        Ok(())
    }
}

impl Visit for usize {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut this = *self as u64;
        this.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = this as usize;
        }
        Ok(())
    }
}

impl Visit for isize {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut this = *self as i64;
        this.visit(name, visitor)?;
        if visitor.is_reading() {
            *self = this as isize;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::visitor::{BinaryBlob, Visit, VisitResult, Visitor};
    use std::{fs::File, io::Write, path::Path, rc::Rc};

    use super::*;

    #[derive(Visit, Default)]
    pub struct Model {
        data: u64,
    }

    #[derive(Default)]
    pub struct Texture {
        data: Vec<u8>,
    }

    impl Visit for Texture {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            let mut region = visitor.enter_region(name)?;
            let mut proxy = BinaryBlob {
                vec: &mut self.data,
            };
            proxy.visit("Data", &mut region)?;
            Ok(())
        }
    }

    #[allow(dead_code)]
    #[derive(Visit)]
    pub enum ResourceKind {
        Unknown,
        Model(Model),
        Texture(Texture),
    }

    impl Default for ResourceKind {
        fn default() -> Self {
            Self::Unknown
        }
    }

    #[derive(Visit)]
    struct Resource {
        kind: ResourceKind,
        data: u16,
    }

    impl Resource {
        fn new(kind: ResourceKind) -> Self {
            Self { kind, data: 0 }
        }
    }

    impl Default for Resource {
        fn default() -> Self {
            Self {
                kind: ResourceKind::Unknown,
                data: 0,
            }
        }
    }

    #[derive(Default, Visit)]
    struct Foo {
        bar: u64,
        shared_resource: Option<Rc<Resource>>,
    }

    impl Foo {
        fn new(resource: Rc<Resource>) -> Self {
            Self {
                bar: 123,
                shared_resource: Some(resource),
            }
        }
    }

    #[test]
    fn visitor_test() {
        let path = Path::new("test.bin");

        // Save
        {
            let mut visitor = Visitor::new();
            let mut resource = Rc::new(Resource::new(ResourceKind::Model(Model { data: 555 })));
            resource.visit("SharedResource", &mut visitor).unwrap();

            let mut objects = vec![Foo::new(resource.clone()), Foo::new(resource)];

            objects.visit("Objects", &mut visitor).unwrap();

            visitor.save_binary(path).unwrap();
            if let Ok(mut file) = File::create(Path::new("test.txt")) {
                file.write_all(visitor.save_text().as_bytes()).unwrap();
            }
        }

        // Load
        {
            let mut visitor = futures::executor::block_on(Visitor::load_binary(path)).unwrap();
            let mut resource: Rc<Resource> = Rc::new(Default::default());
            resource.visit("SharedResource", &mut visitor).unwrap();

            let mut objects: Vec<Foo> = Vec::new();
            objects.visit("Objects", &mut visitor).unwrap();
        }
    }

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

    #[test]
    fn field_kind_as_string() {
        assert_eq!(
            FieldKind::Bool(true).as_string(),
            "<bool = true>, ".to_string()
        );
        assert_eq!(
            FieldKind::BinaryBlob(Vec::<u8>::new()).as_string(),
            "<data = >, ".to_string()
        );

        assert_eq!(FieldKind::F32(0.0).as_string(), "<f32 = 0>, ".to_string());
        assert_eq!(FieldKind::F64(0.0).as_string(), "<f64 = 0>, ".to_string());

        assert_eq!(FieldKind::I8(0).as_string(), "<i8 = 0>, ".to_string());
        assert_eq!(FieldKind::I16(0).as_string(), "<i16 = 0>, ".to_string());
        assert_eq!(FieldKind::I32(0).as_string(), "<i32 = 0>, ".to_string());
        assert_eq!(FieldKind::I64(0).as_string(), "<i64 = 0>, ".to_string());

        assert_eq!(
            FieldKind::Matrix2(Matrix2::default()).as_string(),
            "<mat2 = 0; 0; 0; 0; ".to_string()
        );
        assert_eq!(
            FieldKind::Matrix3(Matrix3::default()).as_string(),
            "<mat3 = 0; 0; 0; 0; 0; 0; 0; 0; 0; ".to_string()
        );
        assert_eq!(
            FieldKind::Matrix4(Matrix4::default()).as_string(),
            "<mat4 = 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; 0; ".to_string()
        );
        assert_eq!(
            FieldKind::PodArray {
                type_id: 0,
                element_size: 0,
                bytes: Vec::new()
            }
            .as_string(),
            "<podarray = 0; 0; []>".to_string()
        );

        assert_eq!(FieldKind::U8(0).as_string(), "<u8 = 0>, ".to_string());
        assert_eq!(FieldKind::U16(0).as_string(), "<u16 = 0>, ".to_string());
        assert_eq!(FieldKind::U32(0).as_string(), "<u32 = 0>, ".to_string());
        assert_eq!(FieldKind::U64(0).as_string(), "<u64 = 0>, ".to_string());

        assert_eq!(
            FieldKind::UnitComplex(UnitComplex::default()).as_string(),
            "<complex = 1; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::UnitQuaternion(UnitQuaternion::default()).as_string(),
            "<quat = 0; 0; 0; 1>, ".to_string()
        );
        assert_eq!(
            FieldKind::Uuid(Uuid::default()).as_string(),
            "<uuid = 00000000-0000-0000-0000-000000000000".to_string()
        );

        assert_eq!(
            FieldKind::Vector2F32(Vector2::new(0.0, 0.0)).as_string(),
            "<vec2f32 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2F64(Vector2::new(0.0, 0.0)).as_string(),
            "<vec2f64 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2U8(Vector2::new(0, 0)).as_string(),
            "<vec2u8 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2U16(Vector2::new(0, 0)).as_string(),
            "<vec2u16 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2U32(Vector2::new(0, 0)).as_string(),
            "<vec2u32 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2U64(Vector2::new(0, 0)).as_string(),
            "<vec2u64 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2I8(Vector2::new(0, 0)).as_string(),
            "<vec2i8 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2I16(Vector2::new(0, 0)).as_string(),
            "<vec2i16 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2I32(Vector2::new(0, 0)).as_string(),
            "<vec2i32 = 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector2I64(Vector2::new(0, 0)).as_string(),
            "<vec2i64 = 0; 0>, ".to_string()
        );

        assert_eq!(
            FieldKind::Vector3F32(Vector3::new(0.0, 0.0, 0.0)).as_string(),
            "<vec3f32 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3F64(Vector3::new(0.0, 0.0, 0.0)).as_string(),
            "<vec3f64 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3U8(Vector3::new(0, 0, 0)).as_string(),
            "<vec3u8 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3U16(Vector3::new(0, 0, 0)).as_string(),
            "<vec3u16 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3U32(Vector3::new(0, 0, 0)).as_string(),
            "<vec3u32 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3U64(Vector3::new(0, 0, 0)).as_string(),
            "<vec3u64 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3I8(Vector3::new(0, 0, 0)).as_string(),
            "<vec3i8 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3I16(Vector3::new(0, 0, 0)).as_string(),
            "<vec3i16 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3I32(Vector3::new(0, 0, 0)).as_string(),
            "<vec3i32 = 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector3I64(Vector3::new(0, 0, 0)).as_string(),
            "<vec3i64 = 0; 0; 0>, ".to_string()
        );

        assert_eq!(
            FieldKind::Vector4F32(Vector4::new(0.0, 0.0, 0.0, 0.0)).as_string(),
            "<vec4f32 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4F64(Vector4::new(0.0, 0.0, 0.0, 0.0)).as_string(),
            "<vec4f64 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4U8(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4u8 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4U16(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4u16 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4U32(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4u32 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4U64(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4u64 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4I8(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4i8 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4I16(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4i16 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4I32(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4i32 = 0; 0; 0; 0>, ".to_string()
        );
        assert_eq!(
            FieldKind::Vector4I64(Vector4::new(0, 0, 0, 0)).as_string(),
            "<vec4i64 = 0; 0; 0; 0>, ".to_string()
        );
    }
}
