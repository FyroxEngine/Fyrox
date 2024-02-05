pub use fyrox_core_derive::ScriptSourcePathProvider;

pub mod prelude {
    pub use super::ScriptSourcePathProvider;
}

/// Script source path provider
pub trait ScriptSourcePathProvider: Sized {
    /// Script source path to open it from Editor
    fn script_source_path() -> &'static str;
}
