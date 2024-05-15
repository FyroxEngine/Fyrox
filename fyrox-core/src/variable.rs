//! A wrapper for a variable that hold additional flag that tells that initial value was changed in runtime.
//!
//! For more info see [`InheritableVariable`]

use crate::{
    reflect::prelude::*,
    visitor::{prelude::*, VisitorFlags},
};
use bitflags::bitflags;
use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Reflect, Copy, Clone, Ord, PartialOrd, PartialEq, Eq)]
#[repr(transparent)]
pub struct VariableFlags(u8);

bitflags! {
    impl VariableFlags: u8 {
        /// Nothing.
        const NONE = 0;
        /// A variable was externally modified.
        const MODIFIED = 0b0000_0001;
        /// A variable must be synced with respective variable from data model.
        const NEED_SYNC = 0b0000_0010;
    }
}

impl Debug for VariableFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == VariableFlags::NONE {
            write!(f, "NONE")
        } else {
            for (i, flag) in self.iter().enumerate() {
                if i != 0 {
                    write!(f, "|")?
                }
                match flag {
                    VariableFlags::MODIFIED => write!(f, "MOD")?,
                    VariableFlags::NEED_SYNC => write!(f, "SYNC")?,
                    _ => {}
                }
            }
            Ok(())
        }
    }
}

/// An error that could occur during inheritance.
#[derive(Debug)]
pub enum InheritError {
    /// Types of properties mismatch.
    TypesMismatch {
        /// Type of left property.
        left_type: &'static str,
        /// Type of right property.
        right_type: &'static str,
    },
}

/// A wrapper for a variable that hold additional flag that tells that initial value was changed in runtime.
///
/// InheritableVariables are used for resource inheritance system. Resource inheritance may just sound weird,
/// but the idea behind it is very simple - take property values from parent resource if the value in current
/// hasn't changed in runtime.
///
/// To get better understanding, let's look at very simple example. Imagine you have a scene with a 3d model
/// instance. Now you realizes that the 3d model has a misplaced object and you need to fix it, you open a
/// 3D modelling software (Blender, 3Ds max, etc) and move the object to a correct spot and re-save the 3D model.
/// The question is: what should happen with the instance of the object in the scene? Logical answer would be:
/// if it hasn't been modified, then just take the new position from the 3D model. This is where inheritable
/// variable comes into play. If you've change the value of such variable, it will remember changes and the object
/// will stay on its new position instead of changed.
///
/// # Deref and DerefMut
///
/// Access via Deref provides access to inner variable. **DerefMut marks variable as modified** and returns a
/// mutable reference to inner variable.
pub struct InheritableVariable<T> {
    value: T,
    flags: Cell<VariableFlags>,
}

impl<T: Debug> Debug for InheritableVariable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} (flags:{:?})", self.value, self.flags.get())
    }
}

impl<T: Clone> Clone for InheritableVariable<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            flags: self.flags.clone(),
        }
    }
}

impl<T> From<T> for InheritableVariable<T> {
    #[inline]
    fn from(v: T) -> Self {
        InheritableVariable::new_modified(v)
    }
}

impl<T: PartialEq> PartialEq for InheritableVariable<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // `custom` flag intentionally ignored!
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for InheritableVariable<T> {}

impl<T: Default> Default for InheritableVariable<T> {
    #[inline]
    fn default() -> Self {
        Self {
            value: T::default(),
            flags: Cell::new(VariableFlags::MODIFIED),
        }
    }
}

impl<T: Clone> InheritableVariable<T> {
    /// Clones wrapped value.
    #[inline]
    pub fn clone_inner(&self) -> T {
        self.value.clone()
    }

    /// Tries to sync a value in a data model with a value in the inheritable variable. The value
    /// will be synced only if it was marked as needs sync.
    #[inline]
    pub fn try_sync_model<S: FnOnce(T)>(&self, setter: S) -> bool {
        if self.need_sync() {
            // Drop flag first.
            let mut flags = self.flags.get();
            flags.remove(VariableFlags::NEED_SYNC);
            self.flags.set(flags);

            // Set new value in a data model.
            (setter)(self.value.clone());

            true
        } else {
            false
        }
    }
}

