//! Runtime reflection

mod external_impls;
mod std_impls;

pub use fyrox_core_derive::Reflect;

use std::{
    any::{Any, TypeId},
    fmt,
};
use thiserror::Error;

/// Trait for runtime reflection
///
/// Derive macro is available.
///
/// # Type attributes
/// - `#[reflect(hide_all)]`: Hide all fields, just like `Any`
/// - `#[reflect(bounds)]`: Add type boundary for `Reflect` impl
///
/// # Field attributes
/// - `#[reflect(deref)]`: Delegate the field access with deref
/// - `#[reflect(field = <method call>)]
/// - `#[reflect(field_mut = <method call>)]
pub trait Reflect: Any {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn as_reflect(&self) -> &dyn Reflect;

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect;

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>>;

    /// Calls user method specified with `#[reflect(setter = ..)]` or falls back to
    /// [`Reflect::field_mut`]
    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        match self.field_mut(field) {
            Some(f) => f.set(value),
            None => Err(value),
        }
    }

    fn field(&self, _name: &str) -> Option<&dyn Reflect> {
        None
    }

    fn field_mut(&mut self, _name: &str) -> Option<&mut dyn Reflect> {
        None
    }

    fn as_array(&self) -> Option<&dyn ReflectArray> {
        None
    }

    fn as_array_mut(&mut self) -> Option<&mut dyn ReflectArray> {
        None
    }

    fn as_list(&self) -> Option<&dyn ReflectList> {
        None
    }

    fn as_list_mut(&mut self) -> Option<&mut dyn ReflectList> {
        None
    }
}

/// [`Reflect`] sub trait for working with slices.
pub trait ReflectArray: Reflect {
    fn reflect_index(&self, index: usize) -> Option<&dyn Reflect>;
    fn reflect_index_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn reflect_len(&self) -> usize;
}

/// [`Reflect`] sub trait for working with `Vec`-like types
pub trait ReflectList: ReflectArray {
    fn reflect_push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;
    fn reflect_pop(&mut self) -> Option<Box<dyn Reflect>>;
    fn reflect_remove(&mut self, index: usize) -> Option<Box<dyn Reflect>>;
    fn reflect_insert(
        &mut self,
        index: usize,
        value: Box<dyn Reflect>,
    ) -> Result<(), Box<dyn Reflect>>;
}

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    // syntax errors
    #[error("unclosed brackets: `{s}`")]
    UnclosedBrackets { s: &'a str },
    #[error("not index syntax: `{s}`")]
    InvalidIndexSyntax { s: &'a str },

    // access errors
    #[error("given unknown field: `{s}`")]
    UnknownField { s: &'a str },
    #[error("no item for index: `{s}`")]
    NoItemForIndex { s: &'a str },

    // type cast errors
    #[error("failed to downcast to the target type after path resolution")]
    InvalidDowncast,
    #[error("tried to resolve index access, but the reflect type does not implement list API")]
    NotAnArray,
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

// --------------------------------------------------------------------------------
// impl dyn Trait
// --------------------------------------------------------------------------------

/// Simple path parser / reflect path component
enum Component<'p> {
    Field(&'p str),
    Index(&'p str),
}

impl<'p> Component<'p> {
    fn next(mut path: &'p str) -> Result<(Self, &'p str), ReflectPathError<'p>> {
        // Discard the first comma:
        if path.bytes().next() == Some(b'.') {
            path = &path[1..];
        }

        let mut bytes = path.bytes().enumerate();
        while let Some((i, b)) = bytes.next() {
            if b == b'.' {
                let (l, r) = path.split_at(i);
                return Ok((Self::Field(l), &r[1..]));
            }

            if b == b'[' {
                if i != 0 {
                    // delimit the field access
                    let (l, r) = path.split_at(i);
                    return Ok((Self::Field(l), r));
                }

                // find ']'
                if let Some((end, _)) = bytes.find(|(_, b)| *b == b']') {
                    let l = &path[1..end];
                    let r = &path[end + 1..];
                    return Ok((Self::Index(l), r));
                } else {
                    return Err(ReflectPathError::UnclosedBrackets { s: path });
                }
            }
        }

        // NOTE: the `path` can be empty
        Ok((Self::Field(path), ""))
    }

    fn resolve<'r>(
        &self,
        reflect: &'r dyn Reflect,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        match self {
            Self::Field(path) => reflect
                .field(path)
                .ok_or(ReflectPathError::UnknownField { s: path }),
            Self::Index(path) => {
                let list = reflect.as_array().ok_or(ReflectPathError::NotAnArray)?;
                let index = path
                    .parse::<usize>()
                    .map_err(|_| ReflectPathError::InvalidIndexSyntax { s: path })?;
                list.reflect_index(index)
                    .ok_or(ReflectPathError::NoItemForIndex { s: path })
            }
        }
    }

    fn resolve_mut<'r>(
        &self,
        reflect: &'r mut dyn Reflect,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        match self {
            Self::Field(path) => reflect
                .field_mut(path)
                .ok_or(ReflectPathError::UnknownField { s: path }),
            Self::Index(path) => {
                let list = reflect.as_array_mut().ok_or(ReflectPathError::NotAnArray)?;
                let index = path
                    .parse::<usize>()
                    .map_err(|_| ReflectPathError::InvalidIndexSyntax { s: path })?;
                list.reflect_index_mut(index)
                    .ok_or(ReflectPathError::NoItemForIndex { s: path })
            }
        }
    }
}

impl ResolvePath for dyn Reflect {
    fn resolve_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        let (component, r) = Component::next(path)?;
        let child = component.resolve(self)?;
        if r.is_empty() {
            Ok(child)
        } else {
            child.resolve_path(r)
        }
    }

    fn resolve_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let (component, r) = Component::next(path)?;
        let child = component.resolve_mut(self)?;
        if r.is_empty() {
            Ok(child)
        } else {
            child.resolve_path_mut(r)
        }
    }
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

// Make it a trait?
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
macro_rules! blank_reflect {
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

        fn field(&self, name: &str) -> Option<&dyn Reflect> {
            if name == "self" {
                Some(self)
            } else {
                None
            }
        }

        fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
            if name == "self" {
                Some(self)
            } else {
                None
            }
        }

        fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
            let this = std::mem::replace(self, value.take()?);
            Ok(Box::new(this))
        }
    };
}

pub use blank_reflect;
