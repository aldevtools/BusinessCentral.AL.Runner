//! The save of a new row on a DelayedInsert page whose NotBlank primary key field is still blank.
//!
//! The corpus pins the save of a DelayedInsert row that fails on its typed key (testpage/TestPageFailedInsert_Tests.al:1-30, a
//! duplicate key) and the save of a blank key without NotBlank (codeunits/run-onrun/BlankKeyRowNextTest.al:319
//! H1_Write_DelayedCardOpenNewSetValue, a Guid key). It does not pin a blank key whose table field has NotBlank = true: an
//! outside reporter's BC run shows a validation error 'Code must be filled in.' after `New()`, a SetValue on another field and
//! `Next()` on a List (al-runner issue 6924), and neither the text, the step that raises it nor whether the row is inserted is
//! measured. Every save of such a row (a move, New, Close, an action, page code) is refused before it inserts.

use super::*;

impl Session {
    /// Refuses the insert of the new row `st` holds when the page has DelayedInsert = true (the page or an extension) and a
    /// primary key field of the source table has NotBlank = true and holds its blank value.
    pub(super) fn refuse_delayed_blank_not_blank_key(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let delayed = effective_prop(&engine, object, "DelayedInsert")?.is_some_and(|v| v.eq_ignore_ascii_case("true"));
        if !delayed {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let r = rec.borrow();
        let Some(table) = engine.program.objects[r.table as usize].table.as_ref() else {
            return Ok(());
        };
        let Some(pk) = table.keys.first() else {
            return Ok(());
        };
        for &k in pk {
            let not_blank = table.fields[k]
                .properties
                .iter()
                .any(|(n, v)| n.eq_ignore_ascii_case("NotBlank") && v.trim().eq_ignore_ascii_case("true"));
            let blank = al_builtins::ops::compare(&r.fields[k], &Value::default_for(&table.fields[k].ty)).ok()
                == Some(std::cmp::Ordering::Equal);
            if not_blank && blank {
                return unsupported(format!(
                    "saving a new row of a DelayedInsert page while its NotBlank primary key field '{}' is blank (the validation error and whether the row is inserted are not pinned)",
                    table.fields[k].name
                ));
            }
        }
        Ok(())
    }
}
