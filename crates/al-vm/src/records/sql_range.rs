//! The Modify of a value the SQL column of a database table cannot hold (`sqltime::store`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

use super::Rec;
use crate::Session;
use crate::sqltime::{BadValue, Evidence};

/// What a failed Insert and a failed Modify put back on the buffer: its row version and the stamped fields.
pub(super) type Stamps = (Option<(usize, Value)>, crate::hostvalues::Replaced);

/// The facts of an Insert the gate of `defer_insert_sql_range` takes.
pub(super) struct InsertShape {
    /// The table has an AutoIncrement field.
    pub(super) auto: bool,
    /// The Insert runs the table's triggers.
    pub(super) run: bool,
}

/// The facts of a Modify the gate takes by equality (`fail_modify_sql_range`).
pub(super) struct ModifyShape {
    /// The statement of AL code on a Record variable (`TxState::modify_from_al`): not ModifyAll, not a RecordRef, not a page save.
    pub(super) from_al: bool,
    /// The row is stored.
    pub(super) row_exists: bool,
}

impl Session {
    /// An Insert of a value its SQL column cannot hold: the Insert does not fail, the next access of the table does
    /// (record/filters/P130DateFilters.al:255-275, stage `get`), as for a duplicate key. With a Blob field, an AutoIncrement
    /// field, the result used, an after-insert trigger or event, or a page write it is not pinned. A CRM table has no SQL column:
    /// what its table connection does with such a value is not pinned.
    pub(super) fn defer_insert_sql_range(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        bad: BadValue,
        shape: InsertShape,
        (old_version, stamped): Stamps,
    ) -> Result<Value, Signal> {
        let has_blob = t
            .fields
            .iter()
            .any(|f| matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob)));
        if self.ret_used
            || rec.borrow().crm
            || has_blob
            || shape.auto
            || self.tx.immediate_errors
            || (shape.run && !self.ext_triggers(t, "OnAfterInsert").is_empty())
            || self.listened(t, "onafterinsertevent", None)?
        {
            return unsupported(format!(
                "an Insert into {} of a value its SQL column cannot hold, in a form the corpus does not pin",
                t.name
            ));
        }
        if bad.evidence == Evidence::Reporter1753 {
            assume(Assumption::SqlDateFirstYear);
        }
        Self::restore_row_version(rec, old_version);
        Self::unstamp(rec, stamped);
        self.defer_insert(rec, t.object, bad.error);
        Ok(Value::Boolean(true))
    }

    /// A Modify of a row with a value its SQL column cannot hold. Assumed (`sql-date-first-year`, al-runner issue 8006, the
    /// reporter's report on Business Central 28.4 that BC refused the write; the statement that raised is not reported): the
    /// Modify raises the platform's error and writes nothing, in this shape only: the Modify statement of AL code on a Record
    /// variable (`from_al`), of a stored row, the result not used, outside the page client, a table without a Blob field that is
    /// not a CRM table and runs no table code (`has_write_hooks`). Every other Modify of such a value is refused: ModifyAll
    /// (both forms), RecordRef.Modify, a page save, a row that does not exist (the corpus pins only the Insert:
    /// record/filters/P130DateFilters.al:234-275).
    pub(super) fn fail_modify_sql_range(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        bad: BadValue,
        shape: ModifyShape,
        (old_version, stamped): Stamps,
    ) -> Result<Value, Signal> {
        Self::restore_row_version(rec, old_version);
        Self::unstamp(rec, stamped);
        let has_blob = t
            .fields
            .iter()
            .any(|f| matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob)));
        let measured = matches!(bad.evidence, Evidence::InsertPinnedIn1753 | Evidence::Reporter1753)
            && shape.from_al
            && shape.row_exists
            && !self.ret_used
            && !self.tx.immediate_errors
            && !rec.borrow().crm
            && !has_blob
            && !self.has_write_hooks(t)?;
        if !measured {
            return unsupported("a Modify of a value its SQL column cannot hold, in a form not measured (not pinned)");
        }
        assume(Assumption::SqlDateFirstYear);
        Err(Signal::Error(Box::new(bad.error)))
    }
}
