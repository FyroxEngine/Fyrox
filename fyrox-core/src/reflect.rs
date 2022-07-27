//! Runtime reflection

mod external_impls;
mod std_impls;

pub use fyrox_core_derive::Reflect;

use thiserror::Error;

use std::{
    any::{Any, TypeId},
    fmt,
};

pub trait Reflect: Any {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_reflect(&self) -> &dyn Reflect;

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;

    fn field(&self, _name: &str) -> Option<&dyn Reflect> {
        None
    }

    fn field_mut(&mut self, _name: &str) -> Option<&mut dyn Reflect> {
        None
    }

    fn as_list(&self) -> Option<&dyn ReflectList> {
        None
    }

    fn as_list_mut(&mut self) -> Option<&mut dyn ReflectList> {
        None
    }
}

/// [`Reflect`] sub trait for working with `Vec`-like types
// add `ReflectArray` sub trait?
pub trait ReflectList: Reflect {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect>;
    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn reflect_push(&mut self, value: Box<dyn Reflect>);
    fn reflect_len(&self) -> usize;
}

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    #[error("given invalid path component: `{s}`")]
    InvalidComponent { s: &'a str },
    #[error("failed to downcast to the path result to the given type")]
    InvalidDowncast,
}

pub trait ResolvePath {
    fn resolve_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>>;

    fn resolve_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>>;

    fn get_resolve_path<'r, 'p, T: Reflect>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r T, ReflectPathError<'p>> {
        self.resolve_path(path)
            .and_then(|r| r.downcast_ref().ok_or(ReflectPathError::InvalidDowncast))
    }

    fn get_resolve_path_mut<'r, 'p, T: Reflect>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut T, ReflectPathError<'p>> {
        self.resolve_path_mut(path)
            .and_then(|r| r.downcast_mut().ok_or(ReflectPathError::InvalidDowncast))
    }
}

impl<T: Reflect> ResolvePath for T {
    fn resolve_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        (self as &dyn Reflect).resolve_path(path)
    }

    fn resolve_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        (self as &mut dyn Reflect).resolve_path_mut(path)
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

impl ResolvePath for dyn Reflect {
    fn resolve_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        if let Some(comma) = path.find('.') {
            let (l, r) = path.split_at(comma);
            let child = self::resolve_stem(self, l)?;

            // discard comma
            child.resolve_path(&r[1..])
        } else {
            self::resolve_stem(self, path)
        }
    }

    fn resolve_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        if let Some(comma) = path.find('.') {
            let (l, r) = path.split_at(comma);
            let child = self::resolve_stem_mut(self, l)?;

            // discard comma
            child.resolve_path_mut(&r[1..])
        } else {
            self::resolve_stem_mut(self, path)
        }
    }
}

fn resolve_stem<'r, 'p>(
    reflect: &'r dyn Reflect,
    path: &'p str,
) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
    reflect
        .field(path)
        .ok_or(ReflectPathError::InvalidComponent { s: path })
}

fn resolve_stem_mut<'r, 'p>(
    reflect: &'r mut dyn Reflect,
    path: &'p str,
) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
    reflect
        .field_mut(path)
        .ok_or(ReflectPathError::InvalidComponent { s: path })
}

/// Type-erased API
impl dyn Reflect {
    pub fn downcast<T: Reflect>(self: Box<dyn Reflect>) -> Result<Box<T>, Box<dyn Reflect>> {
        if self.is::<T>() {
            Ok(self.into_any().downcast().unwrap())
        } else {
            Err(self)
        }
    }

    pub fn take<T: Reflect>(self: Box<dyn Reflect>) -> Result<T, Box<dyn Reflect>> {
        self.downcast::<T>().map(|value| *value)
    }

    #[inline]
    pub fn is<T: Reflect>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
    }

    #[inline]
    pub fn downcast_ref<T: Reflect>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    #[inline]
    pub fn downcast_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

impl dyn ReflectList {
    pub fn get_reflect_index<T: Reflect + 'static>(&self, index: usize) -> Option<&T> {
        self.reflect_index(index)
            .and_then(|reflect| reflect.downcast_ref())
    }

    pub fn get_reflect_index_mut<T: Reflect + 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.reflect_index_mut(index)
            .and_then(|reflect| reflect.downcast_mut())
    }
}

// for simple `#[derive(Debug)]`
impl fmt::Debug for dyn Reflect {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "dyn Reflect")
    }
}

impl fmt::Debug for dyn Reflect + 'static + Send {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "dyn Reflect")
    }
}

#[macro_export]
macro_rules! _blank_reflect {
    () => {
        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            self
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn as_reflect(&self) -> &dyn Reflect {
            self
        }

        fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
            self
        }

        fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
            *self = value.take()?;
            Ok(())
        }
    };
}

pub use _blank_reflect as blank_reflect;
