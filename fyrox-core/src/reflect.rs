//! Runtime reflection

mod external_impls;
mod std_impls;

pub use fyrox_core_derive::Reflect;
use std::{
    any::{Any, TypeId},
    fmt::{self, Debug, Display, Formatter},
    mem::ManuallyDrop,
};

pub mod prelude {
    pub use super::{
        FieldInfo, Reflect, ReflectArray, ReflectHashMap, ReflectInheritableVariable, ReflectList,
        ResolvePath, SetFieldByPathError,
    };
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

pub struct FieldInfo<'a, 'b> {
    /// A type id of the owner of the property.
    pub owner_type_id: TypeId,

    /// A name of the property.
    pub name: &'b str,

    /// A human-readable name of the property.
    pub display_name: &'b str,

    /// Description of the property.
    pub description: &'b str,

    /// Type name of the property.
    pub type_name: &'b str,

    /// Doc comment content.
    pub doc: &'b str,

    /// An reference to the actual value of the property. This is "non-mangled" reference, which
    /// means that while `field/fields/field_mut/fields_mut` might return a reference to other value,
    /// than the actual field, the `value` is guaranteed to be a reference to the real value.
    pub value: &'a dyn FieldValue,

    /// A reference to the value casted to `Reflect`.
    pub reflect_value: &'a dyn Reflect,

    /// A property is not meant to be edited.
    pub read_only: bool,

    /// Only for dynamic collections (Vec, etc) - means that its size cannot be changed, however the
    /// _items_ of the collection can still be changed.
    pub immutable_collection: bool,

    /// A minimal value of the property. Works only with numeric properties!
    pub min_value: Option<f64>,

    /// A minimal value of the property. Works only with numeric properties!
    pub max_value: Option<f64>,

    /// A minimal value of the property. Works only with numeric properties!
    pub step: Option<f64>,

    /// Maximum amount of decimal places for a numeric property.
    pub precision: Option<usize>,
}

impl<'a, 'b> FieldInfo<'a, 'b> {
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

impl<'a, 'b> fmt::Debug for FieldInfo<'a, 'b> {
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

impl<'a, 'b> PartialEq<Self> for FieldInfo<'a, 'b> {
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

pub trait ReflectBase: Any + Debug {
    fn as_any_raw(&self) -> &dyn Any;
    fn as_any_raw_mut(&mut self) -> &mut dyn Any;
}

impl<T: Reflect> ReflectBase for T {
    fn as_any_raw(&self) -> &dyn Any {
        self
    }

