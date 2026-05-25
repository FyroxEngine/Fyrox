// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Runtime reflection

mod array;
mod error;
mod field;
mod handle;
mod impls;
mod inherit;
mod macros;
mod map;

use crate::sstorage::ImmutableString;

pub use fyrox_core_derive::Reflect;
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    mem::ManuallyDrop,
};

pub use array::*;
pub use error::*;
pub use field::*;
pub use handle::*;
pub use inherit::*;
pub use macros::*;
pub use map::*;

pub mod prelude {
    pub use super::{
        FieldMetadata, FieldMut, FieldRef, Reflect, ReflectArray, ReflectHashMap,
        ReflectInheritableVariable, ReflectList, ResolvePath, SetFieldByPathError, SetFieldError,
        TypeInfo,
    };
}

/// A set of useful information about a type that can be queried at runtime.
pub struct TypeInfo {
    /// Name of the file where the type is located. Usually, it is just `file!()` macro call result.
    pub source_path: &'static str,

    /// The actual name of the type in human-readable form. Typically, it just contains the result
    /// of calling `std::any::type_name::<T>()`.
    pub type_name: &'static str,

    /// A parent assembly name of the type that implements this trait.
    ///
    /// ## Important notes for implementors
    ///
    /// Use proc-macro (`#[derive(Reflect)]`) to ensure that this field contains the correct assembly
    /// name. In other words - there's no guarantee, that any implementation other than proc-macro
    /// will return a correct name of the assembly. Alternatively, you can use `env!("CARGO_PKG_NAME")`
    /// as an implementation.
    pub assembly_name: &'static str,

    /// A documentation of the type.
    pub doc_comment: &'static str,

    /// A set of types that are derived from this type. Derived here does not mean classic OOPs
    /// derived class, it is just a "link" between this type an others. It is widely used to link
    /// an actual type with its wrapper type (which typically contains `Box<dyn SomeTrait>`).
    pub derived_types: &'static [TypeId],
}

/// A trait for runtime reflection.
///
/// ## Code Generation
///
/// The derive macro is available under `#[reflect(...)]` attribute that can be placed on both
/// the type and its fields.
///
/// ### Type attributes
///
/// - `#[reflect(hide_all)]` - hide all fields from reflection.
/// - `#[reflect(bounds)]` - add type boundary for `Reflect` impl, for example
/// `#[reflect(bounds = "T: Reflect + Clone")]`
/// - `#[reflect(non_cloneable)]` - prevent the macro from generating an implementation of
/// [`Self::try_clone_box`] trait for your type. Could be useful for non-cloneable types.
/// - `#[reflect(derived_type = "Type")]` - marks the type for which the attribute is added as a
/// subtype for the `Type`.
///
/// ### Direct vs Indirect Access
///
/// There are two kinds of methods that can be used to access internals of an object. The preferred
/// one is called indirect - such methods accepts a closure that will be called by the method. These
/// methods support types with interior mutability (Mutex, RefCell, etc.), but cannot give you a
/// direct reference outside the provided closure. This is essential limitation to support types with
/// interior mutability (for example, mutex requires holding some sort of a lock guard while accessing
/// its content). Indirect access can be quite annoying to use, and it is possible to get direct
/// access to the fields by the price of not supporting types with interior mutability.
///
/// ### Field attributes
///
/// - `#[reflect(hidden)]` - hides the field from reflection.
/// - `#[reflect(setter = "foo")]` - set the desired method that will be used by [`Self::set_field`]
/// default implementation.
/// - `#[reflect(deref)]` - delegate the field access with `deref` + `deref_mut` calls. Could be
/// useful for new-type objects.
/// - `#[reflect(field = "foo")]` - sets the desired method, that will be used to access
/// the field.
/// - `#[reflect(field_mut = "foo")]` - sets the desired method, that will be used to access
/// the field.
/// - `#[reflect(name = "name")]` - overrides the name of the field.
/// - `#[reflect(display_name = "name")]` - sets the human-readable name for the field.
/// - `#[reflect(tag = "tag")]` - sets some arbitrary string tag of the field. It could be used to
/// group properties by a certain criteria or to find a specific property by its tag.
/// - `#[reflect(read_only)]` - the field is not meant to be editable. This flag does not prevent
/// the reflection API from changing the actual value, it is just an instruction for external
/// users (editors, tools, etc.)
/// - `[#reflect(immutable_collection)]` - only for dynamic collections (`Vec`, etc.) - means that its
/// size cannot be changed, however the _items_ of the collection can still be changed.
/// - `#[reflect(min_value = "0.0")]` - minimal value of the field. Works only for numeric fields!
/// - `#[reflect(max_value = "1.0")]` - maximal value of the field. Works only for numeric fields!
/// - `#[reflect(step = "0.1")]` - increment/decrement step of the field. Works only for numeric fields!
/// - `#[reflect(precision = "3")]` - maximum amount of decimal places for a numeric property.
///
/// ### Clone
///
/// By default, the proc macro adds an implementation of [`Self::try_clone_box`] with the assumption
/// that your type implements the [`Clone`] trait. Not all types can implement this trait, in this
/// case, add `#[reflect(non_cloneable)]` attribute for your type. This will force the implementation
/// of [`Self::try_clone_box`] to return `None`.
///
/// ## Additional Trait Bounds
///
/// `Reflect` restricted to types that implement `Debug` trait, this is needed to convert the actual value
/// to string. `Display` isn't used here, because it can't be derived and it is very tedious to implement it
/// for every type that should support `Reflect` trait. It is a good compromise between development speed
/// and the quality of the string output.
pub trait Reflect: Any + Debug {
    /// Returns the info about the type that implements this trait.
    fn type_info() -> TypeInfo
    where
        Self: Sized;

