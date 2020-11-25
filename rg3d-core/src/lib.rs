#[macro_use]
extern crate memoffset;
#[macro_use]
extern crate lazy_static;

pub use arrayvec;
pub use byteorder;
pub use nalgebra as algebra;
pub use rand;

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub mod color;
pub mod color_gradient;
pub mod math;
pub mod numeric_range;
pub mod octree;
pub mod pool;
pub mod profiler;
pub mod rectpack;
pub mod visitor;

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
#[macro_export]
macro_rules! define_is_as {
    ($typ:tt : $kind:ident -> ref $result:ty => fn $is:ident, fn $as_ref:ident, fn $as_mut:ident) => {
        /// Returns true if node is instance of given type.
        pub fn $is(&self) -> bool {
            match self {
                $typ::$kind(_) => true,
                _ => false,
            }
        }

        /// Tries to cast shared reference to a node to given type, panics if
        /// cast is not possible.
        pub fn $as_ref(&self) -> &$result {
            match self {
                $typ::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind)),
            }
        }

        /// Tries to cast mutable reference to a node to given type, panics if
        /// cast is not possible.
        pub fn $as_mut(&mut self) -> &mut $result {
            match self {
                $typ::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind)),
            }
        }
    };
}

/// Utility function that replaces back slashes \ to forward /
/// It replaces slashes only on windows!
pub fn replace_slashes<P: AsRef<Path>>(path: P) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // Replace all \ to /. This is needed because on macos or linux \ is a valid symbol in
        // file name, and not separator (except linux which understand both variants).
        let mut os_str = OsString::new();
        let count = path.as_ref().components().count();
        for (i, component) in path.as_ref().components().enumerate() {
            os_str.push(component.as_os_str());
            if i != count - 1 {
                os_str.push("/");
            }
        }
        PathBuf::from(os_str)
    }

    #[cfg(not(target_os = "windows"))]
    {
        path.as_ref().to_owned()
    }
}
