use std::{
    any::{Any, TypeId},
    cmp::PartialEq,
    fmt::{self, Debug},
};

pub trait PropertyValue: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + Sync + Debug + 'static> PropertyValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
pub enum CastError {
    TypeMismatch {
        property_name: String,
        expected_type_id: TypeId,
        actual_type_id: TypeId,
    },
}

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

pub trait Inspect {
    fn properties(&self) -> Vec<PropertyInfo<'_>>;
}

pub use rg3d_core_derive::Inspect;
