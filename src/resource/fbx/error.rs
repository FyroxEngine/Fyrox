//! Contains all possible errors that can occur during FBX parsing and conversion.

use rg3d_core::io::FileLoadError;
use std::fmt::Formatter;

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
    /// Version is not supported. Keep in mind that binary FBX 7500 is still not supported!
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
    FileLoadError(FileLoadError),
}

impl std::fmt::Display for FbxError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Self::Io(io) => write!(f, "Io error: {}", io),
            Self::UnknownAttributeType(attrib_type) => {
                write!(f, "Unknown attribute type {}", attrib_type)
            }
            Self::InvalidNullRecord => write!(f, "Invalid null record"),
            Self::InvalidString => write!(f, "Invalid string"),
            Self::Custom(err) => write!(f, "{}", err),
            Self::UnsupportedVersion(ver) => write!(f, "Unsupported version {}", ver),
            Self::InvalidPoolHandle => write!(f, "Invalid pool handle."),
            Self::UnexpectedType => write!(
                f,
                "Unexpected type. This means that invalid cast has occured in fbx component."
            ),
            Self::IndexOutOfBounds => write!(f, "Index out of bounds."),
            Self::UnableToFindBone => write!(f, "Unable to find bone."),
            Self::UnableToRemapModelToNode => write!(f, "Unable to remap model to node."),
            Self::InvalidMapping => write!(f, "Unknown mapping"),
            Self::InvalidReference => write!(f, "Unknown reference"),
            Self::FileLoadError(e) => write!(f, "File load error: {:?}", e),
        }
    }
}

impl From<FileLoadError> for FbxError {
    fn from(err: FileLoadError) -> Self {
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
