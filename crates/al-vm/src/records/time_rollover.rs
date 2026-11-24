//! Rules around a Time the column of a database table holds past the day (`sqltime::is_rolled`): which read lets a Modify keep
//! it (`RolledRead`), and the refusals of the shapes record/filters/TimeRolledPastMidnight.al does not measure: a scan that
//! returns such a row in the order of a key that holds the field, a Modify of such a row that runs trigger code or subscribers, a
//! Rename that writes the rollover, and a query or CalcFormula filter on the field over such a row.
//!
//! the rules around a Time a database column holds past the day (`sqltime::keeps_past_midnight` owns
//! the field shape, record/filters/TimeRolledPastMidnight.al:11-25; `sqltime::read_back` what AL
//! reads). It owns `RecordState::rolled_read`, the read that lets a Modify keep the stored value
//! (`sqltime::keep_rolled`): the variable's own Get from AL code (`TxState::get_from_al`, set in
//! `record_op_of_al`) or ModifyAll's read of the row; every other load, record method, `:=` and
//! builtin operand ends it. Refusals: a scan that returns such a row in the order of a key holding
//! the field, a Modify of such a row that runs trigger code or subscribers, a Rename that writes the
//! rollover, and a query or CalcFormula filter on the field over a table that holds it. Entries:
//! `scan`, `rolled_read_before` and `rolled_read_of_load` (by `record_op` and `load` in
//! `records.rs`), `modify_read_the_rolled_row` and `refuse_rolled_modify_with_code` (by the Modify
//! arm of `op_inner.rs`), `refuse_rename_writing_rolled` (Rename arm), `refuse_filter_over_rolled`
//! (by `queries.rs` and `flowfields.rs`).

use super::sql_range::ModifyCaller;
use super::*;
use al_builtins::value::{RolledRead, RolledReadBy};

/// A builtin given record variables as operands (RecordRef.SetTable, Page.GetRecord, Codeunit.Run, ...) may write their buffers
/// outside their own methods: it ends their [`RolledRead`].
#[inline]
pub(crate) fn rolled_read_end_for_operands(operands: &[Value]) {
    for v in operands {
        if let Value::Record(r) = v
            && r.borrow().rolled_read.0.is_some()
        {
            r.borrow_mut().rolled_read = RolledRead(None);
        }
    }
}

impl Session {
    pub(crate) fn scan(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        ascending: bool,
        after: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let row = self.scan_in_order(rec, t, ascending, after)?;
        // A scan that returns a row holding a Time the column holds past the day, in the order of a key that holds that field:
        // the record then holds 00:00:00, and the next read from it would start from the wrong place. BC orders the row after the
        // day (record/filters/TimeRolledPastMidnight.al:179-182); not modelled.
        if let Some(r) = &row
            && let Some(key) = t.keys.get(rec.borrow().key)
            && key.iter().any(|&i| crate::sqltime::is_rolled(&r[i]))
        {
            return unsupported(format!(
                "a read of {} in the order of a key that holds a Time stored as 23:59:59.999 (not modelled)",
                t.name
            ));
        }
        Ok(row)
    }

    /// Before record method `method` on `rec`: every method but Modify ends the buffer's [`RolledRead`]. A Get sets it again
    /// when it loads such a row ([`Session::rolled_read_of_load`]).
    pub(crate) fn rolled_read_before(method: RecordOp, rec: &Rec) {
        if method != RecordOp::Modify && rec.borrow().rolled_read.0.is_some() {
            rec.borrow_mut().rolled_read = RolledRead(None);
        }
    }

    /// The [`RolledRead`] a load of database row `row` (primary key `pk`) gives the buffer: the Get AL code called on a Record
    /// variable (`TxState::get_from_al`) of a row that holds a Time past the day, as at TimeRolledPastMidnight.al:210, :221. Every
    /// other load gives none.
    pub(super) fn rolled_read_of_load(
        &mut self,
        row: &[Value],
        pk: &[Value],
        by_get: bool,
        temporary: bool,
    ) -> RolledRead {
        let of_al = std::mem::take(&mut self.tx.get_from_al);
        let rolled = !temporary && row.iter().any(crate::sqltime::is_rolled);
        RolledRead((of_al && by_get && rolled).then(|| (RolledReadBy::Get, pk.to_vec())))
    }