impl<T> InheritableVariable<T> {
    /// Creates new modified variable from given value. This method should always be used to create inheritable
    /// variables in the engine.
    #[inline]
    pub fn new_modified(value: T) -> Self {
        Self {
            value,
            flags: Cell::new(VariableFlags::MODIFIED),
        }
    }

    /// Creates new variable without any flags set.
    #[inline]
    pub fn new_non_modified(value: T) -> Self {
        Self {
            value,
            flags: Cell::new(VariableFlags::NONE),
        }
    }

    /// Creates new variable from a given value with custom flags.
    #[inline]
    pub fn new_with_flags(value: T, flags: VariableFlags) -> Self {
        Self {
            value,
            flags: Cell::new(flags),
        }
    }

    /// Replaces value and also raises the [`VariableFlags::MODIFIED`] flag.
    #[inline]
    pub fn set_value_and_mark_modified(&mut self, value: T) -> T {
        self.mark_modified_and_need_sync();
        std::mem::replace(&mut self.value, value)
    }

    /// Replaces value and flags.
    #[inline]
    pub fn set_value_with_flags(&mut self, value: T, flags: VariableFlags) -> T {
        self.flags.set(flags);
        std::mem::replace(&mut self.value, value)
    }

    /// Replaces current value without marking the variable modified.
    #[inline]
    pub fn set_value_silent(&mut self, value: T) -> T {
        std::mem::replace(&mut self.value, value)
    }

    /// Returns true if the respective data model's variable must be synced.
    #[inline]
    pub fn need_sync(&self) -> bool {
        self.flags.get().contains(VariableFlags::NEED_SYNC)
    }

    /// Returns a reference to the wrapped value.
    #[inline]
    pub fn get_value_ref(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the wrapped value.
    ///
    /// # Important notes.
    ///
    /// The method raises `modified` flag, no matter if actual modification was made!
    #[inline]
    pub fn get_value_mut_and_mark_modified(&mut self) -> &mut T {
        self.mark_modified_and_need_sync();
        &mut self.value
    }

    /// Returns a mutable reference to the wrapped value.
    ///
    /// # Important notes.
    ///
    /// This method does not mark the value as modified!
    #[inline]
    pub fn get_value_mut_silent(&mut self) -> &mut T {
        &mut self.value
    }

    /// Returns true if variable was modified and should not be overwritten during property inheritance.
    #[inline]
    pub fn is_modified(&self) -> bool {
        self.flags.get().contains(VariableFlags::MODIFIED)
    }

    /// Marks value as modified, so its value won't be overwritten during property inheritance.
    #[inline]
    pub fn mark_modified(&mut self) {
        self.flags
            .get_mut()
            .insert(VariableFlags::MODIFIED | VariableFlags::NEED_SYNC);
    }

    /// Deconstructs the variable and returns the wrapped value.
    #[inline]
    pub fn take(self) -> T {
        self.value
    }

    #[inline]
    fn mark_modified_and_need_sync(&mut self) {
        self.flags
            .get_mut()
            .insert(VariableFlags::MODIFIED | VariableFlags::NEED_SYNC);
    }
}

impl<T> Deref for InheritableVariable<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for InheritableVariable<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_modified_and_need_sync();
        &mut self.value
    }
}