    fn as_any_raw_mut(&mut self) -> &mut dyn Any {
        self
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
/// - `#[reflect(field = <method call>)]`
/// - `#[reflect(field_mut = <method call>)]`
///
/// # Additional Trait Bounds
///
/// `Reflect` restricted to types that implement `Debug` trait, this is needed to convert the actual value
/// to string. `Display` isn't used here, because it can't be derived and it is very tedious to implement it
/// for every type that should support `Reflect` trait. It is a good compromise between development speed
/// and the quality of the string output.
pub trait Reflect: ReflectBase {
    fn source_path() -> &'static str
    where
        Self: Sized;

    fn type_name(&self) -> &'static str;

    fn doc(&self) -> &'static str;

    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo]));

    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any));

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any));

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect));

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect));

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>>;

    /// Returns a parent assembly name of the type that implements this trait. **WARNING:** You should use
    /// proc-macro (`#[derive(Reflect)]`) to ensure that this method will return correct assembly
    /// name. In other words - there's no guarantee, that any implementation other than proc-macro
    /// will return a correct name of the assembly. Alternatively, you can use `env!("CARGO_PKG_NAME")`
    /// as an implementation.
    fn assembly_name(&self) -> &'static str;

    /// Returns a parent assembly name of the type that implements this trait. **WARNING:** You should use
    /// proc-macro (`#[derive(Reflect)]`) to ensure that this method will return correct assembly
    /// name. In other words - there's no guarantee, that any implementation other than proc-macro
    /// will return a correct name of the assembly. Alternatively, you can use `env!("CARGO_PKG_NAME")`
    /// as an implementation.
    fn type_assembly_name() -> &'static str
    where
        Self: Sized;

    /// Calls user method specified with `#[reflect(setter = ..)]` or falls back to
    /// [`Reflect::field_mut`]
    #[allow(clippy::type_complexity)]
    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),
    ) {
        let mut opt_value = Some(value);
        self.field_mut(field, &mut move |field| {
            let value = opt_value.take().unwrap();
            match field {
                Some(f) => func(f.set(value)),
                None => func(Err(value)),
            };
        });
    }

    fn fields(&self, func: &mut dyn FnMut(&[&dyn Reflect])) {
        func(&[])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
        func(&mut [])
    }

    fn field(
        &self,
        #[allow(unused_variables)] name: &str,
        func: &mut dyn FnMut(Option<&dyn Reflect>),
    ) {
        func(None)
    }

    fn field_mut(
        &mut self,
        #[allow(unused_variables)] name: &str,
        func: &mut dyn FnMut(Option<&mut dyn Reflect>),
    ) {
        func(None)
    }

    fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
        func(None)
    }

    fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
        func(None)
    }

    fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
        func(None)
    }

    fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
        func(None)
    }

    fn as_inheritable_variable(
        &self,
        func: &mut dyn FnMut(Option<&dyn ReflectInheritableVariable>),
    ) {
        func(None)
    }

    fn as_inheritable_variable_mut(
        &mut self,
        func: &mut dyn FnMut(Option<&mut dyn ReflectInheritableVariable>),
    ) {
        func(None)
    }

    fn as_hash_map(&self, func: &mut dyn FnMut(Option<&dyn ReflectHashMap>)) {
        func(None)
    }

    fn as_hash_map_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectHashMap>)) {
        func(None)
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

pub trait ReflectHashMap: Reflect {
    fn reflect_insert(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>>;
    fn reflect_len(&self) -> usize;
    fn reflect_get(&self, key: &dyn Reflect, func: &mut dyn FnMut(Option<&dyn Reflect>));
    fn reflect_get_mut(
        &mut self,
        key: &dyn Reflect,
        func: &mut dyn FnMut(Option<&mut dyn Reflect>),
    );
    fn reflect_get_nth_value_ref(&self, index: usize) -> Option<&dyn Reflect>;
    fn reflect_get_nth_value_mut(&mut self, index: usize) -> Option<&mut dyn Reflect>;
    fn reflect_get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)>;
    fn reflect_get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)>;
    fn reflect_remove(&mut self, key: &dyn Reflect, func: &mut dyn FnMut(Option<Box<dyn Reflect>>));
}

pub trait ReflectInheritableVariable: Reflect + Debug {
    /// Tries to inherit a value from parent. It will succeed only if the current variable is
    /// not marked as modified.
    fn try_inherit(
        &mut self,
        parent: &dyn ReflectInheritableVariable,
        ignored_types: &[TypeId],
    ) -> Result<Option<Box<dyn Reflect>>, InheritError>;

    /// Resets modified flag from the variable.
    fn reset_modified_flag(&mut self);

    /// Returns current variable flags.
    fn flags(&self) -> VariableFlags;

    fn set_flags(&mut self, flags: VariableFlags);

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
    fn resolve_path<'p>(
        &self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&dyn Reflect, ReflectPathError<'p>>),
    );

    fn resolve_path_mut<'p>(
        &mut self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectPathError<'p>>),
    );

    fn get_resolve_path<'p, T: Reflect>(
        &self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&T, ReflectPathError<'p>>),
    ) {
        self.resolve_path(path, &mut |resolve_result| {
            match resolve_result {
                Ok(value) => {
                    value.downcast_ref(&mut |result| {
                        match result {
                            Some(value) => {
                                func(Ok(value));
                            }
                            None => {
                                func(Err(ReflectPathError::InvalidDowncast));
                            }
                        };
                    });
                }
                Err(err) => {
                    func(Err(err));
                }
            };
        })
    }

    fn get_resolve_path_mut<'p, T: Reflect>(
        &mut self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&mut T, ReflectPathError<'p>>),
    ) {
        self.resolve_path_mut(path, &mut |result| match result {
            Ok(value) => value.downcast_mut(&mut |result| match result {
                Some(value) => func(Ok(value)),
                None => func(Err(ReflectPathError::InvalidDowncast)),
            }),
            Err(err) => func(Err(err)),
        })
    }
}

