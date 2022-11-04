//! Runtime reflection

mod external_impls;
mod std_impls;

pub use fyrox_core_derive::Reflect;

use std::fmt::{Display, Formatter};
use std::{
    any::{Any, TypeId},
    fmt::{self, Debug},
};

pub mod prelude {
    pub use super::{FieldInfo, Reflect};
}

/// A value of a field..
pub trait FieldValue: Any + 'static {
    /// Casts `self` to a `&dyn Any`
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> FieldValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An error that can occur during "type casting"
#[derive(Debug)]
pub enum CastError {
    /// Given type does not match expected.
    TypeMismatch {
        /// A name of the field.
        property_name: String,

        /// Expected type identifier.
        expected_type_id: TypeId,

        /// Actual type identifier.
        actual_type_id: TypeId,
    },
}

pub struct FieldInfo<'a> {
    /// A type id of the owner of the property.
    pub owner_type_id: TypeId,

    /// A name of the property.
    pub name: &'static str,

    /// A human-readable name of the property.
    pub display_name: &'static str,

    /// Description of the property.
    pub description: &'static str,

    /// An reference to the actual value of the property. This is "non-mangled" reference, which
    /// means that while `field/fields/field_mut/fields_mut` might return a reference to other value,
    /// than the actual field, the `value` is guaranteed to be a reference to the real value.
    pub value: &'a dyn FieldValue,

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
}

impl<'a> FieldInfo<'a> {
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

impl<'a> fmt::Debug for FieldInfo<'a> {
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

impl<'a> PartialEq<Self> for FieldInfo<'a> {
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
    fn fields_info(&self) -> Vec<FieldInfo>;

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

    fn fields(&self) -> Vec<&dyn Reflect> {
        vec![]
    }

    fn fields_mut(&mut self) -> Vec<&mut dyn Reflect> {
        vec![]
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

    fn as_inheritable_variable(&self) -> Option<&dyn ReflectInheritableVariable> {
        None
    }

    fn as_inheritable_variable_mut(&mut self) -> Option<&mut dyn ReflectInheritableVariable> {
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

pub trait ReflectInheritableVariable: Reflect + Debug {
    /// Tries to inherit a value from parent. It will succeed only if the current variable is
    /// not marked as modified.
    fn try_inherit(
        &mut self,
        parent: &dyn ReflectInheritableVariable,
    ) -> Result<Option<Box<dyn Reflect>>, InheritError>;

    /// Resets modified flag from the variable.
    fn reset_modified_flag(&mut self);

    /// Returns current variable flags.
    fn flags(&self) -> VariableFlags;

    /// Returns true if value was modified.
    fn is_modified(&self) -> bool;

    /// Returns true if value equals to other's value.
    fn value_equals(&self, other: &dyn ReflectInheritableVariable) -> bool;

    /// Clones self value.
    fn clone_value_box(&self) -> Box<dyn Reflect>;

    /// Marks value as modified, so its value won't be overwritten during property inheritance.
    fn mark_modified(&mut self);

    /// Returns a mutable reference to wrapped value without marking the variable itself as modified.
    fn inner_value_mut(&mut self) -> &mut dyn Reflect;

    /// Returns a shared reference to wrapped value without marking the variable itself as modified.
    fn inner_value_ref(&self) -> &dyn Reflect;
}

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq)]
pub enum ReflectPathError<'a> {
    // syntax errors
    UnclosedBrackets { s: &'a str },
    InvalidIndexSyntax { s: &'a str },

    // access errors
    UnknownField { s: &'a str },
    NoItemForIndex { s: &'a str },

    // type cast errors
    InvalidDowncast,
    NotAnArray,
}

impl<'a> Display for ReflectPathError<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReflectPathError::UnclosedBrackets { s } => {
                write!(f, "unclosed brackets: `{s}`")
            }
            ReflectPathError::InvalidIndexSyntax { s } => {
                write!(f, "not index syntax: `{s}`")
            }
            ReflectPathError::UnknownField { s } => {
                write!(f, "given unknown field: `{s}`")
            }
            ReflectPathError::NoItemForIndex { s } => {
                write!(f, "no item for index: `{s}`")
            }
            ReflectPathError::InvalidDowncast => {
                write!(
                    f,
                    "failed to downcast to the target type after path resolution"
                )
            }
            ReflectPathError::NotAnArray => {
                write!(f, "tried to resolve index access, but the reflect type does not implement list API")
            }
        }
    }
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

/// Splits property path into individual components.
pub fn path_to_components(path: &str) -> Vec<Component> {
    let mut components = Vec::new();
    let mut current_path = path;
    while let Ok((component, sub_path)) = Component::next(current_path) {
        if let Component::Field(field) = component {
            if field.is_empty() {
                break;
            }
        }
        current_path = sub_path;
        components.push(component);
    }
    components
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
pub enum Component<'p> {
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
        fn fields_info(&self) -> Vec<FieldInfo> {
            vec![]
        }

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

#[macro_export]
macro_rules! delegate_reflect {
    () => {
        fn fields_info(&self) -> Vec<FieldInfo> {
            self.deref().fields_info()
        }

        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            (*self).into_any()
        }

        fn as_any(&self) -> &dyn Any {
            self.deref().as_any()
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self.deref_mut().as_any_mut()
        }

        fn as_reflect(&self) -> &dyn Reflect {
            self.deref().as_reflect()
        }

        fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
            self.deref_mut().as_reflect_mut()
        }

        fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
            self.deref_mut().set(value)
        }

        fn field(&self, name: &str) -> Option<&dyn Reflect> {
            self.deref().field(name)
        }

        fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
            self.deref_mut().field_mut(name)
        }

        fn as_array(&self) -> Option<&dyn ReflectArray> {
            self.deref().as_array()
        }

        fn as_array_mut(&mut self) -> Option<&mut dyn ReflectArray> {
            self.deref_mut().as_array_mut()
        }

        fn as_list(&self) -> Option<&dyn ReflectList> {
            self.deref().as_list()
        }

        fn as_list_mut(&mut self) -> Option<&mut dyn ReflectList> {
            self.deref_mut().as_list_mut()
        }
    };
}

use crate::variable::{InheritError, VariableFlags};
pub use blank_reflect;
pub use delegate_reflect;
