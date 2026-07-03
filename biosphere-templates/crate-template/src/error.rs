use std::fmt;

/// All errors this module can produce.
///
/// Convention: mirror the shape of fyrox_biosphere::container_format::ContainerError —
/// wrap lower-level errors, don't flatten them.
#[derive(Debug, Clone)]
pub enum ModuleError {
    /// A Capacity Law violation (≤16 at each level).
    Capacity { message: String },
    /// A heraldry mismatch (wrong symbolic type for this container level).
    Heraldry { message: String },
    /// A lifecycle violation (e.g. mutating a Sealed container).
    Lifecycle { message: String },
    /// Catch-all for validation errors not covered above.
    Validation { message: String },
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleError::Capacity { message } => write!(f, "Capacity: {message}"),
            ModuleError::Heraldry { message } => write!(f, "Heraldry: {message}"),
            ModuleError::Lifecycle { message } => write!(f, "Lifecycle: {message}"),
            ModuleError::Validation { message } => write!(f, "Validation: {message}"),
        }
    }
}

impl std::error::Error for ModuleError {}
