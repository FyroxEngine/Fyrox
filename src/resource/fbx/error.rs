use std::fmt::Formatter;

#[derive(Debug)]
pub enum FbxError {
    Io(std::io::Error),
    UnknownAttributeType(u8),
    InvalidNullRecord,
    InvalidString,
    Custom(Box<String>),
    UnsupportedVersion(i32),
    InvalidPoolHandle,
    UnexpectedType,
    InvalidPath,
    IndexOutOfBounds,
    UnableToFindBone,
    UnableToRemapModelToNode,
    InvalidMapping,
    InvalidReference,
}

impl std::fmt::Display for FbxError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            FbxError::Io(io) => write!(f, "Io error: {}", io),
            FbxError::UnknownAttributeType(attrib_type) => write!(f, "Unknown attribute type {}", attrib_type),
            FbxError::InvalidNullRecord => write!(f, "Invalid null record"),
            FbxError::InvalidString => write!(f, "Invalid string"),
            FbxError::Custom(err) => write!(f, "{}", err),
            FbxError::UnsupportedVersion(ver) => write!(f, "Unsupported version {}", ver),
            FbxError::InvalidPoolHandle => write!(f, "Invalid pool handle."),
            FbxError::UnexpectedType => write!(f, "Unexpected type. This means that invalid cast has occured in fbx component."),
            FbxError::InvalidPath => write!(f, "Invalid path. This means that some path was stored in invalid format."),
            FbxError::IndexOutOfBounds => write!(f, "Index out of bounds."),
            FbxError::UnableToFindBone => write!(f, "Unable to find bone."),
            FbxError::UnableToRemapModelToNode => write!(f, "Unable to remap model to node."),
            FbxError::InvalidMapping => write!(f, "Unknown mapping"),
            FbxError::InvalidReference => write!(f, "Unknown reference"),
        }
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
