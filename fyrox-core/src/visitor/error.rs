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

//! Possible errors that may occur during serialization/deserialization.

use crate::io::FileError;
use crate::visitor::Visitor;
use base64::DecodeError;
use std::num::{ParseFloatError, ParseIntError};
use std::{
    error::Error,
    fmt::{Display, Formatter},
    string::FromUtf8Error,
};

/// Errors that may occur while reading or writing [`crate::visitor::Visitor`].
#[derive(Debug)]
pub enum VisitError {
    /// An error that occured for multiple reasons, when there are multiple potential ways
    /// to visit a node, and all of them lead to errors.
    Multiple(Vec<VisitError>),
    /// An [std::io::Error] occured while reading or writing a file with Visitor data.
    Io(std::io::Error),
    /// When a field is encoded as bytes, the field data is prefixed by an identifying byte
    /// to allow the bytes to be decoded. This error happens when an identifying byte is
    /// expected during decoding, but an unknown value is found in that byte.
    UnknownFieldType(u8),
    /// Attempting to visit a field on a read-mode Visitor when no field in the visitor data
    /// has the given name.
    FieldDoesNotExist(String),
    /// Attempting to visit a field on a write-mode Visitor when a field already has the
    /// given name.
    FieldAlreadyExists(String),
    /// Attempting to enter a region on a write-mode Visitor when a region already has the
    /// given name.
    RegionAlreadyExists(String),
    /// Current node handle is invalid and does not lead to a real node.
    InvalidCurrentNode,
    /// Attempting to visit a field using a read-mode Visitor when that field was originally
    /// written using a value of a different type.
    FieldTypeDoesNotMatch {
        /// expected [`crate::visitor::FieldKind`] variant name, for instance "FieldKind::F64"
        expected: &'static str,
        /// Debug representation of actual [`crate::visitor::FieldKind`]
        actual: String,
    },
    /// Attempting to enter a region on a read-mode Visitor when no region in the visitor's data
    /// has the given name.
    RegionDoesNotExist(String),
    /// The Visitor tried to leave is current node, but somehow it had no current node. This should never happen.
    NoActiveNode,
    /// The [`crate::Visitor::MAGIC_BINARY_CURRENT`], [`crate::Visitor::MAGIC_ASCII_CURRENT`].
    /// bytes were missing from the beginning of encoded Visitor data.
    NotSupportedFormat,
    /// Some sequence of bytes was not in UTF8 format.
    InvalidName,
    /// Visitor data can be self-referential, such as when the data contains multiple `Rc` references
    /// to a single shared value. This causes the visitor to store the data once and then later references
    /// to the same value point back to its first occurrence. This error occurs if one of these references
    /// points to a value of the wrong type.
    TypeMismatch {
        /// The type that was visiting when the error occurred.
        expected: &'static str,
        /// The type that was stored in the `Rc` or `Arc`.
        actual: &'static str,
    },
    /// Attempting to visit a mutably borrowed RefCell.
    RefCellAlreadyMutableBorrowed,
    /// A plain-text error message that could indicate almost anything.
    User(String),
    /// `Rc` and `Arc` values store an "Id" value in the Visitor data which is based in their internal pointer.
    /// This error indicates that while reading this data, one of those Id values was discovered by be 0.
    UnexpectedRcNullIndex,
    /// A poison error occurred while trying to visit a mutex.
    PoisonedMutex,
    /// A FileLoadError was encountered while trying to decode Visitor data from a file.
    FileLoadError(FileError),
    /// Integer parsing error.
    ParseIntError(ParseIntError),
    /// Floating point number parsing error.
    ParseFloatError(ParseFloatError),
    /// An error occurred when trying to decode base64-encoded data.
    DecodeError(DecodeError),
    /// An error occurred when trying to parse uuid from a string.
    UuidError(uuid::Error),
    /// Arbitrary error.
    Any(Box<dyn Error + Send + Sync>),
}

impl Error for VisitError {}

impl VisitError {
    /// Create a [`VisitError::FieldDoesNotExist`] containing the given field name and the
    /// breadcrumbs of the current visitor node.
    pub fn field_does_not_exist(name: &str, visitor: &Visitor) -> Self {
        Self::FieldDoesNotExist(visitor.breadcrumbs() + " > " + name)
    }
    /// Create an error from two errors.
    pub fn multiple(self, other: Self) -> Self {
        match (self, other) {
            (Self::Multiple(mut a), Self::Multiple(mut b)) => {
                a.append(&mut b);
                Self::Multiple(a)
            }
            (Self::Multiple(mut a), b) => {
                a.push(b);
                Self::Multiple(a)
            }
            (a, Self::Multiple(mut b)) => {
                b.push(a);
                Self::Multiple(b)
            }
            (a, b) => Self::Multiple(vec![a, b]),
        }
    }
}

impl Display for VisitError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::Multiple(errs) => {
                write!(f, "multiple errors:[")?;
                for err in errs {
                    write!(f, "{err};")?;
                }
                write!(f, "]")
            }
            Self::Io(io) => write!(f, "io error: {io}"),
            Self::UnknownFieldType(type_index) => write!(f, "unknown field type {type_index}"),
            Self::FieldDoesNotExist(name) => write!(f, "field does not exist: {name}"),
            Self::FieldAlreadyExists(name) => write!(f, "field already exists {name}"),
            Self::RegionAlreadyExists(name) => write!(f, "region already exists {name}"),
            Self::InvalidCurrentNode => write!(f, "invalid current node"),
            Self::FieldTypeDoesNotMatch { expected, actual } => write!(
                f,
                "field type does not match. expected: {expected}, actual: {actual}"
            ),
            Self::RegionDoesNotExist(name) => write!(f, "region does not exist: {name}"),
            Self::NoActiveNode => write!(f, "no active node"),
            Self::NotSupportedFormat => write!(f, "not supported format"),
            Self::InvalidName => write!(f, "invalid name"),
            Self::TypeMismatch { expected, actual } => {
                write!(f, "type mismatch. expected: {expected}, actual: {actual}")
            }
            Self::RefCellAlreadyMutableBorrowed => write!(f, "ref cell already mutable borrowed"),
            Self::User(msg) => write!(f, "user defined error: {msg}"),
            Self::UnexpectedRcNullIndex => write!(f, "unexpected rc null index"),
            Self::PoisonedMutex => write!(f, "attempt to lock poisoned mutex"),
            Self::FileLoadError(e) => write!(f, "file load error: {e:?}"),
            Self::ParseIntError(e) => write!(f, "unable to parse integer: {e:?}"),
            Self::ParseFloatError(e) => write!(f, "unable to parse float: {e:?}"),
            Self::DecodeError(e) => write!(f, "base64 decoding error: {e:?}"),
            Self::UuidError(e) => write!(f, "uuid error: {e:?}"),
            Self::Any(e) => {
                write!(f, "{e}")
            }
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

impl From<FileError> for VisitError {
    fn from(e: FileError) -> Self {
        Self::FileLoadError(e)
    }
}

impl From<ParseIntError> for VisitError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

impl From<ParseFloatError> for VisitError {
    fn from(value: ParseFloatError) -> Self {
        Self::ParseFloatError(value)
    }
}

impl From<DecodeError> for VisitError {
    fn from(value: DecodeError) -> Self {
        Self::DecodeError(value)
    }
}

impl From<uuid::Error> for VisitError {
    fn from(value: uuid::Error) -> Self {
        Self::UuidError(value)
    }
}

impl From<Box<dyn Error + Send + Sync>> for VisitError {
    fn from(value: Box<dyn Error + Send + Sync>) -> Self {
        Self::Any(value)
    }
}