impl<T: Reflect> ResolvePath for T {
    fn resolve_path<'p>(
        &self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&dyn Reflect, ReflectPathError<'p>>),
    ) {
        (self as &dyn Reflect).resolve_path(path, func)
    }

    fn resolve_path_mut<'p>(
        &mut self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectPathError<'p>>),
    ) {
        (self as &mut dyn Reflect).resolve_path_mut(path, func)
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
    fn get_field<T: 'static>(&self, name: &str, func: &mut dyn FnMut(Option<&T>));

    fn get_field_mut<T: 'static>(&mut self, _name: &str, func: &mut dyn FnMut(Option<&mut T>));
}

impl<R: Reflect> GetField for R {
    fn get_field<T: 'static>(&self, name: &str, func: &mut dyn FnMut(Option<&T>)) {
        self.field(name, &mut |field| match field {
            None => func(None),
            Some(reflect) => reflect.as_any(&mut |any| func(any.downcast_ref())),
        })
    }

    fn get_field_mut<T: 'static>(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut T>)) {
        self.field_mut(name, &mut |field| match field {
            None => func(None),
            Some(reflect) => reflect.as_any_mut(&mut |any| func(any.downcast_mut())),
        })
    }
}

// --------------------------------------------------------------------------------
// impl dyn Trait
// --------------------------------------------------------------------------------

// SAFETY: String usage is safe in immutable contexts only. Calling `ManuallyDrop::drop`
// (running strings destructor) on the returned value will cause crash!
unsafe fn make_fake_string_from_slice(string: &str) -> ManuallyDrop<String> {
    ManuallyDrop::new(String::from_utf8_unchecked(Vec::from_raw_parts(
        string.as_bytes().as_ptr() as *mut _,
        string.as_bytes().len(),
        string.as_bytes().len(),
    )))
}

fn try_fetch_by_str_path_ref(
    hash_map: &dyn ReflectHashMap,
    path: &str,
    func: &mut dyn FnMut(Option<&dyn Reflect>),
) {
    // Create fake string here first, this is needed to avoid memory allocations..
    // SAFETY: We won't drop the fake string or mutate it.
    let fake_string_key = unsafe { make_fake_string_from_slice(path) };

    hash_map.reflect_get(&*fake_string_key, &mut |result| match result {
        Some(value) => func(Some(value)),
        None => hash_map.reflect_get(&ImmutableString::new(path) as &dyn Reflect, func),
    });
}

