//! The insert of a new host that the first `Visible()` or `Editable()` of a field-linked part tries (`new_host_on_part_access`)
//! fails. Pinned by pages/parts/PartLineBeforeHostKey.al R7031_* (codeunit 67582, pages 95604-95606; corpus PR 1134, in the
//! pin since c9aac994), the same on all three supported majors and for a part whose Visible is the constant true, a
//! variable set to false in OnNewRecord or a variable set to true in OnOpenPage:
//! - the page's OnInsertRecord runs, then the table's OnInsert; when either raises, no error reaches the test and Visible() and
//!   Editable() answer their normal values (:4032 R7031_ConstTrue_TableError_VisibleFirst `visible = Yes`, `visible_log =
//!   [P.OnInsertRecord: | H.OnInsert:]`; :4072 R7031_ConstTrue_PageError_VisibleFirst `visible_log = [P.OnInsertRecord:]`: the
//!   table's OnInsert does not run after OnInsertRecord raised);
//! - the host is not stored (`after_visible = 0 hosts, 1 lines`);
//! - nothing tries again: a second Visible() or Editable() and Close() run no insert trigger and the host stays unsaved
//!   (`editable_log = []`, `close_log = []`, `after_close = 0 hosts, 1 lines`; :4212 R7031_VarFalse_PageError_EditableFirst).
//!
//! Only a TestField error of triggers that wrote nothing to the database and changed no field of the record is measured
//! (whether BC rolls back a write made before the error is not pinned). Any other error, and any later call on the host or its
//! parts other than Close and Visible() or Editable() of a part, is refused. What GetLastErrorText reads after the swallowed
//! error is not pinned.

use super::*;
use al_builtins::native::AlError;
use al_builtins::page_op::PageMethod;

/// The field number of SystemId.
const SYSTEM_ID: u32 = 2_000_000_000;

impl Session {
    /// The insert that `m` (Visible or Editable of a part) tried on `host` failed with `e`. `writes` is the write mark and
    /// `fields` the host record's fields before the insert. `Ok(())`: the error is swallowed.
    pub(super) fn swallow_part_prop_insert(
        &mut self,
        host: &St,
        m: PageMethod,
        e: &AlError,
        writes: (i64, u64),
        fields: &[Value],
    ) -> Result<(), Signal> {
        if e.code != "TestField" {
            return unsupported(format!(
                "an error other than TestField in the insert of a new host that {m} of its part tried (only TestField is pinned)"
            ));
        }
        if self.write_mark() != writes {
            return unsupported(format!(
                "a failed insert of a new host that {m} of its part tried, after its triggers wrote to the database (whether BC rolls the writes back is not pinned)"
            ));
        }
        let rec = self.page_rec(host);
        // The insert gives the record its SystemId before the triggers run; the other fields must be as before. The page's record
        // is put back as it was, without the SystemId (which SystemId it holds after the failed insert is not pinned; nothing
        // pinned after the swallow reads it).
        let table = self.table(&rec)?;
        let changed = rec.borrow().fields.iter().zip(fields).enumerate().any(|(i, (a, b))| {
            table.fields.get(i).is_none_or(|f| f.id != SYSTEM_ID)
                && !Self::same_values(std::slice::from_ref(a), std::slice::from_ref(b))
        });
        if changed || rec.borrow().fields.len() != fields.len() {
            return unsupported(format!(
                "a failed insert of a new host that {m} of its part tried, after its triggers changed the record (what the page shows then is not pinned)"
            ));
        }
        rec.borrow_mut().fields = fields.to_vec();
        let mut h = host.borrow_mut();
        h.dirty = false;
        h.part_insert_swallowed = true;
        drop(h);
        // What GetLastErrorText reads after the swallowed error is not pinned.
        self.last_error = al_builtins::native::unpinned_error();
        Ok(())
    }

    /// A call on `st` after the swallowed insert of its host (or of `st` itself): only Close of the host and Visible() or
    /// Editable() of a part are pinned. `name` is `None` for a control call.
    pub(super) fn refuse_after_swallowed_part_insert(&self, st: &St, name: Option<PageMethod>) -> Result<(), Signal> {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        let (swallowed, is_part) = match &host {
            Some(h) => (h.borrow().part_insert_swallowed, true),
            None => (st.borrow().part_insert_swallowed, false),
        };
        if !swallowed {
            return Ok(());
        }
        let allowed = match name {
            Some(PageMethod::Visible | PageMethod::Editable) => is_part,
            Some(PageMethod::Close) => !is_part,
            _ => false,
        };
        if allowed {
            return Ok(());
        }
        let call = name.map_or_else(|| "a control call".to_string(), |n| n.to_string());
        unsupported(format!(
            "{call} on a new host (or its part) whose insert, tried by Visible() or Editable() of a part, failed (only Close and the part's Visible() and Editable() are pinned)"
        ))
    }
}
