//! A wrapper for a variable that hold additional flag that tells that initial value was changed in runtime.
//!
//! For more info see [`InheritableVariable`]

use crate::{
    inspect::{Inspect, PropertyInfo},
    reflect::{Reflect, ReflectArray, ReflectInheritableVariable, ReflectList},
    visitor::prelude::*,
};
use bitflags::bitflags;
use std::{
    any::{Any, TypeId},
    cell::Cell,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

bitflags! {
    /// A set of possible variable flags.
    #[derive(Reflect)]
    pub struct VariableFlags: u8 {
        /// Nothing.
        const NONE = 0;
        /// A variable was externally modified.
        const MODIFIED = 0b0000_0001;
        /// A variable must be synced with respective variable from data model.
        const NEED_SYNC = 0b0000_0010;
    }
}

/// An error that could occur during inheritance.
#[derive(Debug)]
pub enum InheritError {
    /// Types of properties mismatch.
    TypesMismatch {
        /// Type of left property.
        left_type: TypeId,
        /// Type of right property.
        right_type: TypeId,
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
#[derive(Debug)]
pub struct InheritableVariable<T> {
    value: T,
    flags: Cell<VariableFlags>,
}

impl<T: Clone> Clone for InheritableVariable<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            flags: self.flags.clone(),
        }
    }
}

impl<T> From<T> for InheritableVariable<T> {
    fn from(v: T) -> Self {
        InheritableVariable::new(v)
    }
}

impl<T: PartialEq> PartialEq for InheritableVariable<T> {
    fn eq(&self, other: &Self) -> bool {
        // `custom` flag intentionally ignored!
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for InheritableVariable<T> {}

impl<T: Default> Default for InheritableVariable<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            flags: Cell::new(VariableFlags::NONE),
        }
    }
}

impl<T: Clone> InheritableVariable<T> {
    /// Clones wrapped value.
    pub fn clone_inner(&self) -> T {
        self.value.clone()
    }

    /// Tries to sync a value in a data model with a value in the inheritable variable. The value
    /// will be synced only if it was marked as needs sync.
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
    /// Creates new non-modified variable from given value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            flags: Cell::new(VariableFlags::NONE),
        }
    }

    /// Creates new variable from given value and marks it with [`VariableFlags::MODIFIED`] flag.
    pub fn new_modified(value: T) -> Self {
        Self {
            value,
            flags: Cell::new(VariableFlags::MODIFIED),
        }
    }

    /// Creates new variable from a given value with custom flags.
    pub fn new_with_flags(value: T, flags: VariableFlags) -> Self {
        Self {
            value,
            flags: Cell::new(flags),
        }
    }

    /// Replaces value and also raises the [`VariableFlags::MODIFIED`] flag.
    pub fn set(&mut self, value: T) -> T {
        self.mark_modified_and_need_sync();
        std::mem::replace(&mut self.value, value)
    }

    /// Replaces value and flags.
    pub fn set_with_flags(&mut self, value: T, flags: VariableFlags) -> T {
        self.flags.set(flags);
        std::mem::replace(&mut self.value, value)
    }

    /// Replaces current value without marking the variable modified.
    pub fn set_silent(&mut self, value: T) -> T {
        std::mem::replace(&mut self.value, value)
    }

    /// Returns true if the respective data model's variable must be synced.
    pub fn need_sync(&self) -> bool {
        self.flags.get().contains(VariableFlags::NEED_SYNC)
    }

    /// Returns a reference to the wrapped value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the wrapped value.
    ///
    /// # Important notes.
    ///
    /// The method raises `modified` flag, no matter if actual modification was made!
    pub fn get_mut(&mut self) -> &mut T {
        self.mark_modified_and_need_sync();
        &mut self.value
    }

    /// Returns a mutable reference to the wrapped value.
    ///
    /// # Important notes.
    ///
    /// This method does not mark the value as modified!
    pub fn get_mut_silent(&mut self) -> &mut T {
        &mut self.value
    }

    /// Returns true if variable was modified and should not be overwritten during property inheritance.
    pub fn is_modified(&self) -> bool {
        self.flags.get().contains(VariableFlags::MODIFIED)
    }

    /// Marks value as modified, so its value won't be overwritten during property inheritance.
    pub fn mark_modified(&mut self) {
        self.flags
            .get_mut()
            .insert(VariableFlags::MODIFIED | VariableFlags::NEED_SYNC);
    }

    fn mark_modified_and_need_sync(&mut self) {
        self.flags
            .get_mut()
            .insert(VariableFlags::MODIFIED | VariableFlags::NEED_SYNC);
    }
}

