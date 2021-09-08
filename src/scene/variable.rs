//! A wrapper for a variable that hold additional flag that tells that initial value was changed in runtime.
//!
//! For more info see [`TemplateVariable`]

use crate::core::visitor::{Visit, VisitResult, Visitor};
use std::ops::Deref;

/// A wrapper for a variable that hold additional flag that tells that initial value was changed in runtime.
///
/// TemplateVariables are used for resource inheritance system. Resource inheritance may just sound weird,
/// but the idea behind it is very simple - take property values from parent resource if the value in current
/// hasn't changed in runtime.
///
/// To get better understanding, let's look at very simple example. Imagine you have a scene with a 3d model
/// instance. Now you realizes that the 3d model has a misplaced object and you need to fix it, you open a
/// 3D modelling software (Blender, 3Ds max, etc) and move the object to a correct spot and re-save the 3D model.
/// The question is: what should happen with the instance of the object in the scene? Logical answer would be:
/// if it hasn't been modified, then just take the new position from the 3D model. This is where template
/// variable comes into play. If you've change the value of such variable, it will remember changes and the object
/// will stay on its new position instead of changed.   
#[derive(Debug)]
pub struct TemplateVariable<T> {
    // Actual value.
    value: T,

    // A marker that tells that initial value was changed.
    custom: bool,
}

impl<T: Clone> Clone for TemplateVariable<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            custom: self.custom,
        }
    }
}

impl<T: PartialEq> PartialEq for TemplateVariable<T> {
    fn eq(&self, other: &Self) -> bool {
        // `custom` flag intentionally ignored!
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for TemplateVariable<T> {}

impl<T: Copy> Copy for TemplateVariable<T> {}

impl<T: Default> Default for TemplateVariable<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            custom: false,
        }
    }
}

impl<T: Clone> TemplateVariable<T> {
    /// Clones wrapped value.
    pub fn clone_inner(&self) -> T {
        self.value.clone()
    }
}

impl<T> TemplateVariable<T> {
    /// Creates new non-custom variable from given value.
    pub fn new(value: T) -> Self {
        Self {
            value,
            custom: false,
        }
    }

    /// Creates new custom variable from given value.
    pub fn new_custom(value: T) -> Self {
        Self {
            value,
            custom: true,
        }
    }

    /// Replaces value and also raises the `custom` flag.
    pub fn set(&mut self, value: T) -> T {
        self.custom = true;
        std::mem::replace(&mut self.value, value)
    }

    /// Returns a reference to wrapped value.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Returns true if value has changed.
    pub fn is_custom(&self) -> bool {
        self.custom
    }
}

impl<T> Deref for TemplateVariable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Visit for TemplateVariable<T>
where
    T: Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.value.visit("Value", visitor)?;
        self.custom.visit("IsCustom", visitor)?;

        visitor.leave_region()
    }
}