fn try_fetch_by_str_path_mut(
    hash_map: &mut dyn ReflectHashMap,
    path: &str,
    func: &mut dyn FnMut(Option<&mut dyn Reflect>),
) {
    // Create fake string here first, this is needed to avoid memory allocations..
    // SAFETY: We won't drop the fake string or mutate it.
    let fake_string_key = unsafe { make_fake_string_from_slice(path) };

    let mut succeeded = true;

    hash_map.reflect_get_mut(&*fake_string_key, &mut |result| match result {
        Some(value) => func(Some(value)),
        None => succeeded = false,
    });

    if !succeeded {
        hash_map.reflect_get_mut(&ImmutableString::new(path) as &dyn Reflect, func)
    }
}

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

    fn resolve(
        &self,
        reflect: &dyn Reflect,
        func: &mut dyn FnMut(Result<&dyn Reflect, ReflectPathError<'p>>),
    ) {
        match self {
            Self::Field(path) => reflect.field(path, &mut |field| {
                func(field.ok_or(ReflectPathError::UnknownField { s: path }))
            }),
            Self::Index(path) => {
                reflect.as_array(&mut |result| match result {
                    Some(array) => match path.parse::<usize>() {
                        Ok(index) => match array.reflect_index(index) {
                            None => func(Err(ReflectPathError::NoItemForIndex { s: path })),
                            Some(value) => func(Ok(value)),
                        },
                        Err(_) => func(Err(ReflectPathError::InvalidIndexSyntax { s: path })),
                    },
                    None => reflect.as_hash_map(&mut |result| match result {
                        Some(hash_map) => {
                            try_fetch_by_str_path_ref(hash_map, path, &mut |result| {
                                func(result.ok_or(ReflectPathError::NoItemForIndex { s: path }))
                            })
                        }
                        None => func(Err(ReflectPathError::NotAnArray)),
                    }),
                });
            }
        }
    }

    fn resolve_mut(
        &self,
        reflect: &mut dyn Reflect,
        func: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectPathError<'p>>),
    ) {
        match self {
            Self::Field(path) => reflect.field_mut(path, &mut |field| {
                func(field.ok_or(ReflectPathError::UnknownField { s: path }))
            }),
            Self::Index(path) => {
                let mut succeeded = true;
                reflect.as_array_mut(&mut |array| match array {
                    Some(list) => match path.parse::<usize>() {
                        Ok(index) => match list.reflect_index_mut(index) {
                            None => func(Err(ReflectPathError::NoItemForIndex { s: path })),
                            Some(value) => func(Ok(value)),
                        },
                        Err(_) => func(Err(ReflectPathError::InvalidIndexSyntax { s: path })),
                    },
                    None => succeeded = false,
                });

                if !succeeded {
                    reflect.as_hash_map_mut(&mut |result| match result {
                        Some(hash_map) => {
                            try_fetch_by_str_path_mut(hash_map, path, &mut |result| {
                                func(result.ok_or(ReflectPathError::NoItemForIndex { s: path }))
                            })
                        }
                        None => func(Err(ReflectPathError::NotAnArray)),
                    })
                }
            }
        }
    }
}

impl ResolvePath for dyn Reflect {
    fn resolve_path<'p>(
        &self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&dyn Reflect, ReflectPathError<'p>>),
    ) {
        match Component::next(path) {
            Ok((component, r)) => component.resolve(self, &mut |result| match result {
                Ok(child) => {
                    if r.is_empty() {
                        func(Ok(child))
                    } else {
                        child.resolve_path(r, func)
                    }
                }
                Err(err) => func(Err(err)),
            }),
            Err(err) => func(Err(err)),
        }
    }

    fn resolve_path_mut<'p>(
        &mut self,
        path: &'p str,
        func: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectPathError<'p>>),
    ) {
        match Component::next(path) {
            Ok((component, r)) => component.resolve_mut(self, &mut |result| match result {
                Ok(child) => {
                    if r.is_empty() {
                        func(Ok(child))
                    } else {
                        child.resolve_path_mut(r, func)
                    }
                }
                Err(err) => func(Err(err)),
            }),
            Err(err) => func(Err(err)),
        }
    }
}

