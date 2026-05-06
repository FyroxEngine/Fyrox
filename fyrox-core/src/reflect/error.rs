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

use crate::reflect::Reflect;
use std::any::TypeId;
use std::fmt;
use std::fmt::{Display, Formatter};

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq)]
pub enum ReflectPathError<'a> {
    // syntax errors
    UnclosedBrackets { s: &'a str },
    InvalidIndexSyntax { s: &'a str },

    // access errors
    UnknownField { s: &'a str },
    NoItemForIndex { s: &'a str },

    // type cast errors
    InvalidDowncast,
    NotAnArray,
}

impl std::error::Error for ReflectPathError<'_> {}

impl Display for ReflectPathError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReflectPathError::UnclosedBrackets { s } => {
                write!(f, "unclosed brackets: `{s}`")
            }
            ReflectPathError::InvalidIndexSyntax { s } => {
                write!(f, "not index syntax: `{s}`")
            }
            ReflectPathError::UnknownField { s } => {
                write!(f, "given unknown field: `{s}`")
            }
            ReflectPathError::NoItemForIndex { s } => {
                write!(f, "no item for index: `{s}`")
            }
            ReflectPathError::InvalidDowncast => {
                write!(
                    f,
                    "failed to downcast to the target type after path resolution"
                )
            }
            ReflectPathError::NotAnArray => {
                write!(f, "tried to resolve index access, but the reflect type does not implement list API")
            }
        }
    }
}

/// An error that can occur during "type casting"
#[derive(Debug)]
pub enum CastError {
    /// Given type does not match expected.
    TypeMismatch {
        /// A name of the field.
        property_name: String,

        /// Expected type identifier.
        expected_type_id: TypeId,

        /// Actual type identifier.
        actual_type_id: TypeId,
    },
}

impl std::error::Error for CastError {}

impl Display for CastError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CastError::TypeMismatch { property_name, .. } => {
                write!(
                    f,
                    "Given type does not match expected for property {property_name:?}"
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum SetFieldError {
    NoSuchField {
        name: String,
        value: Box<dyn Reflect>,
    },
    InvalidValue {
        field_type_name: &'static str,
        value: Box<dyn Reflect>,
    },
}

impl std::error::Error for SetFieldError {}

impl Display for SetFieldError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SetFieldError::NoSuchField { name, value } => {
                write!(f, "No such field as {name:?} for value {value:?}")
            }
            SetFieldError::InvalidValue {
                field_type_name,
                value,
            } => write!(
                f,
                "Invalid value for field type {field_type_name}: {value:?}"
            ),
        }
    }
}

#[derive(Debug)]
pub enum SetFieldByPathError<'p> {
    InvalidPath {
        value: Box<dyn Reflect>,
        reason: ReflectPathError<'p>,
    },
    InvalidValue {
        field_type_name: &'static str,
        value: Box<dyn Reflect>,
    },
    SetFieldError(SetFieldError),
}

impl std::error::Error for SetFieldByPathError<'_> {}

impl Display for SetFieldByPathError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SetFieldByPathError::InvalidPath { value, reason } => {
                write!(f, "Invalid path: {value:?}. Reason: {reason}")
            }
            SetFieldByPathError::InvalidValue {
                field_type_name,
                value,
            } => {
                write!(f, "Invalid value: {value:?}. Type: {field_type_name}")
            }
            SetFieldByPathError::SetFieldError(set_field_error) => Display::fmt(set_field_error, f),
        }
    }
}