    /// Returns the info about the type that implements this trait.
    fn type_info_ref(&self) -> TypeInfo;

    /// Tries to clone the object and return it as a boxed trait object. This method can return
    /// [`None`] for non-cloneable objects.
    fn try_clone_box(&self) -> Option<Box<dyn Reflect>>;

    /// Calls the given closure with the reference to a slice that contains description for all
    /// fields in the object.
    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef]));

    /// Calls the given closure with the reference to a slice that contains description for all
    /// fields in the object.
    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut]));

    /// Replaces the self value with the specified value. If the call is successful, returns
    /// `Ok(previous_value)`, otherwise returns `Err(specified_value)`.
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>>;

    /// Tries to get a shared reference to a field at the specified index. Returns [`None`] in two cases:
    /// 1) The type does not have such field
    /// 2) The type uses interior mutability. This case is special - pretty much every type with
    ///    interior mutability (Mutex, RefCell, etc.) requires holding some sort of lock guard
    ///    while giving access to its content. This method returns the field reference directly,
    ///    but returning a lock guard would require boxing which in most cases would ruin performance.
    ///    If you need to get a field reference for types with interior mutability, then use
    ///    [`Reflect::fields_ref`] instead.
    fn field_direct_ref(&self, index: usize) -> Option<FieldRef>;

    /// Tries to get a mutable reference to a field at the specified index. Returns [`None`] in two cases:
    /// 1) The type does not have such field
    /// 2) The type uses interior mutability. This case is special - pretty much every type with
    ///    interior mutability (Mutex, RefCell, etc.) requires holding some sort of lock guard
    ///    while giving access to its content. This method returns the field reference directly,
    ///    but returning a lock guard would require boxing which in most cases would ruin performance.
    ///    If you need to get a field reference for types with interior mutability, then use
    ///    [`Reflect::fields_ref`] instead.
    fn field_direct_mut(&mut self, index: usize) -> Option<FieldMut>;

    /// Returns the total number of fields.
    fn fields_count(&self) -> usize {
        let mut count = 0;
        self.fields_ref(&mut |fields| count = fields.len());
        count
    }

    /// Tries to find a field with the specified name and set its value to the specified `value`.
    /// This method may fail in two main reasons:
    /// 1) The field does not exist (or it is hidden via `#[reflect(hidden)]` attribute).
    /// 2) The type of the specified value does match the type of the field at the given name.
    #[allow(clippy::type_complexity)]
    fn set_field(
        &mut self,
        field_name: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, SetFieldError>),
    ) {
        let mut opt_value = Some(value);
        self.find_field_mut(field_name, &mut move |field| {
            let value = opt_value.take().unwrap();
            match field {
                Some(f) => func(f.set(value).map_err(|value| SetFieldError::InvalidValue {
                    field_type_name: f.type_info_ref().type_name,
                    value,
                })),
                None => func(Err(SetFieldError::NoSuchField {
                    name: field_name.to_string(),
                    value,
                })),
            };
        });
    }

    /// Tries to find a field with the given name and calls the specified function with the result
    /// (either `Some(field)` or `None`).
    fn find_field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.fields_ref(&mut |fields| {
            func(
                fields
                    .iter()
                    .find(|field| field.name == name)
                    .map(|field| field.value),
            )
        });
    }

    /// Tries to find a field with the given name and calls the specified function with the result
    /// (either `Some(field)` or `None`).
    fn find_field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        self.fields_mut(&mut |fields| {
            func(
                fields
                    .iter_mut()
                    .find(|field| field.name == name)
                    .map(|field| &mut *field.value),
            )
        });
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

    fn as_hash_map(&self) -> Option<&dyn ReflectHashMap> {
        None
    }

    fn as_hash_map_mut(&mut self) -> Option<&mut dyn ReflectHashMap> {
        None
    }

    fn as_handle(&self) -> Option<&dyn ReflectHandle> {
        None
    }

    fn as_handle_mut(&mut self) -> Option<&mut dyn ReflectHandle> {
        None
    }
}