/// Special non-derived implementation of Visit to account for the special needs of InheritableVariable from Visitors.
impl<T> Visit for InheritableVariable<T>
where
    T: Visit,
{
    /// Read or write this value, depending on whether [Visitor::is_reading()] is true or false.
    /// InheritableVariable uses the visit method in a very special way. Rather than just directly
    /// visiting the inner value and flags of the InheritableVariable, it allows for several distinct possibilities.
    ///
    /// # Cases when the visitor is reading:
    ///
    /// 1. If the visitor is reading, InheritableVariable allows for the possibilities that the data being read
    /// is not an InheritableVariable but is data of type T. It uses this data to set the inner value
    /// and adds [VariableFlags::MODIFIED] to [InheritableVariable::flags].
    ///
    /// 2. The data for this InheritableVariable may be missing entirely from the given visitor.
    /// If so, then leave inner value unmodified and remove the `MODIFIED` flag from `flags`.
    ///
    /// # Cases when the visitor is writing:
    ///
    /// 1. If the visitor is writing and the `MODIFIED` flag is not set, then InheritableVariable writes **nothing at all.**
    /// It does not even write an empty region.
    ///
    /// 2. If the visitor is writing and the `MODIFIED` flag is set, then the InheritableVariable writes itself to the Visitor
    /// as if InheritableVariable were a normal struct, writing a Field for "Flags" and causing `value` to write itself.
    ///
    /// If the [VisitorFlags::SERIALIZE_EVERYTHING] flag is set in the [Visitor::flags], this causes the InheritableVariable to act
    /// as if its `MODIFIED` flag were set.
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut visited = false;

        if visitor.is_reading() {
            // Try to visit inner value first, this is very useful if user decides to make their
            // variable inheritable, but still keep backward compatibility.
            visited = self.value.visit(name, visitor).is_ok();
            self.flags.get_mut().insert(VariableFlags::MODIFIED);
        }

        if !visited {
            if visitor.is_reading() {
                // The entire region could be missing if the variable wasn't modified.
                if let Ok(mut region) = visitor.enter_region(name) {
                    let _ = self.value.visit("Value", &mut region);
                    self.flags.get_mut().0.visit("Flags", &mut region)?;
                } else {
                    // Default flags contains `modified` flag, we need to remove it if there's no
                    // region at all.
                    self.flags.get_mut().remove(VariableFlags::MODIFIED);
                }
            } else if self.flags.get().contains(VariableFlags::MODIFIED)
                || visitor.flags.contains(VisitorFlags::SERIALIZE_EVERYTHING)
            {
                let mut region = visitor.enter_region(name)?;
                self.value.visit("Value", &mut region)?;
                self.flags.get_mut().0.visit("Flags", &mut region)?;
            } else {
                // Non-modified variables do not write anything.
            }
        }

        Ok(())
    }
}

impl<T> Reflect for InheritableVariable<T>
where
    T: Reflect + Clone + PartialEq + Debug,
{
    #[inline]
    fn source_path() -> &'static str {
        file!()
    }

    #[inline]
    fn type_name(&self) -> &'static str {
        self.value.type_name()
    }

    #[inline]
    fn doc(&self) -> &'static str {
        self.value.doc()
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    #[inline]
    fn fields_info(&self, func: &mut dyn FnMut(&[FieldInfo])) {
        self.value.fields_info(func)
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.value).into_any()
    }

    #[inline]
    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        self.value.as_any(func)
    }

    #[inline]
    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        self.value.as_any_mut(func)
    }

    #[inline]
    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        self.value.as_reflect(func)
    }

    #[inline]
    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        self.value.as_reflect_mut(func)
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.mark_modified_and_need_sync();
        self.value.set(value)
    }

    #[inline]
    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, Box<dyn Reflect>>),
    ) {
        self.mark_modified_and_need_sync();
        self.value.set_field(field, value, func)
    }

    #[inline]
    fn fields(&self, func: &mut dyn FnMut(&[&dyn Reflect])) {
        self.value.fields(func)
    }

    #[inline]
    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [&mut dyn Reflect])) {
        self.value.fields_mut(func)
    }

    #[inline]
    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.value.field(name, func)
    }

    #[inline]
    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        // Any modifications inside of compound structs must mark the variable as modified.
        self.mark_modified_and_need_sync();
        self.value.field_mut(name, func)
    }

    #[inline]
    fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
        self.value.as_array(func)
    }

    #[inline]
    fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
        // Any modifications inside of inheritable arrays must mark the variable as modified.
        self.mark_modified_and_need_sync();
        self.value.as_array_mut(func)
    }

    #[inline]
    fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
        self.value.as_list(func)
    }

    #[inline]
    fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
        // Any modifications inside of inheritable lists must mark the variable as modified.
        self.mark_modified_and_need_sync();
        self.value.as_list_mut(func)
    }

    #[inline]
    fn as_inheritable_variable(
        &self,
        func: &mut dyn FnMut(Option<&dyn ReflectInheritableVariable>),
    ) {
        func(Some(self))
    }

    #[inline]
    fn as_inheritable_variable_mut(
        &mut self,
        func: &mut dyn FnMut(Option<&mut dyn ReflectInheritableVariable>),
    ) {
        func(Some(self))
    }
}

