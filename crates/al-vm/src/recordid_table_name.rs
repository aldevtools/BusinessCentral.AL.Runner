//! The table name of the text `Evaluate(RecordId, Text)` reads, beyond the exact name (`Session::evaluate_record_id_text`):
//! the name in double quotes and a prefix of a table name (recordref-fieldref/N582EvalFieldRefUnused.al:143-169
//! WA_Evaluate_FieldRef_RecordId_Statement_TableNamePrefix, waldo run 2 of al-runner issue 6924, corpus master 176baa02).

use crate::Session;
use al_builtins::native::Signal;

/// `"<name>"` is read as `<name>` (N582EvalFieldRefUnused.al:151 `name in quotes`, :165-167: `"LT WA RecId Row": 2` reads
/// back `LT WA RecId Row: 2`).
pub(crate) fn unquoted(name: &str) -> &str {
    match name.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
        Some(n) if !n.contains('"') => n,
        _ => name,
    }
}

impl Session {
    /// The tables whose name starts with `name` (any case), for a text that names no table exactly. One such table is that
    /// table (N582EvalFieldRefUnused.al:147-148 `LT WA RecId R` with and without a key, :152-157). Several: BC took 97240
    /// "LT WA RecId Row" for `LT WA RecId` over 97241 "LT WA RecId Two" (:149, :158-160), the table with the lowest id and the
    /// first name alike; which rule it follows when the two differ is not pinned, and that case is refused. A text that is
    /// longer than every table name it starts with names no table (:150 `name followed by text`, :161-164).
    pub(crate) fn record_id_tables_by_prefix(&self, name: &str) -> Result<Vec<usize>, Signal> {
        if name.is_empty() {
            return Ok(Vec::new());
        }
        let heads = self.engine.program.objects.heads();
        let found: Vec<usize> = heads
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                o.table.is_some()
                    && o.name.len() > name.len()
                    && o.name.get(..name.len()).is_some_and(|p| p.eq_ignore_ascii_case(name))
            })
            .map(|(i, _)| i)
            .collect();
        if found.len() < 2 {
            return Ok(found);
        }
        let by_id = found.iter().min_by_key(|i| heads[**i].id.unwrap_or(u32::MAX)).copied();
        let by_name = found
            .iter()
            .min_by_key(|i| heads[**i].name.to_ascii_lowercase())
            .copied();
        match (by_id, by_name) {
            (Some(a), Some(b)) if a == b => Ok(vec![a]),
            _ => Err(Signal::Unsupported(format!(
                "Evaluate into a RecordId of `{name}`, a prefix of several table names whose lowest id is not the first name (not pinned)"
            ))),
        }
    }
}
