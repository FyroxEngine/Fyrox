//! Contains all possible errors that can occur during FBX parsing and conversion.

use crate::core::io::FileLoadError;

/// See module docs.
#[derive(Debug, thiserror::Error)]
pub enum FbxError {
    /// An input/output error has occurred (unexpected end of file, etc.)
    #[error("FBX: Io error: {0}")]
    Io(std::io::Error),

    /// Type of attribute is unknown or not supported.
    #[error("FBX: Unknown or unsupported attribute type {0}")]
    UnknownAttributeType(u8),

    /// Corrupted null record of binary FBX.
    #[error("FBX: Corrupted null record of binary FBX")]
    InvalidNullRecord,

    /// A string has invalid content (non UTF8-compliant)
    #[error("FBX: A string has invalid content (non UTF8-compliant)")]
    InvalidString,

    /// Arbitrary error that can have any meaning.
    #[error("FBX: An error has occurred: {0}")]
    Custom(Box<String>),

    /// Version is not supported.
    #[error("FBX: Version is not supported: {0}")]
    UnsupportedVersion(i32),

    /// Internal handle is invalid.
    #[error("FBX: Internal handle is invalid.")]
    InvalidPoolHandle,

    /// Attempt to "cast" enum to unexpected variant.
    #[error("FBX: Internal invalid cast.")]
    UnexpectedType,

    /// Internal error that means some index was out of bounds. Probably a bug in implementation.
    #[error("FBX: Index is out-of-bounds.")]
    IndexOutOfBounds,

    /// Vertex references non existing bone.
    #[error("FBX: Vertex references non existing bone.")]
    UnableToFindBone,

    /// There is no corresponding scene node for a FBX model.
    #[error("FBX: There is no corresponding scene node for a FBX model.")]
    UnableToRemapModelToNode,

    /// Unknown or unsupported mapping.
    #[error("FBX: Unknown or unsupported mapping.")]
    InvalidMapping,

    /// Unknown or unsupported reference.
    #[error("FBX: Unknown or unsupported reference.")]
    InvalidReference,

    /// An error occurred during file loading.
    #[error("FBX: File load error {0:?}.")]
    FileLoadError(FileLoadError),
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