impl<T> ReflectInheritableVariable for InheritableVariable<T>
where
    T: Reflect + Clone + PartialEq + Debug,
{
    fn try_inherit(
        &mut self,
        parent: &dyn ReflectInheritableVariable,
        ignored_types: &[TypeId],
    ) -> Result<Option<Box<dyn Reflect>>, InheritError> {
        let mut result: Result<Option<Box<dyn Reflect>>, InheritError> = Ok(None);

        match parent.inner_value_ref().as_any_raw().downcast_ref::<T>() {
            Some(parent_value) => {
                if !self.is_modified() {
                    let mut parent_value_clone = parent_value.clone();

                    mark_inheritable_properties_non_modified(
                        &mut parent_value_clone,
                        ignored_types,
                    );

                    result = Ok(Some(Box::new(std::mem::replace(
                        &mut self.value,
                        parent_value_clone,
                    ))));
                }
            }
            None => {
                result = Err(InheritError::TypesMismatch {
                    left_type: self.inner_value_ref().type_name(),
                    right_type: parent.inner_value_ref().type_name(),
                });
            }
        }

        result
    }

    #[inline]
    fn reset_modified_flag(&mut self) {
        self.flags.get_mut().remove(VariableFlags::MODIFIED)
    }

    #[inline]
    fn flags(&self) -> VariableFlags {
        self.flags.get()
    }

    #[inline]
    fn set_flags(&mut self, flags: VariableFlags) {
        self.flags.set(flags)
    }

    #[inline]
    fn is_modified(&self) -> bool {
        self.is_modified()
    }

    #[inline]
    fn value_equals(&self, other: &dyn ReflectInheritableVariable) -> bool {
        let mut output_result = false;
        other.as_reflect(&mut |reflect| {
            reflect.downcast_ref::<T>(&mut |result| {
                output_result = match result {
                    Some(other) => &self.value == other,
                    None => false,
                };
            })
        });
        output_result
    }

    #[inline]
    fn clone_value_box(&self) -> Box<dyn Reflect> {
        Box::new(self.value.clone())
    }

    #[inline]
    fn mark_modified(&mut self) {
        self.mark_modified()
    }

    #[inline]
    fn inner_value_mut(&mut self) -> &mut dyn Reflect {
        &mut self.value
    }

    #[inline]
    fn inner_value_ref(&self) -> &dyn Reflect {
        &self.value
    }
}

