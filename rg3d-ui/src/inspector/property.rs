use crate::inspector::InspectorError;
use std::any::{Any, TypeId};
use std::fmt::Debug;

pub trait PropertyValue: Any + Send + Sync + Debug {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Send + Sync + Debug + 'static> PropertyValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct PropertyInfo<'a> {
    pub name: &'a str,
    pub group: &'static str,
    pub value: &'a dyn PropertyValue,
}

impl<'a> PropertyInfo<'a> {
    pub fn cast_value<T: 'static>(&self) -> Result<&T, InspectorError> {
        match self.value.as_any().downcast_ref::<T>() {
            Some(value) => Ok(value),
            None => Err(InspectorError::TypeMismatch {
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
