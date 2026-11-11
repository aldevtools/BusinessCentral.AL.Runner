//! The value a variable starts with, from its slot (al-ir `Slot`): the default of its type, and for a record declared with
//! `[SecurityFiltering(x)]` that value. Every place that makes a routine's locals or an object's globals, or sets them back
//! (`ClearAll`, and `Clear` of a record for its table's globals), goes through here.

use super::*;

impl Session {
    /// The initial value of slot `s`. A record declared with `[SecurityFiltering(x)]` starts with x
    /// (al-language/reports/runtime/ACAttributes.al:176-179; a codeunit's global: :432, :451-458) and keeps x as the value
    /// `Clear` sets back (`clear_record`). A plain record starts Validated (`default_value`). ClearAll gives a codeunit's
    /// globals these values again (:436-438, :471-479).
    pub fn slot_default(&mut self, s: &al_ir::Slot) -> Value {
        let v = self.default_value(&s.ty);
        if let (Some(x), Value::Record(r)) = (s.security_filtering, &v) {
            let mut r = r.borrow_mut();
            r.security_filtering = x;
            r.declared_security_filtering = x;
        }
        v
    }

    /// The initial values of `slots`, in order.
    pub(crate) fn slot_defaults(&mut self, slots: &[al_ir::Slot]) -> Vec<Value> {
        slots.iter().map(|s| self.slot_default(s)).collect()
    }
}
