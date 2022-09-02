//! Lightweight read-only runtime reflection.
//!
//! See [`Inspect`] for more info.

#![warn(missing_docs)]

use fyrox_core_derive::impl_inspect;
use nalgebra::{UnitQuaternion, Vector2, Vector3, Vector4};
use std::{
    any::{Any, TypeId},
    fmt::{self, Debug},
};

pub mod prelude {
    //! Standard import for `Inspect` proc macro.
    pub use super::{Inspect, PropertyInfo};
}

/// A value of a property.
pub trait PropertyValue: Any + Debug {
    /// Casts `self` to a `&dyn Any`
    fn as_any(&self) -> &dyn Any;
}

impl<T: Debug + 'static> PropertyValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An error that can occur during "type casting"
#[derive(Debug)]
pub enum CastError {
    /// Given type does not match expected.
    TypeMismatch {
        /// A name of the property.
        property_name: String,

        /// Expected type identifier.
        expected_type_id: TypeId,

        /// Actual type identifier.
        actual_type_id: TypeId,
    },
}

/// Information about a property of an object.
pub struct PropertyInfo<'a> {
    /// A type id of the owner of the property.
    pub owner_type_id: TypeId,

    /// A name of the property.
    pub name: &'a str,

    /// A human-readable name of the property.
    pub display_name: &'static str,

    /// An reference to the actual value of the property.
    pub value: &'a dyn PropertyValue,

    /// A property is not meant to be edited.
    pub read_only: bool,

    /// A minimal value of the property. Works only with numeric properties!
    pub min_value: Option<f64>,

    /// A minimal value of the property. Works only with numeric properties!
    pub max_value: Option<f64>,

    /// A minimal value of the property. Works only with numeric properties!
    pub step: Option<f64>,

    /// Maximum amount of decimal places for a numeric property.
    pub precision: Option<usize>,

    /// Description of the property.
    pub description: String,

    /// True if the value has been modified.
    pub is_modified: bool,
}

impl<'a> PartialEq<Self> for PropertyInfo<'a> {
    fn eq(&self, other: &Self) -> bool {
        let value_ptr_a = self.value as *const _ as *const ();
        let value_ptr_b = other.value as *const _ as *const ();

        self.owner_type_id == other.owner_type_id
            && self.name == other.name
            && self.display_name == other.display_name
            && std::ptr::eq(value_ptr_a, value_ptr_b)
            && self.read_only == other.read_only
            && self.min_value == other.min_value
            && self.max_value == other.max_value
            && self.step == other.step
            && self.precision == other.precision
            && self.description == other.description
    }
}

impl<'a> fmt::Debug for PropertyInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyInfo")
            .field("owner_type_id", &self.owner_type_id)
            .field("name", &self.name)
            .field("display_name", &self.display_name)
            .field("value", &format_args!("{:?}", self.value as *const _))
            .field("read_only", &self.read_only)
            .field("min_value", &self.min_value)
            .field("max_value", &self.max_value)
            .field("step", &self.step)
            .field("precision", &self.precision)
            .field("description", &self.description)
            .finish()
    }
}

impl<'a> PropertyInfo<'a> {
    /// Tries to cast a value to a given type.
    pub fn cast_value<T: 'static>(&self) -> Result<&T, CastError> {
        match self.value.as_any().downcast_ref::<T>() {
            Some(value) => Ok(value),
            None => Err(CastError::TypeMismatch {
                property_name: self.name.to_string(),
                expected_type_id: TypeId::of::<T>(),
                actual_type_id: self.value.type_id(),
            }),
        }
    }
}

/// A trait that allows you to "look inside" an object that implements it. It is used for lightweight
/// runtime read-only reflection. The most common use case for it is various editors.
///
/// It is not advised to manually implement this trait. You should use `#[derive(Inspect, Reflect)]`
/// whenever possible.
///
/// ## `#[derive(Inspect)]`
///
/// The proc macro reduces amount of boilerplate code to the minimum and significantly reduces a
/// change of error.
///
/// ### Supported attributes
///
/// - `#[inspect(display_name = "Human-readable Name")]` - override display name.
/// - `#[inspect(group = "Group Name")]` - override group name.
/// - `#[inspect(expand)]` - extends the list of properties in case of composition, in other words it
/// "flattens" and exposes the properties of an inner object. Useful when you have a structure that
/// has some fields that are complex objects that implements `Inspect` too.
pub trait Inspect {
    /// Returns information about "public" properties.
    fn properties(&self) -> Vec<PropertyInfo<'_>>;
}

impl_inspect! {
    pub enum Option<T: Inspect + Debug + 'static> {
        Some(T),
        None
    }
}

impl_inspect! {
    pub struct Box<T: Inspect + Debug + 'static>;
}

#[macro_export]
macro_rules! impl_numeric_inspect {
    ($ty:ty, $min:expr, $max:expr, $step:expr, $precision:expr) => {
        impl Inspect for $ty {
            fn properties(&self) -> Vec<PropertyInfo<'_>> {
                vec![PropertyInfo {
                    owner_type_id: TypeId::of::<Self>(),
                    name: "self",
                    display_name: "Value",
                    value: self,
                    read_only: false,
                    min_value: Some($min),
                    max_value: Some($max),
                    step: Some($step),
                    precision: Some($precision),
                    description: "".to_string(),
                    is_modified: false,
                }]
            }
        }
    };
}

impl_numeric_inspect!(f32, f32::MIN as f64, f32::MAX as f64, 1.0, 7);
impl_numeric_inspect!(f64, f64::MIN, f64::MAX, 1.0, 15);
impl_numeric_inspect!(i64, i64::MIN as f64, i64::MAX as f64, 1.0, 0);
impl_numeric_inspect!(u64, u64::MIN as f64, u64::MAX as f64, 1.0, 0);
impl_numeric_inspect!(i32, i32::MIN as f64, i32::MAX as f64, 1.0, 0);
impl_numeric_inspect!(u32, u32::MIN as f64, u32::MAX as f64, 1.0, 0);
impl_numeric_inspect!(i16, i16::MIN as f64, i16::MAX as f64, 1.0, 0);
impl_numeric_inspect!(u16, u16::MIN as f64, u16::MAX as f64, 1.0, 0);
impl_numeric_inspect!(i8, i8::MIN as f64, i8::MAX as f64, 1.0, 0);
impl_numeric_inspect!(u8, u8::MIN as f64, u8::MAX as f64, 1.0, 0);
impl_numeric_inspect!(usize, usize::MIN as f64, usize::MAX as f64, 1.0, 0);
impl_numeric_inspect!(isize, isize::MIN as f64, isize::MAX as f64, 1.0, 0);

#[macro_export]
macro_rules! impl_simple_inspect {
    ($ty:ty) => {
        impl Inspect for $ty {
            fn properties(&self) -> Vec<PropertyInfo<'_>> {
                vec![PropertyInfo {
                    owner_type_id: TypeId::of::<Self>(),
                    name: "self",
                    display_name: "Value",
                    value: self,
                    read_only: false,
                    min_value: None,
                    max_value: None,
                    step: None,
                    precision: None,
                    description: "".to_string(),
                    is_modified: false,
                }]
            }
        }
    };
}

impl_simple_inspect!(String);
impl_simple_inspect!(bool);
impl_simple_inspect!(Vector2<f32>);
impl_simple_inspect!(Vector3<f32>);
impl_simple_inspect!(Vector4<f32>);
impl_simple_inspect!(UnitQuaternion<f32>);

pub use fyrox_core_derive::Inspect;