    /// Whether the Modify of `rec` starting now writes the row the buffer read (and uses up its [`RolledRead`]): the Modify
    /// statement after the variable's own Get of that row (TimeRolledPastMidnight.al:210-212, :221-223), or ModifyAll's own read of
    /// each row (:245).
    pub(super) fn modify_read_the_rolled_row(rec: &Rec, t: &TableDef, caller: ModifyCaller) -> bool {
        let mark = std::mem::take(&mut rec.borrow_mut().rolled_read).0;
        let pk = t.primary_key(&rec.borrow().fields);
        match mark {
            Some((RolledReadBy::Get, at)) => caller == ModifyCaller::Statement && same_key(&at, &pk),
            Some((RolledReadBy::ModifyAll, at)) => same_key(&at, &pk),
            None => false,
        }
    }

    /// A Modify of a row that holds a Time past the day is measured with no code around it: table 75011 declares no trigger
    /// and no subscriber listens to it (record/filters/TimeRolledPastMidnight.al:12-25, Modify() and ModifyAll() at :203-262).
    /// Trigger code a Modify(true) runs, an extension's modify trigger, a modify event subscriber or a global Modify trigger
    /// is refused.
    pub(crate) fn refuse_rolled_modify_with_code(
        &mut self,
        t: &TableDef,
        old: &[Value],
        run: bool,
    ) -> Result<(), Signal> {
        if !old.iter().any(crate::sqltime::is_rolled) {
            return Ok(());
        }
        let mut code = run && self.table_trigger(t, "OnModify").is_some();
        for n in ["OnBeforeModify", "OnModify", "OnAfterModify"] {
            code |= run && !self.ext_triggers(t, n).is_empty();
        }
        if code
            || self.listened(t, "onbeforemodifyevent", None)?
            || self.listened(t, "onaftermodifyevent", None)?
            || self.global_enabled(t, crate::globaltriggers::GlobalOp::Modify)?
        {
            return unsupported(format!(
                "a Modify with trigger code or subscribers of a {} row that holds a Time stored as 23:59:59.999 (not measured)",
                t.name
            ));
        }
        Ok(())
    }

    /// The pin measures the rollover written by Insert, Modify and ModifyAll (TimeRolledPastMidnight.al:203-262). A Rename whose
    /// row `stored` (as the columns hold it) holds it is refused.
    pub(super) fn refuse_rename_writing_rolled(stored: &[Value]) -> Result<(), Signal> {
        if stored.iter().any(crate::sqltime::is_rolled) {
            return unsupported("a Rename that writes a Time of 23:59:59.999 (not measured)");
        }
        Ok(())
    }

    /// The pin measures a filter on a [`crate::sqltime::keeps_past_midnight`] field through the record's own filters and FieldRef
    /// filters only (TimeRolledPastMidnight.al:99-159, :369-376). `filters` of a query's dataitem or of a CalcFormula over
    /// database table `t` that name such a field are refused when a row of `t` holds the rollover in it, whether the filter
    /// keeps that row or not.
    pub(crate) fn refuse_filter_over_rolled(
        &mut self,
        cx: &al_data::AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let Some(schema) = engine.program.objects[t.object as usize].table.as_ref() else {
            return Ok(());
        };
        let fields: Vec<usize> = filters
            .iter()
            .map(|f| f.field)
            .filter(|&i| crate::sqltime::keeps_past_midnight(t, schema, i))
            .collect();
        if fields.is_empty() {
            return Ok(());
        }
        let mut rolled = false;
        self.each_row(cx, t, &[], |row| {
            rolled |= fields.iter().any(|&i| crate::sqltime::is_rolled(&row[i]));
            Ok(())
        })?;
        if rolled {
            return unsupported(
                "a query or CalcFormula filter on a Time field over a row that holds 23:59:59.999 (not measured)",
            );
        }
        Ok(())
    }
}