pub enum SetFieldByPathError<'p> {
    InvalidPath {
        value: Box<dyn Reflect>,
        reason: ReflectPathError<'p>,
    },
    InvalidValue(Box<dyn Reflect>),
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
    pub fn downcast_ref<T: Reflect>(&self, func: &mut dyn FnMut(Option<&T>)) {
        self.as_any(&mut |any| func(any.downcast_ref::<T>()))
    }

    #[inline]
    pub fn downcast_mut<T: Reflect>(&mut self, func: &mut dyn FnMut(Option<&mut T>)) {
        self.as_any_mut(&mut |any| func(any.downcast_mut::<T>()))
    }

    /// Sets a field by its path in the given entity. This method always uses [`Reflect::set_field`] which means,
    /// that it will always call custom property setters.
    #[inline]
    pub fn set_field_by_path<'p>(
        &mut self,
        path: &'p str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, SetFieldByPathError<'p>>),
    ) {
        if let Some(separator_position) = path.rfind('.') {
            let mut opt_value = Some(value);
            let parent_path = &path[..separator_position];
            let field = &path[(separator_position + 1)..];
            self.resolve_path_mut(parent_path, &mut |result| match result {
                Err(reason) => {
                    func(Err(SetFieldByPathError::InvalidPath {
                        reason,
                        value: opt_value.take().unwrap(),
                    }));
                }
                Ok(property) => {
                    property.set_field(field, opt_value.take().unwrap(), &mut |result| match result
                    {
                        Ok(value) => func(Ok(value)),
                        Err(e) => func(Err(SetFieldByPathError::InvalidValue(e))),
                    })
                }
            });
        } else {
            self.set_field(path, value, &mut |result| match result {
                Ok(value) => func(Ok(value)),
                Err(e) => func(Err(SetFieldByPathError::InvalidValue(e))),
            });
        }
    }

    pub fn enumerate_fields_recursively<F>(&self, func: &mut F, ignored_types: &[TypeId])
    where
        F: FnMut(&str, Option<&FieldInfo>, &dyn Reflect),
    {
        self.enumerate_fields_recursively_internal("", None, func, ignored_types)
    }

    fn enumerate_fields_recursively_internal<F>(
        &self,
        path: &str,
        field_info: Option<&FieldInfo>,
        func: &mut F,
        ignored_types: &[TypeId],
    ) where
        F: FnMut(&str, Option<&FieldInfo>, &dyn Reflect),
    {
        if ignored_types.contains(&self.type_id()) {
            return;
        }

        func(path, field_info, self);

        let mut done = false;

        self.as_inheritable_variable(&mut |variable| {
            if let Some(variable) = variable {
                // Inner variable might also contain inheritable variables, so continue iterating.
                variable
                    .inner_value_ref()
                    .enumerate_fields_recursively_internal(path, field_info, func, ignored_types);

                done = true;
            }
        });

        if done {
            return;
        }

        self.as_array(&mut |array| {
            if let Some(array) = array {
                for i in 0..array.reflect_len() {
                    if let Some(item) = array.reflect_index(i) {
                        let item_path = format!("{path}[{i}]");

                        item.enumerate_fields_recursively_internal(
                            &item_path,
                            field_info,
                            func,
                            ignored_types,
                        );
                    }
                }

                done = true;
            }
        });

        if done {
            return;
        }

        self.as_hash_map(&mut |hash_map| {
            if let Some(hash_map) = hash_map {
                for i in 0..hash_map.reflect_len() {
                    if let Some((key, value)) = hash_map.reflect_get_at(i) {
                        // TODO: Here we just using `Debug` impl to obtain string representation for keys. This is
                        // fine for most cases in the engine.
                        let mut key_str = format!("{:?}", key);

                        let mut is_key_string = false;
                        key.downcast_ref::<String>(&mut |string| is_key_string |= string.is_some());
                        key.downcast_ref::<ImmutableString>(&mut |string| {
                            is_key_string |= string.is_some()
                        });

                        if is_key_string {
                            // Strip quotes at the beginning and the end, because Debug impl for String adds
                            // quotes at the beginning and the end, but we want raw value.
                            // TODO: This is unreliable mechanism.
                            key_str.remove(0);
                            key_str.pop();
                        }

                        let item_path = format!("{}[{}]", path, key_str);

                        value.enumerate_fields_recursively_internal(
                            &item_path,
                            field_info,
                            func,
                            ignored_types,
                        );
                    }
                }

                done = true;
            }
        });

        if done {
            return;
        }

        self.fields_info(&mut |fields| {
            for field in fields {
                let compound_path;
                let field_path = if path.is_empty() {
                    field.name
                } else {
                    compound_path = format!("{}.{}", path, field.name);
                    &compound_path
                };

                field.reflect_value.enumerate_fields_recursively_internal(
                    field_path,
                    Some(field),
                    func,
                    ignored_types,
                );
            }
        })
    }

    pub fn apply_recursively<F>(&self, func: &mut F, ignored_types: &[TypeId])
    where
        F: FnMut(&dyn Reflect),
    {
        if ignored_types.contains(&(*self).type_id()) {
            return;
        }

        func(self);

        let mut done = false;

        self.as_inheritable_variable(&mut |variable| {
            if let Some(variable) = variable {
                // Inner variable might also contain inheritable variables, so continue iterating.
                variable
                    .inner_value_ref()
                    .apply_recursively(func, ignored_types);

                done = true;
            }
        });

        if done {
            return;
        }

        self.as_array(&mut |array| {
            if let Some(array) = array {
                for i in 0..array.reflect_len() {
                    if let Some(item) = array.reflect_index(i) {
                        item.apply_recursively(func, ignored_types);
                    }
                }

                done = true;
            }
        });

        if done {
            return;
        }

        self.as_hash_map(&mut |hash_map| {
            if let Some(hash_map) = hash_map {
                for i in 0..hash_map.reflect_len() {
                    if let Some(item) = hash_map.reflect_get_nth_value_ref(i) {
                        item.apply_recursively(func, ignored_types);
                    }
                }

                done = true;
            }
        });

        if done {
            return;
        }

        self.fields(&mut |fields| {
            for field in fields {
                field.apply_recursively(func, ignored_types);
            }
        })
    }

    pub fn apply_recursively_mut<F>(&mut self, func: &mut F, ignored_types: &[TypeId])
    where
        F: FnMut(&mut dyn Reflect),
    {
        if ignored_types.contains(&(*self).type_id()) {
            return;
        }

        func(self);

        let mut done = false;

        self.as_inheritable_variable_mut(&mut |variable| {
            if let Some(variable) = variable {
                // Inner variable might also contain inheritable variables, so continue iterating.
                variable
                    .inner_value_mut()
                    .apply_recursively_mut(func, ignored_types);

                done = true;
            }
        });

        if done {
            return;
        }

        self.as_array_mut(&mut |array| {
            if let Some(array) = array {
                for i in 0..array.reflect_len() {
                    if let Some(item) = array.reflect_index_mut(i) {
                        item.apply_recursively_mut(func, ignored_types);
                    }
                }

                done = true;
            }
        });

        if done {
            return;
        }

        self.as_hash_map_mut(&mut |hash_map| {
            if let Some(hash_map) = hash_map {
                for i in 0..hash_map.reflect_len() {
                    if let Some(item) = hash_map.reflect_get_nth_value_mut(i) {
                        item.apply_recursively_mut(func, ignored_types);
                    }
                }

                done = true;
            }
        });

        if done {
            return;
        }

        self.fields_mut(&mut |fields| {
            for field in fields {
                (*field).apply_recursively_mut(func, ignored_types);
            }
        })
    }
}

