//! Refusals around a Time the column of a database table holds past the day (`sqltime::is_rolled`): a key-order scan that
//! cannot keep its position, and a Modify of such a row that runs trigger code or subscribers.

use super::*;

impl Session {
    pub(crate) fn scan(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        ascending: bool,
        after: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let row = self.scan_in_order(rec, t, ascending, after)?;
        // A record that read a Time the column holds past the day holds 00:00:00, so the next read from it would start from the
        // wrong place in a key holding that field. BC orders it after the day (record/filters/TimeRolledPastMidnight.al:179-182); not modelled.
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
}
