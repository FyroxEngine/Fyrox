pub use fyrox_reflect::variable::*;

use crate::visitor::{prelude::*, VisitorFlags};

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
