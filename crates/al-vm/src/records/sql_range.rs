//! The Modify of a value the SQL column of a database table cannot hold (`sqltime::store`).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{RtType, Value};
use al_data::TableDef;

use super::Rec;
use crate::Session;
use crate::sqltime::{BadValue, Evidence};

impl Session {
    /// A Modify of a row with a value its SQL column cannot hold. Assumed (`sql-date-first-year`, al-runner issue 8006, the
    /// reporter's measurement on Business Central 28.4): for a date in 1753 the Modify itself raises the platform's error and writes
    /// nothing, in the measured shape only: the result not used, a table without a Blob field that is not a CRM table and
    /// runs no table code (`has_write_hooks`), outside ModifyAll. Every other Modify of such a value is refused (the corpus
    /// pins only the Insert: record/filters/P130DateFilters.al:234-275).
    pub(super) fn fail_modify_sql_range(&mut self, rec: &Rec, t: &TableDef, bad: BadValue) -> Result<Value, Signal> {
        let has_blob = t
            .fields
            .iter()
            .any(|f| matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob)));
        let measured = matches!(bad.evidence, Evidence::InsertPinnedIn1753 | Evidence::Reporter1753)
            && !self.ret_used
            && !self.bulk_modify
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