/// Type-erased API
impl dyn Reflect {
    pub fn downcast<T: Reflect>(self: Box<dyn Reflect>) -> Result<Box<T>, Box<dyn Reflect>> {
        if self.is::<T>() {
            Ok((self as Box<dyn Any>).downcast().unwrap())
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
        (self as &dyn Any).downcast_ref::<T>()
    }

    #[inline]
    pub fn downcast_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }

    /// Tries to find the first field of the given type. This method internally uses
    /// [`Reflect::field_direct_ref`] with all of its limitations.
    #[inline]
    pub fn first_field_ref<T: Reflect>(&self) -> Option<&T> {
        let count = self.fields_count();

        for i in 0..count {
            if let Some(field) = self.field_direct_ref(i) {
                if let Some(typed_field) = (field.value as &dyn Any).downcast_ref::<T>() {
                    return Some(typed_field);
                }
            }
        }

        None
    }

    /// Tries to find the first field of the given type. This method internally uses
    /// [`Reflect::field_direct_ref`] with all of its limitations.
    #[inline]
    pub fn first_field_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        let count = self.fields_count();

        for i in 0..count {
            // SAFETY: Current implementation of borrow checker is just dumb. When a reborrow of self
            // happens in every iteration of the loop, it assigns a new lifetime to the new reference.
            // This way the returned reference has a different lifetime than in the method definition.
            // The following unsafe block reborrows self with the correct lifetime, while the initial
            // reference is not used so this is absolutely safe.
            let this = unsafe { &mut *(self as *mut Self) };
            if let Some(field) = this.field_direct_mut(i) {
                if let Some(typed_field) = (field.value as &mut dyn Any).downcast_mut::<T>() {
                    return Some(typed_field);
                }
            }
        }

