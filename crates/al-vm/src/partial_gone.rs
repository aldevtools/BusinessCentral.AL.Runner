//! Partial records whose row another variable deleted or replaced: the errors BC raises when it loads the missing fields
//! (record/load-fields/P170JitLoadGone.al, al-runner issue 6924).

use crate::Session;
use crate::records::Rec;
use crate::transactions::al_error_code;
use al_builtins::native::Signal;
use al_builtins::value::{FieldSet, Value};
use al_data::{Store, TableDef};

const FIRST_SYSTEM_FIELD: u32 = 2_000_000_000;
const SYSTEM_ID: u32 = 2_000_000_000;

/// The table, the buffer's fields and the loaded set of a partial record.
type GonePartial = (std::sync::Arc<TableDef>, Vec<Value>, FieldSet);

impl Session {
    /// A read of field `p` that is not loaded, whose row was deleted and inserted again under the same key: the row has another
    /// SystemId, so BC raises InconsistentRead instead of loading it (P170JitLoadGone.al:1006-1020
    /// PG_RowDeletedAndInsertedAgainWithSameKey). `None` when the SystemId is the same or either side has none.
    pub(crate) fn jit_inconsistent_read(
        &self,
        t: &TableDef,
        fields: &[Value],
        stored: &[Value],
        p: usize,
    ) -> Result<Option<Signal>, Signal> {
        let Some(sid) = t.fields.iter().position(|f| f.id == SYSTEM_ID) else {
            return Ok(None);
        };
        match (fields.get(sid), stored.get(sid)) {
            (Some(Value::Guid(a)), Some(Value::Guid(b))) if a != b => {}
            _ => return Ok(None),
        }
        let key = self.identification(t, fields)?;
        Ok(Some(al_error_code(
            &format!(
                "The field '$systemId' on table '{}' has changed in the database between initial and JIT load. The fields causing the JIT load are: '{}'.\\\\The error may be transient if multiple users are editing the same record or permanent if the same user edits the record while keeping an old record.\\\\Identification values: '{key}'\\    ",
                self.caption(t),
                t.fields[p].name
            ),
            "InconsistentRead",
        )))
    }

    /// The table, the buffer's fields and the loaded set of a partial record that holds a row another variable removed; `None` when
    /// the record is not partial, does not hold the row's key, or the row exists.
    fn gone_partial(&mut self, rec: &Rec) -> Result<Option<GonePartial>, Signal> {
        let (have, held) = {
            let r = rec.borrow();
            match &r.load.buffer {
                Some(b) if r.load.partial && !b.cleared && !b.init && !matches!(b.have, FieldSet::All) => {
                    (b.have.clone(), b.row.clone())
                }
                _ => return Ok(None),
            }
        };
        let t = self.table(rec)?;
        let fields = rec.borrow().fields.clone();
        if held.as_deref() != Some(crate::partial::row_id(&t, &fields).as_str()) {
            return Ok(None);
        }
        let pk = Self::sql_key(rec, &t, &fields);
        let cx = self.cx.clone();
        let stored = self
            .store_for(rec)
            .get(&cx, &t, &pk)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
        Ok(stored.is_none().then_some((t, fields, have)))
    }

    /// A Validate of a field that is not loaded, on a partial record whose row is gone: the JIT error naming that field
    /// (P170JitLoadGone.al:618-632 PG_Validate_FieldWithoutTrigger). While the row exists nothing happens here.
    pub(crate) fn validate_jit_gone(&mut self, rec: &Rec, pos: usize) -> Result<(), Signal> {
        let Some((t, fields, have)) = self.gone_partial(rec)? else {
            return Ok(());
        };
        let f = &t.fields[pos];
        if !f.stored || f.id >= FIRST_SYSTEM_FIELD || have.contains(pos) || matches!(fields[pos], Value::Blob(_)) {
            return Ok(());
        }
        Err(self.jit_row_gone(&t, &fields, &[pos]).unwrap_or_else(|e| e))
    }

    /// A Rename of a partial record whose row is gone: BC loads every unloaded field first and raises the JIT error naming them all,
    /// in field order (P170JitLoadGone.al:653-666 PG_Rename_AfterGone). Nothing happens while the row exists (that load is not
    /// pinned here) or when the buffer does not hold the row's key.
    pub(crate) fn rename_jit_gone(&mut self, rec: &Rec) -> Result<(), Signal> {
        let Some((t, fields, have)) = self.gone_partial(rec)? else {
            return Ok(());
        };
        let missing: Vec<usize> = (0..t.fields.len())
            .filter(|&p| t.fields[p].stored && t.fields[p].id < FIRST_SYSTEM_FIELD && !have.contains(p))
            .collect();
        if missing.iter().any(|&p| matches!(fields[p], Value::Blob(_))) {
            return Err(Signal::Unsupported(
                "a Rename of a partial record whose row is gone, with a Blob field not loaded (not pinned)".into(),
            ));
        }
        if missing.is_empty() {
            return Ok(());
        }
        Err(self.jit_row_gone(&t, &fields, &missing).unwrap_or_else(|e| e))
    }
}
