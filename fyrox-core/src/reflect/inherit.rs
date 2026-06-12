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

use crate::reflect::Reflect;
use crate::variable::{InheritError, VariableFlags};
use std::any::TypeId;

pub trait ReflectInheritableVariable: Reflect {
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

    /// Clones self value.
    fn clone_value_box(&self) -> Box<dyn Reflect>;

    /// Marks value as modified, so its value won't be overwritten during property inheritance.
    fn mark_modified(&mut self);

    /// Returns a mutable reference to wrapped value without marking the variable itself as modified.
    fn inner_value_mut(&mut self) -> &mut dyn Reflect;

    /// Returns a shared reference to wrapped value without marking the variable itself as modified.
    fn inner_value_ref(&self) -> &dyn Reflect;
}
