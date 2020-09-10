extern crate rand;
#[macro_use]
extern crate memoffset;
#[macro_use]
extern crate lazy_static;

pub mod color;
pub mod color_gradient;
pub mod math;
pub mod numeric_range;
pub mod octree;
pub mod pool;
pub mod rectpack;
pub mod visitor;

pub mod profiler;

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
