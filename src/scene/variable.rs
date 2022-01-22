//! A wrapper for a variable that hold additional flag that tells that initial value was changed in runtime.
//!
//! For more info see [`TemplateVariable`]

use crate::core::visitor::prelude::*;
use bitflags::bitflags;
use std::{cell::Cell, ops::Deref};

bitflags! {
    /// A set of possible variable flags.
    pub struct VariableFlags: u8 {
        /// Nothing.
        const NONE = 0;
        /// A variable was externally modified.
        const MODIFIED = 0b0000_0001;
        /// A variable must be synced with respective variable from data model.
        const NEED_SYNC = 0b0000_0010;
    }
}

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
    value: T,
    flags: Cell<VariableFlags>,
}

impl<T: Clone> Clone for TemplateVariable<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            flags: self.flags.clone(),
        }
    }
}

impl<T> From<T> for TemplateVariable<T> {
    fn from(v: T) -> Self {
        TemplateVariable::new(v)
    }
}

impl<T: PartialEq> PartialEq for TemplateVariable<T> {
    fn eq(&self, other: &Self) -> bool {
        // `custom` flag intentionally ignored!
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for TemplateVariable<T> {}

impl<T: Default> Default for TemplateVariable<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            flags: Cell::new(VariableFlags::NONE),
        }
    }
}

impl<T: Clone> TemplateVariable<T> {
    /// Clones wrapped value.
    pub fn clone_inner(&self) -> T {
        self.value.clone()
    }

    /// Tries to sync a value in a data model with a value in the template variable. The value
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

    /// Tries to inherit a value from parent. It will succeed only if the current variable is
    /// not marked as modified.
    pub fn try_inherit(&mut self, parent: &Self) -> bool {
        if !self.is_modified() {
            self.value = parent.value.clone();
            true
        } else {
            false
        }
    }
}

impl<T> TemplateVariable<T> {
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

    /// Replaces value and also raises the [`VariableFlags::MODIFIED`] flag.
    pub fn set(&mut self, value: T) -> T {
        self.mark_modified();
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
        self.mark_modified();
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

    /// Returns true if value was modified.
    pub fn is_modified(&self) -> bool {
        self.flags.get().contains(VariableFlags::MODIFIED)
    }

    fn mark_modified(&mut self) {
        self.flags
            .get_mut()
            .insert(VariableFlags::MODIFIED | VariableFlags::NEED_SYNC);
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

        // Backward compatibility. Convert bool -> VariableFlags
        let mut old = false;
        if visitor.is_reading() {
            let mut is_custom = false;
            if is_custom.visit("IsCustom", visitor).is_ok() {
                self.flags.get_mut().insert(VariableFlags::MODIFIED);
                old = true;
            }
        }

        if !old {
            self.flags.get_mut().bits.visit("Flags", visitor)?;
        }

        visitor.leave_region()
    }
}
