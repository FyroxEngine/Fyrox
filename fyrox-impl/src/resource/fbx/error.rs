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

//! Contains all possible errors that can occur during FBX parsing and conversion.

use crate::core::io::FileError;
use std::fmt::{Display, Formatter};

/// See module docs.
#[derive(Debug)]
pub enum FbxError {
    /// An input/output error has occurred (unexpected end of file, etc.)
    Io(std::io::Error),

    /// Type of attribute is unknown or not supported.
    UnknownAttributeType(u8),

    /// Corrupted null record of binary FBX.
    InvalidNullRecord,

    /// A string has invalid content (non UTF8-compliant)
    InvalidString,

    /// Arbitrary error that can have any meaning.
    Custom(Box<String>),

    /// Version is not supported.
    UnsupportedVersion(i32),

    /// Internal handle is invalid.
    InvalidPoolHandle,

    /// Attempt to "cast" enum to unexpected variant.
    UnexpectedType,

    /// Internal error that means some index was out of bounds. Probably a bug in implementation.
    IndexOutOfBounds,

    /// Vertex references non existing bone.
    UnableToFindBone,

    /// There is no corresponding scene node for a FBX model.
    UnableToRemapModelToNode,

    /// Unknown or unsupported mapping.
    InvalidMapping,

    /// Unknown or unsupported reference.
    InvalidReference,

    /// An error occurred during file loading.
    FileLoadError(FileError),
}

impl Display for FbxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FbxError::Io(v) => {
                write!(f, "FBX: Io error: {v}")
            }
            FbxError::UnknownAttributeType(v) => {
                write!(f, "FBX: Unknown or unsupported attribute type {v}")
            }
            FbxError::InvalidNullRecord => {
                write!(f, "FBX: Corrupted null record of binary FBX")
            }
            FbxError::InvalidString => {
                write!(f, "FBX: A string has invalid content (non UTF8-compliant)")
            }
            FbxError::Custom(v) => {
                write!(f, "FBX: An error has occurred: {v}")
            }
            FbxError::UnsupportedVersion(v) => {
                write!(f, "FBX: Version is not supported: {v}")
            }
            FbxError::InvalidPoolHandle => {
                write!(f, "FBX: Internal handle is invalid.")
            }
            FbxError::UnexpectedType => {
                write!(f, "FBX: Internal invalid cast.")
            }
            FbxError::IndexOutOfBounds => {
                write!(f, "FBX: Index is out-of-bounds.")
            }
            FbxError::UnableToFindBone => {
                write!(f, "FBX: Vertex references non existing bone.")
            }
            FbxError::UnableToRemapModelToNode => {
                write!(
                    f,
                    "FBX: There is no corresponding scene node for a FBX model."
                )
            }
            FbxError::InvalidMapping => {
                write!(f, "FBX: Unknown or unsupported mapping.")
            }
            FbxError::InvalidReference => {
                write!(f, "FBX: Unknown or unsupported reference.")
            }
            FbxError::FileLoadError(v) => {
                write!(f, "FBX: File load error {v:?}.")
            }
        }
    }
}

impl From<FileError> for FbxError {
    fn from(err: FileError) -> Self {
        FbxError::FileLoadError(err)
    }
}

impl From<std::io::Error> for FbxError {
    fn from(err: std::io::Error) -> Self {
        FbxError::Io(err)
    }
}

impl From<String> for FbxError {
    fn from(err: String) -> Self {
        FbxError::Custom(Box::new(err))
    }
}

impl From<std::string::FromUtf8Error> for FbxError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        FbxError::InvalidString
    }
}
