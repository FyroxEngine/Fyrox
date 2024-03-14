//! A crate that allows using Fyrox as a dynamically linked library. It could be useful for fast
//! prototyping, that can save some time on avoiding potentially time-consuming static linking
//! stage.
//!
//! The crate just re-exports everything from the engine, and you can use it as Fyrox. To use the
//! crate all you need to do is re-define `fyrox` dependency in your project like so:
//!
//! ```toml
//! [dependencies.fyrox]
//! version = "0.1.0"
//! registry = "fyrox-dylib"
//! package = "fyrox-dylib"
//! ```
//!
//! You can also use the latest version from git:
//!
//! ```toml
//! [dependencies.fyrox]
//! git = "https://github.com/FyroxEngine/Fyrox"
//! package = "fyrox-dylib"
//! ```

// Just re-export everything.
pub use fyrox::*;