/// Simultaneously walks over fields of given child and parent and tries to inherit values of properties
/// of child with parent's properties. It is done recursively for every fields in entities.
///
/// ## How it works
///
/// In general, it uses reflection to iterate over child and parent properties and trying to inherit values.
/// Child's field will take parent's field value only if child's field is **non-modified**. There are one
/// edge case in inheritance: collections.
///
/// Inheritance for collections itself works the same as described above, however the content of collections
/// can only be inherited if their sizes are equal. Also, since inheritance uses plain copy of inner data of
/// inheritable variables, it works in a special way.
///
/// ```text
/// Child                                       Parent (root)
///     InheritableVariableA            <-         InheritableVariableA*
///     InheritableCollection*          ->         InheritableCollection*
///         Item0                                       Item0
///             InheritableVariableB*   ->                  InheritableVariableB*
///             InheritableVariableC    <-                  InheritableVariableC*
///         Item1                                       Item1
///             ..                                          ..
///         ..                                          ..
///         ItemN                                       ItemN
///             ..                                          ..
///
/// * - means that the variable was modified
/// ```
///
/// At first, `InheritableVariableA` will be copied from the parent as usual. Next, the inheritable collection
/// won't be copied (because it is modified), however its items will be inherited separately.
/// `InheritableVariableB` won't be copied either (since it is modified too), but `InheritableVariableC` **will**
/// be copied from parent.
pub fn try_inherit_properties(
    child: &mut dyn Reflect,
    parent: &dyn Reflect,
    ignored_types: &[TypeId],
) -> Result<(), InheritError> {
    let child_type_id = (*child).type_id();
    let parent_type_id = (*parent).type_id();

    if ignored_types.contains(&child_type_id) || ignored_types.contains(&parent_type_id) {
        return Ok(());
    }

    if child_type_id != parent_type_id {
        return Err(InheritError::TypesMismatch {
            left_type: (*child).type_name(),
            right_type: (*parent).type_name(),
        });
    }

    let mut result = None;

    child.as_inheritable_variable_mut(&mut |inheritable_child| {
        if let Some(inheritable_child) = inheritable_child {
            parent.as_inheritable_variable(&mut |inheritable_parent| {
                if let Some(inheritable_parent) = inheritable_parent {
                    if let Err(e) = inheritable_child.try_inherit(inheritable_parent, ignored_types)
                    {
                        result = Some(Err(e));
                    }

                    if !matches!(result, Some(Err(_))) {
                        result = Some(try_inherit_properties(
                            inheritable_child.inner_value_mut(),
                            inheritable_parent.inner_value_ref(),
                            ignored_types,
                        ));
                    }
                }
            })
        }
    });

    if result.is_none() {
        child.as_array_mut(&mut |child_collection| {
            if let Some(child_collection) = child_collection {
                parent.as_array(&mut |parent_collection| {
                    if let Some(parent_collection) = parent_collection {
                        if child_collection.reflect_len() == parent_collection.reflect_len() {
                            for i in 0..child_collection.reflect_len() {
                                // Sparse arrays (like Pool) could have empty entries.
                                if let (Some(child_item), Some(parent_item)) = (
                                    child_collection.reflect_index_mut(i),
                                    parent_collection.reflect_index(i),
                                ) {
                                    if let Err(e) = try_inherit_properties(
                                        child_item,
                                        parent_item,
                                        ignored_types,
                                    ) {
                                        result = Some(Err(e));

                                        break;
                                    }
                                }
                            }
                        }
                    }
                })
            }
        })
    }

    if result.is_none() {
        child.fields_mut(&mut |child_fields| {
            parent.fields(&mut |parent_fields| {
                for (child_field, parent_field) in child_fields.iter_mut().zip(parent_fields) {
                    // Look into inner properties recursively and try to inherit them. This is mandatory step, because inner
                    // fields may also be InheritableVariable<T>.
                    if let Err(e) =
                        try_inherit_properties(*child_field, *parent_field, ignored_types)
                    {
                        result = Some(Err(e));
                    }

                    if matches!(result, Some(Err(_))) {
                        break;
                    }
                }
            })
        });
    }

    result.unwrap_or(Ok(()))
}

pub fn do_with_inheritable_variables<F>(
    root: &mut dyn Reflect,
    func: &mut F,
    ignored_types: &[TypeId],
) where
    F: FnMut(&mut dyn ReflectInheritableVariable),
{
    root.apply_recursively_mut(
        &mut |object| {
            object.as_inheritable_variable_mut(&mut |variable| {
                if let Some(variable) = variable {
                    func(variable);
                }
            });
        },
        ignored_types,
    )
}

pub fn mark_inheritable_properties_non_modified(
    object: &mut dyn Reflect,
    ignored_types: &[TypeId],
) {
    do_with_inheritable_variables(
        object,
        &mut |variable| variable.reset_modified_flag(),
        ignored_types,
    );
}