pub fn is_path_to_array_element(path: &str) -> bool {
    path.chars().last().map_or(false, |c| c == ']')
}

// Make it a trait?
impl dyn ReflectList {
    pub fn get_reflect_index<T: Reflect + 'static>(
        &self,
        index: usize,
        func: &mut dyn FnMut(Option<&T>),
    ) {
        if let Some(reflect) = self.reflect_index(index) {
            reflect.downcast_ref(func)
        } else {
            func(None)
        }
    }

    pub fn get_reflect_index_mut<T: Reflect + 'static>(
        &mut self,
        index: usize,
        func: &mut dyn FnMut(Option<&mut T>),
    ) {
        if let Some(reflect) = self.reflect_index_mut(index) {
            reflect.downcast_mut(func)
        } else {
            func(None)
        }
    }
}

#[macro_export]
macro_rules! blank_reflect {
    () => {
        fn source_path() -> &'static str {
            file!()
        }

        fn type_name(&self) -> &'static str {
            std::any::type_name::<Self>()
        }

        fn doc(&self) -> &'static str {
            ""
        }

        fn assembly_name(&self) -> &'static str {
            env!("CARGO_PKG_NAME")
        }

        fn type_assembly_name() -> &'static str {
            env!("CARGO_PKG_NAME")
        }

        fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
            func(&[])
        }

        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            self
        }

        fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
            func(self)
        }

        fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
            func(self)
        }

        fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
            func(self)
        }

        fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
            func(self)
        }

        fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
            func(if name == "self" { Some(self) } else { None })
        }

        fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
            func(if name == "self" { Some(self) } else { None })
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
        fn source_path() -> &'static str {
            file!()
        }

        fn type_name(&self) -> &'static str {
            self.deref().type_name()
        }

        fn doc(&self) -> &'static str {
            self.deref().doc()
        }

        fn assembly_name(&self) -> &'static str {
            self.deref().assembly_name()
        }

        fn type_assembly_name() -> &'static str {
            env!("CARGO_PKG_NAME")
        }

        fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
            self.deref().fields_info(func)
        }

        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            (*self).into_any()
        }

        fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
            self.deref().as_any(func)
        }

        fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
            self.deref_mut().as_any_mut(func)
        }

        fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
            self.deref().as_reflect(func)
        }

        fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
            self.deref_mut().as_reflect_mut(func)
        }

        fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
            self.deref_mut().set(value)
        }

        fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
            self.deref().field(name, func)
        }

        fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
            self.deref_mut().field_mut(name, func)
        }

        fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
            self.deref().as_array(func)
        }

        fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
            self.deref_mut().as_array_mut(func)
        }

        fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
            self.deref().as_list(func)
        }

        fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
            self.deref_mut().as_list_mut(func)
        }
    };
}

