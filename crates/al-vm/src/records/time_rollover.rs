//! Refuse a key-order scan that cannot retain a stored Time's rollover position.

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
        // wrong place in a key holding that field. BC orders it after the day (TimeRolledPastMidnight.al:162-182); not modelled.
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
}