impl<T> Deref for InheritableVariable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for InheritableVariable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_modified_and_need_sync();
        &mut self.value
    }
}

impl<T> Visit for InheritableVariable<T>
where
    T: Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.value.visit("Value", &mut region)?;
        self.flags.get_mut().bits.visit("Flags", &mut region)?;

        Ok(())
    }
}

impl<T> Inspect for InheritableVariable<T>
where
    T: Inspect,
{
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        self.value.properties()
    }
}

impl<T> Reflect for InheritableVariable<T>
where
    T: Reflect + Clone + PartialEq + Debug,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.value).into_any()
    }

    fn as_any(&self) -> &dyn Any {
        self.value.as_any()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.value.as_any_mut()
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self.value.as_reflect()
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self.value.as_reflect_mut()
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.value.set(value)
    }

    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.value.set_field(field, value)
    }

    fn field(&self, name: &str) -> Option<&dyn Reflect> {
        self.value.field(name)
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn Reflect> {
        self.value.field_mut(name)
    }

    fn as_array(&self) -> Option<&dyn ReflectArray> {
        self.value.as_array()
    }

    fn as_array_mut(&mut self) -> Option<&mut dyn ReflectArray> {
        self.value.as_array_mut()
    }

    fn as_list(&self) -> Option<&dyn ReflectList> {
        self.value.as_list()
    }

    fn as_list_mut(&mut self) -> Option<&mut dyn ReflectList> {
        self.value.as_list_mut()
    }

    fn as_inheritable_variable(&self) -> Option<&dyn ReflectInheritableVariable> {
        Some(self)
    }

    fn as_inheritable_variable_mut(&mut self) -> Option<&mut dyn ReflectInheritableVariable> {
        Some(self)
    }
}

impl<T> ReflectInheritableVariable for InheritableVariable<T>
where
    T: Reflect + Clone + PartialEq + Debug,
{
    fn try_inherit(
        &mut self,
        parent: &dyn ReflectInheritableVariable,
    ) -> Result<Option<Box<dyn Reflect>>, InheritError> {
        // Cast directly to inner type, because any type that implements ReflectInheritableVariable,
        // has delegating methods for almost every method of Reflect trait implementation.
        if let Some(parent_value) = parent.as_reflect().downcast_ref::<T>() {
            if !self.is_modified() {
                Ok(Some(Box::new(std::mem::replace(
                    &mut self.value,
                    parent_value.clone(),
                ))))
            } else {
                Ok(None)
            }
        } else {
            Err(InheritError::TypesMismatch {
                left_type: TypeId::of::<Self>(),
                right_type: parent.type_id(),
            })
        }
    }

    fn reset_modified_flag(&mut self) {
        self.flags.get_mut().remove(VariableFlags::MODIFIED)
    }

    fn flags(&self) -> VariableFlags {
        self.flags.get()
    }

    fn is_modified(&self) -> bool {
        self.is_modified()
    }

    fn value_equals(&self, other: &dyn ReflectInheritableVariable) -> bool {
        other
            .as_reflect()
            .downcast_ref::<T>()
            .map_or(false, |other| &self.value == other)
    }

    fn clone_value_box(&self) -> Box<dyn Reflect> {
        Box::new(self.value.clone())
    }

    fn mark_modified(&mut self) {
        self.mark_modified()
    }
}