#[macro_export]
macro_rules! newtype_reflect {
    () => {
        fn type_name(&self) -> &'static str {
            self.0.type_name()
        }

        fn doc(&self) -> &'static str {
            self.0.doc()
        }

        fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
            self.0.fields_info(func)
        }

        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            self
        }

        fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
            self.0.as_any(func)
        }

        fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
            self.0.as_any_mut(func)
        }

        fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
            self.0.as_reflect(func)
        }

        fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
            self.0.as_reflect_mut(func)
        }

        fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
            self.0.set(value)
        }

        fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
            self.0.field(name, func)
        }

        fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
            self.0.field_mut(name, func)
        }

        fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
            self.0.as_array(func)
        }

        fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
            self.0.as_array_mut(func)
        }

        fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
            self.0.as_list(func)
        }

        fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
            self.0.as_list_mut(func)
        }

        fn as_inheritable_variable(
            &self,
            func: &mut dyn FnMut(Option<&dyn ReflectInheritableVariable>),
        ) {
            self.0.as_inheritable_variable(func)
        }

        fn as_inheritable_variable_mut(
            &mut self,
            func: &mut dyn FnMut(Option<&mut dyn ReflectInheritableVariable>),
        ) {
            self.0.as_inheritable_variable_mut(func)
        }

        fn as_hash_map(&self, func: &mut dyn FnMut(Option<&dyn ReflectHashMap>)) {
            self.0.as_hash_map(func)
        }

        fn as_hash_map_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectHashMap>)) {
            self.0.as_hash_map_mut(func)
        }
    };
}

use crate::sstorage::ImmutableString;
use crate::variable::{InheritError, VariableFlags};
pub use blank_reflect;
pub use delegate_reflect;

#[cfg(test)]
mod test {
    use super::prelude::*;
    use std::collections::HashMap;

    #[derive(Reflect, Default, Debug)]
    struct Foo {
        bar: Bar,
        baz: f32,
        collection: Vec<Item>,
        hash_map: HashMap<String, Item>,
    }

    #[derive(Reflect, Default, Debug)]
    struct Item {
        payload: u32,
    }

    #[derive(Reflect, Default, Debug)]
    struct Bar {
        stuff: String,
    }

    #[test]
    fn enumerate_fields_recursively() {
        let foo = Foo {
            bar: Default::default(),
            baz: 0.0,
            collection: vec![Item::default()],
            hash_map: [("Foobar".to_string(), Item::default())].into(),
        };

        let mut names = Vec::new();
        (&foo as &dyn Reflect).enumerate_fields_recursively(
            &mut |path, _, _| {
                names.push(path.to_string());
            },
            &[],
        );

        assert_eq!(names[0], "");
        assert_eq!(names[1], "bar");
        assert_eq!(names[2], "bar.stuff");
        assert_eq!(names[3], "baz");
        assert_eq!(names[4], "collection");
        assert_eq!(names[5], "collection[0]");
        assert_eq!(names[6], "collection[0].payload");
        assert_eq!(names[7], "hash_map");
        assert_eq!(names[8], "hash_map[Foobar]");
        assert_eq!(names[9], "hash_map[Foobar].payload");
    }
}
