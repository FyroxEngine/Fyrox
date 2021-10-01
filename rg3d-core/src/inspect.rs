//! Lightweight read-only runtime reflection.
//!
//! See [`Inspect`] for more info.

#![warn(missing_docs)]

use std::{
    any::{Any, TypeId},
    cmp::PartialEq,
    fmt::{self, Debug},
};

/// A value of a property.
pub trait PropertyValue: Any + Send + Sync + Debug {
    /// Casts `self` to a `&dyn Any`
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + Sync + Debug + 'static> PropertyValue for T {
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

    /// A name of the group that the property belongs to.
    pub group: &'static str,

    /// An reference to the actual value of the property.
    pub value: &'a dyn PropertyValue,

    /// A property is not meant to be edited.
    pub read_only: bool,
}

impl<'a> fmt::Debug for PropertyInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyInfo")
            .field("owner_type_id", &self.owner_type_id)
            .field("name", &self.name)
            .field("display_name", &self.display_name)
            .field("group", &self.group)
            .field("value", &format_args!("{:?}", self.value as *const _))
            .finish()
    }
}

impl<'a> PartialEq<Self> for PropertyInfo<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.group == other.group
            && self.display_name == other.display_name
            && self.value as *const _ == other.value as *const _
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
/// It is not advised to manually implement this trait. You should use `#[derive(Inspect)]` whenever
/// possible.
///
/// ## `#[derive(Inspect)]`
///
/// The proc macro reduces amount of boilerplate code to the minimum and significantly reduces a
/// change of error.
///
/// ### Supported attributes
///
/// - `#[inspect(name = "new_field_name")]` - override field name.
/// - `#[inspect(display_name = "Human-readable Name")]` - override display name.
/// - `#[inspect(group = "Group Name")]` - override group name.
/// - `#[inspect(expand)]` - extends the list of properties in case of composition, in other words it
/// "flattens" and exposes the properties of an inner object. Useful when you have a structure that
/// has some fields that are complex objects that implements `Inspect` too.  
pub trait Inspect {
    /// Returns information about "public" properties.
    fn properties(&self) -> Vec<PropertyInfo<'_>>;
}

impl<T: Inspect> Inspect for Option<T> {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            Some(v) => v.properties(),
            None => vec![],
        }
    }
}

pub use rg3d_core_derive::Inspect;
