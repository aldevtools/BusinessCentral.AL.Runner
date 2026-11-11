//! The value a variable starts with, from its slot (al-ir `Slot`): the default of its type, and for a record declared with
//! `[SecurityFiltering(x)]` that value. Every place that makes a routine's locals or an object's globals, or sets them back
//! (`ClearAll`, and `Clear` of a record for its table's globals), goes through here.

use super::*;

impl Session {
    /// The initial value of slot `s`. A record declared with `[SecurityFiltering(x)]` starts with x
    /// (al-language/reports/runtime/ACAttributes.al:176-179; a codeunit's global: :432, :451-458) and keeps x as the value
    /// `Clear` sets back (`clear_record`). A plain record starts Validated (`default_value`). ClearAll gives a codeunit's
    /// globals these values again (:436-438, :471-479), and a table's, a page's and a report's
    /// (al-language/reports/runtime/SFSecurityFilteringFlow.al:406-424).
    pub fn slot_default(&mut self, s: &al_ir::Slot) -> Value {
        let v = self.default_value(&s.ty);
        let Some(x) = s.security_filtering else {
            return v;
        };
        let declare = |r: &Value| {
            if let Value::Record(r) = r {
                let mut r = r.borrow_mut();
                r.security_filtering = x;
                r.declared_security_filtering = x;
            }
        };
        match &v {
            // Every element of an array of records declared with the attribute starts with x and keeps it as the value Clear
            // of the element and of the array sets back (al-language/reports/runtime/SFSecurityFilteringFlow.al:429-459).
            Value::Array(a) => a.borrow().iter().for_each(declare),
            r => declare(r),
        }
        v
    }

    /// The initial values of `slots`, in order.
    pub(crate) fn slot_defaults(&mut self, slots: &[al_ir::Slot]) -> Vec<Value> {
        slots.iter().map(|s| self.slot_default(s)).collect()
    }
}