        None
    }

    /// Tries to downcast self to the specified type, or if it is not possible, tries to find a
    /// field of the specified type.
    pub fn self_or_field_ref<T: Reflect>(&self) -> Option<&T> {
        if let Some(value) = (self as &dyn Any).downcast_ref::<T>() {
            Some(value)
        } else {
            self.first_field_ref()
        }
    }

    /// Tries to downcast self to the specified type, or if it is not possible, tries to find a
    /// field of the specified type.
    pub fn self_or_field_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        // SAFETY: See the comment in `first_field_mut_of_type` method.
        let this = unsafe { &mut *(self as *mut Self) };
        if let Some(value) = (self as &mut dyn Any).downcast_mut::<T>() {
            Some(value)
        } else {
            this.first_field_mut()
        }
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
                        Err(err) => func(Err(SetFieldByPathError::SetFieldError(err))),
                    })
                }
            });
        } else {
            self.set_field(path, value, &mut |result| match result {
                Ok(value) => func(Ok(value)),
                Err(err) => func(Err(SetFieldByPathError::SetFieldError(err))),
            });
        }
    }

    pub fn enumerate_fields_recursively<F>(&self, func: &mut F, ignored_types: &[TypeId])
    where
        F: FnMut(&str, Option<&FieldRef>, &dyn Reflect),
    {
        self.enumerate_fields_recursively_internal("", None, func, ignored_types)
    }

    fn enumerate_fields_recursively_internal<F>(
        &self,
        path: &str,
        field_info: Option<&FieldRef>,
        func: &mut F,
        ignored_types: &[TypeId],
    ) where
        F: FnMut(&str, Option<&FieldRef>, &dyn Reflect),
    {
        if ignored_types.contains(&self.type_id()) {
            return;
        }

        let mut done = false;

        if let Some(variable) = self.as_inheritable_variable() {
            // Inner variable might also contain inheritable variables, so continue iterating.
            variable
                .inner_value_ref()
                .enumerate_fields_recursively_internal(path, field_info, func, ignored_types);

            done = true;
        }

        if done {
            return;
        }

        func(path, field_info, self);

        if let Some(array) = self.as_array() {
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

        if done {
            return;
        }

        if let Some(hash_map) = self.as_hash_map() {
            for i in 0..hash_map.reflect_len() {
                if let Some((key, value)) = hash_map.reflect_get_at(i) {
                    // TODO: Here we just using `Debug` impl to obtain string representation for keys. This is
                    // fine for most cases in the engine.
                    let mut key_str = format!("{key:?}");

                    let is_key_string = key.downcast_ref::<String>().is_some()
                        || key.downcast_ref::<ImmutableString>().is_some();

                    if is_key_string {
                        // Strip quotes at the beginning and the end, because Debug impl for String adds
                        // quotes at the beginning and the end, but we want raw value.
                        // TODO: This is unreliable mechanism.
                        key_str.remove(0);
                        key_str.pop();
                    }

                    let item_path = format!("{path}[{key_str}]");

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

        if done {
            return;
        }

        self.fields_ref(&mut |fields| {
            for field in fields {
                let compound_path;
                let field_path = if path.is_empty() {
                    field.metadata.name
                } else {
                    compound_path = format!("{}.{}", path, field.metadata.name);
                    &compound_path
                };

                field.value.enumerate_fields_recursively_internal(
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

        if let Some(variable) = self.as_inheritable_variable() {
            // Inner variable might also contain inheritable variables, so continue iterating.
            variable
                .inner_value_ref()
                .apply_recursively(func, ignored_types);

            done = true;
        }

        if done {
            return;
        }

        if let Some(array) = self.as_array() {
            for i in 0..array.reflect_len() {
                if let Some(item) = array.reflect_index(i) {
                    item.apply_recursively(func, ignored_types);
                }
            }

            done = true;
        }

        if done {
            return;
        }

        if let Some(hash_map) = self.as_hash_map() {
            for i in 0..hash_map.reflect_len() {
                if let Some(item) = hash_map.reflect_get_nth_value_ref(i) {
                    item.apply_recursively(func, ignored_types);
                }
            }

            done = true;
        }

        if done {
            return;
        }

        self.fields_ref(&mut |fields| {
            for field_info_ref in fields {
                field_info_ref.value.apply_recursively(func, ignored_types);
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

        if let Some(variable) = self.as_inheritable_variable_mut() {
            // Inner variable might also contain inheritable variables, so continue iterating.
            variable
                .inner_value_mut()
                .apply_recursively_mut(func, ignored_types);

            done = true;
        }

        if done {
            return;
        }

        if let Some(array) = self.as_array_mut() {
            for i in 0..array.reflect_len() {
                if let Some(item) = array.reflect_index_mut(i) {
                    item.apply_recursively_mut(func, ignored_types);
                }
            }

            done = true;
        }

        if done {
            return;
        }

        if let Some(hash_map) = self.as_hash_map_mut() {
            for i in 0..hash_map.reflect_len() {
                if let Some(item) = hash_map.reflect_get_nth_value_mut(i) {
                    item.apply_recursively_mut(func, ignored_types);
                }
            }

            done = true;
        }

        if done {
            return;
        }

        self.fields_mut(&mut |fields| {
            for field_info_mut in fields {
                (*field_info_mut.value).apply_recursively_mut(func, ignored_types);
            }
        })
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
                    match value.downcast_ref::<T>() {
                        Some(value) => {
                            func(Ok(value));
                        }
                        None => {
                            func(Err(ReflectPathError::InvalidDowncast));
                        }
                    };
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
            Ok(value) => match value.downcast_mut() {
                Some(value) => func(Ok(value)),
                None => func(Err(ReflectPathError::InvalidDowncast)),
            },
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
        self.find_field(name, &mut |opt_field| match opt_field {
            None => func(None),
            Some(field) => func((field as &dyn Any).downcast_ref()),
        })
    }

    fn get_field_mut<T: 'static>(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut T>)) {
        self.find_field_mut(name, &mut |opt_field| match opt_field {
            None => func(None),
            Some(field) => func((field as &mut dyn Any).downcast_mut()),
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
        string.len(),
        string.len(),
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
            Self::Field(path) => reflect.find_field(path, &mut |field| {
                func(field.ok_or(ReflectPathError::UnknownField { s: path }))
            }),
            Self::Index(path) => match reflect.as_array() {
                Some(array) => match path.parse::<usize>() {
                    Ok(index) => match array.reflect_index(index) {
                        None => func(Err(ReflectPathError::NoItemForIndex { s: path })),
                        Some(value) => func(Ok(value)),
                    },
                    Err(_) => func(Err(ReflectPathError::InvalidIndexSyntax { s: path })),
                },
                None => match reflect.as_hash_map() {
                    Some(hash_map) => try_fetch_by_str_path_ref(hash_map, path, &mut |result| {
                        func(result.ok_or(ReflectPathError::NoItemForIndex { s: path }))
                    }),
                    None => func(Err(ReflectPathError::NotAnArray)),
                },
            },
        }
    }

    fn resolve_mut(
        &self,
        reflect: &mut dyn Reflect,
        func: &mut dyn FnMut(Result<&mut dyn Reflect, ReflectPathError<'p>>),
    ) {
        match self {
            Self::Field(path) => reflect.find_field_mut(path, &mut |field| {
                func(field.ok_or(ReflectPathError::UnknownField { s: path }))
            }),
            Self::Index(path) => {
                let mut succeeded = true;
                match reflect.as_array_mut() {
                    Some(list) => match path.parse::<usize>() {
                        Ok(index) => match list.reflect_index_mut(index) {
                            None => func(Err(ReflectPathError::NoItemForIndex { s: path })),
                            Some(value) => func(Ok(value)),
                        },
                        Err(_) => func(Err(ReflectPathError::InvalidIndexSyntax { s: path })),
                    },
                    None => succeeded = false,
                }

                if !succeeded {
                    match reflect.as_hash_map_mut() {
                        Some(hash_map) => {
                            try_fetch_by_str_path_mut(hash_map, path, &mut |result| {
                                func(result.ok_or(ReflectPathError::NoItemForIndex { s: path }))
                            })
                        }
                        None => func(Err(ReflectPathError::NotAnArray)),
                    }
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

pub fn is_path_to_array_element(path: &str) -> bool {
    path.ends_with(']')
}

// Make it a trait?
impl dyn ReflectList {
    pub fn get_reflect_index<T: Reflect>(&self, index: usize, func: &mut dyn FnMut(Option<&T>)) {
        if let Some(reflect) = self.reflect_index(index) {
            func(reflect.downcast_ref())
        } else {
            func(None)
        }
    }

    pub fn get_reflect_index_mut<T: Reflect>(
        &mut self,
        index: usize,
        func: &mut dyn FnMut(Option<&mut T>),
    ) {
        if let Some(reflect) = self.reflect_index_mut(index) {
            func(reflect.downcast_mut())
        } else {
            func(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::prelude::*;
    use crate::variable::InheritableVariable;
    use std::any::TypeId;
    use std::collections::HashMap;

    #[derive(Reflect, Clone, Default, PartialEq, Debug)]
    enum Enum {
        #[default]
        Empty,
        Stuff {
            field: u32,
        },
    }

    #[derive(Reflect, Clone, Default, Debug)]
    struct Foo {
        enum_field: InheritableVariable<Enum>,
        bar: Bar,
        baz: f32,
        collection: Vec<Item>,
        hash_map: HashMap<String, Item>,
    }

    #[derive(Reflect, Clone, Default, Debug)]
    struct Item {
        payload: u32,
    }

    #[derive(Reflect, Clone, Default, Debug)]
    struct Bar {
        stuff: String,
    }

    #[test]
    fn enumerate_fields_recursively() {
        let baz = 123.321;

        let foo = Foo {
            enum_field: Enum::Stuff { field: 123 }.into(),
            bar: Default::default(),
            baz,
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

        foo.resolve_path("enum_field.Content.Stuff@field", &mut |result| {
            let enum_field = result.expect("the field must exist!");
            assert_eq!(
                *enum_field
                    .downcast_ref::<u32>()
                    .expect("the type must be u32"),
                123
            );
        });

        assert_eq!(names[0], "");
        assert_eq!(names[1], "enum_field");
        assert_eq!(names[2], "enum_field.Stuff@field");
        assert_eq!(names[3], "bar");
        assert_eq!(names[4], "bar.stuff");
        assert_eq!(names[5], "baz");
        assert_eq!(names[6], "collection");
        assert_eq!(names[7], "collection[0]");
        assert_eq!(names[8], "collection[0].payload");
        assert_eq!(names[9], "hash_map");
        assert_eq!(names[10], "hash_map[Foobar]");
        assert_eq!(names[11], "hash_map[Foobar].payload");

        assert_eq!(foo.fields_count(), 5);

        assert_eq!(
            (&foo as &dyn Reflect).first_field_ref::<f32>().unwrap(),
            &baz
        );
    }

    #[derive(Reflect, Clone, Debug)]
    #[reflect(derived_type = "Derived")]
    struct Base;

    #[allow(dead_code)]
    struct Derived(Box<Base>);

    #[test]
    fn test_derived() {
        let base = Base;
        assert_eq!(
            base.type_info_ref().derived_types,
            &[TypeId::of::<Derived>()]
        )
    }
}