/// Simultaneously walks over fields of given child and parent and tries to inherit values of properties
/// of child with parent's properties. It is done recursively for every fields in entities.
pub fn try_inherit_properties(
    child: &mut dyn Reflect,
    parent: &dyn Reflect,
) -> Result<(), InheritError> {
    if (*child).type_id() != (*parent).type_id() {
        return Err(InheritError::TypesMismatch {
            left_type: (*child).type_id(),
            right_type: (*parent).type_id(),
        });
    }

    for (child_field, parent_field) in child.fields_mut().iter_mut().zip(parent.fields()) {
        // If both fields are InheritableVariable<T>, try to inherit.
        if let (Some(child_inheritable_field), Some(parent_inheritable_field)) = (
            child_field.as_inheritable_variable_mut(),
            parent_field.as_inheritable_variable(),
        ) {
            child_inheritable_field.try_inherit(parent_inheritable_field)?;
        }

        // Look into inner properties recursively and try to inherit them. This is mandatory step, because inner
        // fields may also be InheritableVariable<T>.
        try_inherit_properties(child_field.as_reflect_mut(), parent_field.as_reflect())?;
    }

    Ok(())
}

pub fn reset_inheritable_properties(object: &mut dyn Reflect) {
    for field in object.fields_mut() {
        if let Some(inheritable_field) = field.as_inheritable_variable_mut() {
            inheritable_field.reset_modified_flag();
        }

        reset_inheritable_properties(field);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        reflect::{Reflect, ReflectInheritableVariable},
        variable::{try_inherit_properties, InheritableVariable},
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
                value: InheritableVariable::new(1.23),
            },
            other_value: InheritableVariable::new("Foobar".to_string()),
        };

        let mut child = parent.clone();

        // Try inherit non-modified, the result objects must be equal.
        try_inherit_properties(&mut child, &parent).unwrap();
        assert_eq!(parent, child);

        // Then modify parent's and child's values.
        parent.other_value.set("Baz".to_string());
        assert!(ReflectInheritableVariable::is_modified(&parent.other_value),);

        child.foo.value.set(3.21);
        assert!(ReflectInheritableVariable::is_modified(&child.foo.value));

        try_inherit_properties(&mut child, &parent).unwrap();

        // This property reflects parent's changes, because it is non-modified.
        assert_eq!(child.other_value.value, "Baz".to_string());
        // This property must remain unchanged, because it is modified.
        assert_eq!(child.foo.value.value, 3.21);
    }

    #[test]
    fn test_inheritable_variable_equality() {
        let va = InheritableVariable::new(1.23);
        let vb = InheritableVariable::new(1.23);

        assert!(va.value_equals(&vb))
    }

    #[derive(Reflect)]
    enum SomeEnum {
        Bar(InheritableVariable<f32>),
        Baz {
            foo: InheritableVariable<f32>,
            foobar: InheritableVariable<u32>,
        },
    }

    #[test]
    fn test_enum_inheritance_tuple() {
        let mut child = SomeEnum::Bar(InheritableVariable::new(1.23));
        let parent = SomeEnum::Bar(InheritableVariable::new(3.21));

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        if let SomeEnum::Bar(value) = child {
            assert_eq!(*value, 3.21);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn test_enum_inheritance_struct() {
        let mut child = SomeEnum::Baz {
            foo: InheritableVariable::new(1.23),
            foobar: InheritableVariable::new(123),
        };
        let parent = SomeEnum::Baz {
            foo: InheritableVariable::new(3.21),
            foobar: InheritableVariable::new(321),
        };

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        if let SomeEnum::Baz { foo, foobar } = child {
            assert_eq!(*foo, 3.21);
            assert_eq!(*foobar, 321);
        } else {
            unreachable!()
        }
    }
}
