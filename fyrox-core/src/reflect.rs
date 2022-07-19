//! Runtime reflection

use std::any::Any;

pub trait Reflect: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn field(&self, _name: &str) -> Option<&dyn Reflect> {
        None
    }

    fn field_mut(&mut self, _name: &str) -> Option<&mut dyn Reflect> {
        None
    }
}

/// Helper methods over [`Reflect`] types
pub trait GetField {
    fn get_field<T: 'static>(&self, name: &str) -> Option<&T>;

    fn get_field_mut<T: 'static>(&mut self, _name: &str) -> Option<&mut T>;
}

impl<R: Reflect> GetField for R {
    fn get_field<T: 'static>(&self, name: &str) -> Option<&T> {
        self.field(name)
            .and_then(|reflect| reflect.as_any().downcast_ref())
    }

    fn get_field_mut<T: 'static>(&mut self, name: &str) -> Option<&mut T> {
        self.field_mut(name)
            .and_then(|reflect| reflect.as_any_mut().downcast_mut())
    }
}