pub fn mark_inheritable_properties_modified(object: &mut dyn Reflect, ignored_types: &[TypeId]) {
    do_with_inheritable_variables(
        object,
        &mut |variable| variable.mark_modified(),
        ignored_types,
    );
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;
    use std::{cell::Cell, ops::DerefMut};

    use crate::{
        reflect::{prelude::*, ReflectInheritableVariable},
        variable::{try_inherit_properties, InheritableVariable, VariableFlags},
        visitor::{Visit, Visitor},
    };

    #[derive(Reflect, Clone, Debug, PartialEq)]
    struct Foo {
        value: InheritableVariable<f32>,
    }

    #[derive(Reflect, Clone, Debug, PartialEq)]
    struct Bar {
        foo: Foo,

        other_value: InheritableVariable<String>,
    }

    #[test]
    fn test_property_inheritance_via_reflection() {
        let mut parent = Bar {
            foo: Foo {
                value: InheritableVariable::new_non_modified(1.23),
            },
            other_value: InheritableVariable::new_non_modified("Foobar".to_string()),
        };

        let mut child = parent.clone();

        // Try inherit non-modified, the result objects must be equal.
        try_inherit_properties(&mut child, &parent, &[]).unwrap();
        assert_eq!(parent, child);

        // Then modify parent's and child's values.
        parent
            .other_value
            .set_value_and_mark_modified("Baz".to_string());
        assert!(ReflectInheritableVariable::is_modified(&parent.other_value),);

        child.foo.value.set_value_and_mark_modified(3.21);
        assert!(ReflectInheritableVariable::is_modified(&child.foo.value));

        try_inherit_properties(&mut child, &parent, &[]).unwrap();

        // This property reflects parent's changes, because it is non-modified.
        assert_eq!(child.other_value.value, "Baz".to_string());
        // This property must remain unchanged, because it is modified.
        assert_eq!(child.foo.value.value, 3.21);
    }

    #[test]
    fn test_inheritable_variable_equality() {
        let va = InheritableVariable::new_non_modified(1.23);
        let vb = InheritableVariable::new_non_modified(1.23);

        assert!(va.value_equals(&vb))
    }

    #[derive(Reflect, Debug)]
    enum SomeEnum {
        Bar(InheritableVariable<f32>),
        Baz {
            foo: InheritableVariable<f32>,
            foobar: InheritableVariable<u32>,
        },
    }

    #[test]
    fn test_enum_inheritance_tuple() {
        let mut child = SomeEnum::Bar(InheritableVariable::new_non_modified(1.23));
        let parent = SomeEnum::Bar(InheritableVariable::new_non_modified(3.21));

        try_inherit_properties(&mut child, &parent, &[]).unwrap();

        if let SomeEnum::Bar(value) = child {
            assert_eq!(*value, 3.21);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn test_enum_inheritance_struct() {
        let mut child = SomeEnum::Baz {
            foo: InheritableVariable::new_non_modified(1.23),
            foobar: InheritableVariable::new_non_modified(123),
        };
        let parent = SomeEnum::Baz {
            foo: InheritableVariable::new_non_modified(3.21),
            foobar: InheritableVariable::new_non_modified(321),
        };

        try_inherit_properties(&mut child, &parent, &[]).unwrap();

        if let SomeEnum::Baz { foo, foobar } = child {
            assert_eq!(*foo, 3.21);
            assert_eq!(*foobar, 321);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn test_collection_inheritance() {
        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct Foo {
            some_data: f32,
        }

        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct CollectionItem {
            foo: InheritableVariable<Foo>,
            bar: InheritableVariable<u32>,
        }

        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct MyEntity {
            collection: InheritableVariable<Vec<CollectionItem>>,
        }

        let parent = MyEntity {
            collection: InheritableVariable::new_modified(vec![CollectionItem {
                foo: InheritableVariable::new_modified(Foo { some_data: 123.321 }),
                bar: InheritableVariable::new_modified(321),
            }]),
        };

        let mut child = MyEntity {
            collection: InheritableVariable::new_modified(vec![CollectionItem {
                foo: InheritableVariable::new_modified(Foo { some_data: 321.123 }),
                bar: InheritableVariable::new_non_modified(321),
            }]),
        };

        try_inherit_properties(&mut child, &parent, &[]).unwrap();

        // Flags must be transferred correctly.
        let item = &child.collection[0];
        assert!(!item.bar.is_modified());
        assert_eq!(item.bar.value, 321);

        assert_eq!(item.foo.value, Foo { some_data: 321.123 });
        assert!(item.foo.is_modified());
    }

    #[test]
    fn test_compound_inheritance() {
        #[derive(Reflect, Clone, Debug, PartialEq, Eq)]
        struct SomeComplexData {
            foo: InheritableVariable<u32>,
        }

        #[derive(Reflect, Clone, Debug, PartialEq)]
        struct MyEntity {
            some_field: InheritableVariable<f32>,

            // This field won't be inherited correctly - at first it will take parent's value and then
            // will try to inherit inner fields, but its is useless step, because inner data is already
            // a full copy of parent's field value. This absolutely ok, it just indicates issues in user
            // code.
            incorrectly_inheritable_data: InheritableVariable<SomeComplexData>,

            // Subfields of this field will be correctly inherited, because the field itself is not inheritable.
            inheritable_data: SomeComplexData,
        }

        let mut child = MyEntity {
            some_field: InheritableVariable::new_non_modified(1.23),
            incorrectly_inheritable_data: InheritableVariable::new_non_modified(SomeComplexData {
                foo: InheritableVariable::new_modified(222),
            }),
            inheritable_data: SomeComplexData {
                foo: InheritableVariable::new_modified(222),
            },
        };

        let parent = MyEntity {
            some_field: InheritableVariable::new_non_modified(3.21),
            incorrectly_inheritable_data: InheritableVariable::new_non_modified(SomeComplexData {
                foo: InheritableVariable::new_non_modified(321),
            }),
            inheritable_data: SomeComplexData {
                foo: InheritableVariable::new_modified(321),
            },
        };

        assert!(try_inherit_properties(&mut child, &parent, &[]).is_ok());

        assert_eq!(child.some_field.value, 3.21);
        // These fields are equal, despite the fact that they're marked as modified.
        // This is due incorrect usage of inheritance.
        assert_eq!(
            child.incorrectly_inheritable_data.foo.value,
            parent.incorrectly_inheritable_data.foo.value
        );
        // These fields are not equal, as it should be.
        assert_ne!(
            child.inheritable_data.foo.value,
            parent.inheritable_data.foo.value
        );
    }

    #[test]
    fn inheritable_variable_from_t() {
        assert_eq!(
            InheritableVariable::from(42),
            InheritableVariable {
                value: 42,
                ..Default::default()
            }
        );
    }

    #[test]
    fn default_for_inheritable_variable() {
        assert_eq!(
            InheritableVariable::<i32>::default(),
            InheritableVariable {
                value: 0,
                flags: Cell::new(VariableFlags::MODIFIED),
            }
        );
    }

    #[test]
    fn inheritable_variable_clone_inner() {
        let v = InheritableVariable::from(42);

        assert_eq!(v.clone_inner(), 42);
    }

    #[test]
    fn inheritable_variable_try_sync_model() {
        let v = InheritableVariable::from(42);
        assert!(!v.try_sync_model(|s| println!("{}", s)));

        let v = InheritableVariable::new_with_flags(42, VariableFlags::NEED_SYNC);
        assert!(v.try_sync_model(|s| println!("{}", s)));
    }

    #[test]
    fn inheritable_variable_new_with_flags() {
        let v = InheritableVariable::new_with_flags(42, VariableFlags::MODIFIED);

        assert_eq!(
            v,
            InheritableVariable {
                value: 42,
                flags: Cell::new(VariableFlags::MODIFIED),
            }
        );
    }

    #[test]
    fn inheritable_variable_set_value_with_flags() {
        let mut v = InheritableVariable::from(42);
        let res = v.set_value_with_flags(15, VariableFlags::NEED_SYNC);

        assert_eq!(res, 42);
        assert_eq!(
            v,
            InheritableVariable {
                value: 15,
                flags: Cell::new(VariableFlags::NEED_SYNC),
            }
        );
    }

    #[test]
    fn inheritable_variable_set_value_silent() {
        let mut v = InheritableVariable::from(42);
        let res = v.set_value_silent(15);

        assert_eq!(res, 42);
        assert_eq!(
            v,
            InheritableVariable {
                value: 15,
                flags: Cell::new(VariableFlags::MODIFIED),
            }
        );
    }

    #[test]
    fn inheritable_variable_need_sync() {
        let v = InheritableVariable::from(42);
        assert!(!v.need_sync());

        let v = InheritableVariable::new_with_flags(42, VariableFlags::NEED_SYNC);
        assert!(v.need_sync());
    }

    #[test]
    fn inheritable_variable_get_value_ref() {
        let v = InheritableVariable::from(42);

        assert_eq!(v.get_value_ref(), &42);
    }

    #[test]
    fn inheritable_variable_get_value_mut_and_mark_modified() {
        let mut v = InheritableVariable::from(42);

        assert_eq!(v.get_value_mut_and_mark_modified(), &mut 42);
        assert_eq!(
            v,
            InheritableVariable {
                value: 42,
                flags: Cell::new(VariableFlags::MODIFIED),
            }
        );
    }

    #[test]
    fn inheritable_variable_get_value_mut_silent() {
        let mut v = InheritableVariable::from(42);

        assert_eq!(v.get_value_mut_silent(), &mut 42);
    }

    #[test]
    fn inheritable_variable_is_modified() {
        let v = InheritableVariable::new_with_flags(42, VariableFlags::NONE);
        assert!(!v.is_modified());

        let v = InheritableVariable::new_with_flags(42, VariableFlags::MODIFIED);
        assert!(v.is_modified());
    }

    #[test]
    fn inheritable_variable_mark_modified() {
        let mut v = InheritableVariable::new_with_flags(42, VariableFlags::NONE);
        v.mark_modified();

        assert_eq!(
            v,
            InheritableVariable {
                value: 42,
                flags: Cell::new(VariableFlags::MODIFIED),
            }
        );
    }

    #[test]
    fn inheritable_variable_take() {
        let v = InheritableVariable::from(42);

        assert_eq!(v.take(), 42);
    }

    #[test]
    fn deref_mut_for_inheritable_variable() {
        let mut v = InheritableVariable::new_with_flags(42, VariableFlags::NONE);
        let res = v.deref_mut();

        assert_eq!(res, &mut 42);
        assert_eq!(
            v,
            InheritableVariable {
                value: 42,
                flags: Cell::new(VariableFlags::MODIFIED),
            }
        );
    }

    #[test]
    fn visit_for_inheritable_variable() {
        let mut v = InheritableVariable::from(42);
        let mut visitor = Visitor::default();

        assert!(v.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn inheritable_variable_type_name() {
        let v = InheritableVariable::from(42);

        assert_eq!(v.type_name(), "i32");
    }

    #[test]
    fn inheritable_variable_doc() {
        let v = InheritableVariable::from(42);

        assert_eq!(v.doc(), "");
    }

    #[test]
    fn inheritable_variable_flags() {
        let v = InheritableVariable::new_with_flags(42, VariableFlags::NONE);

        assert_eq!(v.flags(), VariableFlags::NONE);
    }

    #[test]
    fn inheritable_variable_set_flags() {
        let mut v = InheritableVariable::new_with_flags(42, VariableFlags::NONE);
        v.set_flags(VariableFlags::NEED_SYNC);

        assert_eq!(v.flags(), VariableFlags::NEED_SYNC);
    }

    #[test]
    fn inheritable_variable_ref_cell() {
        let v = InheritableVariable::new_modified(RefCell::new(123u32));
        assert_eq!(
            v.inner_value_ref()
                .as_any_raw()
                .downcast_ref::<RefCell<u32>>(),
            Some(&RefCell::new(123u32))
        );
    }
}
