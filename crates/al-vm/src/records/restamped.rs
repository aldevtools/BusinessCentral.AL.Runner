//! Rows whose SystemRowVersion is the row version of their Insert, not the one BC gives them (`crate::state::RestampedRows`):
//! where they are noted, and the reads that would show it, which are refused.
//!
//! BC gives the rows an Insert wrote their row version when it sends them (a read or a write of their table, a Commit), after
//! every row version handed out before: corpus PR 1509 (merged, not pinned), session-system/row-version-writes/
//! RowVersionWrites.al:392 InsertTwoTables, `row/other` greater for Row.Insert; Other.Insert; Other.Get; Row.Get. This runtime
//! stamps them at the Insert, so a row whose Insert row version is at or below one sent before it holds a lower value than BC's.
//! `Database.LastUsedRowVersion` still moves as BC's (`Session::note_read_for_row_versions`); a read of the field of such a row
//! gives a value known only to be greater than 0 (`restamped_value`), and a find or query that filters or sorts on it is
//! refused. Other reads of those rows run, in install code too.
//!
//! inserted rows sent after a higher row version (`send_pending_insert`, `Session::restamped_rows`,
//! carried in the Baseline) and what shows their SystemRowVersion (`restamped_value`, and the refusals
//! `restamped_find`, `restamped_query`).

use super::*;
use crate::state::RestampedRows;
use al_data::AccessContext;

/// The field id of SystemRowVersion.
const ROW_VERSION: u32 = 2_000_000_005;

impl Session {
    /// Takes entry `at` of `RowVersionTx::pending_inserts` out as sent now and returns the last row version its Inserts handed
    /// out. Its rows at or below a row version sent before them (the last Commit's or the open transaction's) are noted in
    /// `restamped_rows`.
    pub(crate) fn send_pending_insert(&mut self, at: usize) -> i64 {
        let rv = &mut self.tx.open.row_versions;
        let (table, _, first, last) = rv.pending_inserts.remove(at);
        let to = last.min(self.tx.committed_row_version.max(rv.sent));
        if to >= first {
            self.restamped_rows.push(RestampedRows {
                table,
                from: first - 1,
                to,
            });
        }
        last
    }

    /// A Commit sends every inserted row nothing sent yet (`send_pending_insert`).
    pub(crate) fn send_pending_inserts(&mut self) {
        while !self.tx.open.row_versions.pending_inserts.is_empty() {
            self.send_pending_insert(0);
        }
    }

    /// Whether field `pos` of table `t` holds the row version: SystemRowVersion, or an SQLTimestamp field, which takes the
    /// row version of every write (`sql_timestamp_after`).
    fn shows_row_version(&self, t: &TableDef, pos: usize) -> bool {
        t.fields.get(pos).is_some_and(|f| f.id == ROW_VERSION)
            || self.engine.table_facts[t.object as usize].sql_timestamp.contains(&pos)
    }

    /// Whether row version `v` of a row of `table` is noted in `restamped_rows`, or, with `unsent`, would be when the inserted
    /// rows nothing sent yet were sent now (`send_pending_insert`): a query reads them without sending them.
    fn restamped(&self, table: u32, v: i64, unsent: bool) -> bool {
        let current = self.tx.committed_row_version.max(self.tx.open.row_versions.sent);
        self.restamped_rows
            .iter()
            .any(|x| x.table == table && x.from < v && v <= x.to)
            || unsent
                && self
                    .tx
                    .open
                    .row_versions
                    .pending_inserts
                    .iter()
                    .any(|(o, _, first, last)| *o == table && *first <= v && v <= (*last).min(current))
    }

    /// Whether a row of table `t` that `cx` reads holds a row version `restamped` notes (`unsent` as there). Reads the table
    /// only while one is noted.
    fn holds_restamped(&self, cx: &AccessContext, t: &TableDef, unsent: bool) -> Result<bool, Signal> {
        let current = self.tx.committed_row_version.max(self.tx.open.row_versions.sent);
        let noted = self.restamped_rows.iter().any(|x| x.table == t.object)
            || unsent
                && self
                    .tx
                    .open
                    .row_versions
                    .pending_inserts
                    .iter()
                    .any(|(o, _, first, _)| *o == t.object && *first <= current);
        let Some(version) = t.fields.iter().position(|f| f.id == ROW_VERSION) else {
            return Ok(false);
        };
        if !noted {
            return Ok(false);
        }
        Ok(self
            .all_rows(cx, t)?
            .iter()
            .any(|row| matches!(row.get(version), Some(Value::BigInteger(v)) if self.restamped(t.object, *v, unsent))))
    }

    /// The value a read of field `pos` of the buffer `r` gives when the field holds the row version of a row `restamped` notes
    /// (a Record field, `interp.rs`, and `FieldRef.Value`, `refs.rs`): BC's is greater than the row versions sent before the
    /// row, so greater than this runtime's, which is greater than 0 (LastUsedRowVersion is, metadata/platform-tables/
    /// PlTDatabase.al:229 LastUsedRowVersion_Relations `positive`). So the read gives a BigInteger known only to be greater than
    /// 0 (`placeholder_output::positive`): it may be held, passed and returned, a comparison with 0 answers, and every other use
    /// (another number, Format, a builtin) refuses with the text below (ADR-080).
    pub(crate) fn restamped_value(&self, r: &RecordState, pos: usize) -> Option<Value> {
        if r.temporary || self.restamped_rows.is_empty() {
            return None;
        }
        let t = self.engine.tables.get(r.table as usize)?.as_deref()?;
        let version = t.fields.iter().position(|f| f.id == ROW_VERSION)?;
        let Some(Value::BigInteger(v)) = r.fields.get(version) else {
            return None;
        };
        (self.restamped(t.object, *v, false) && self.shows_row_version(t, pos)).then(|| {
            al_builtins::placeholder_output::positive(format!(
                "{} of a row of {} that the open transaction inserted before a row version sent ahead of it (BC gives the row its row version when it sends it, this runtime the Insert's)",
                t.fields[pos].name, t.name
            ))
        })
    }

    /// A find of the buffer `r` of table `t` (`t` with the record's ad hoc key), read through `cx`, that filters or sorts on its
    /// row version while the table holds a row `restamped` notes: refused, the rows it finds and their order are not BC's. Get,
    /// GetBySystemId and CalcFields do not read the record's filters or key.
    pub(crate) fn restamped_find(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        r: &RecordState,
        method: RecordOp,
    ) -> Result<(), Signal> {
        if r.temporary || matches!(method, RecordOp::Get | RecordOp::GetBySystemId | RecordOp::CalcFields) {
            return Ok(());
        }
        let filtered = r.filters.iter().any(|f| self.shows_row_version(t, f.field));
        let sorted = t
            .keys
            .get(r.key)
            .is_some_and(|k| k.iter().any(|&p| self.shows_row_version(t, p)));
        if (filtered || sorted) && self.holds_restamped(cx, t, false)? {
            return unsupported(format!(
                "a find filtered or sorted on SystemRowVersion of a table ({}) with rows the open transaction inserted before a row version sent ahead of them: BC gives them their row version when it sends them, this runtime the Insert's",
                t.name
            ));
        }
        Ok(())
    }

    /// A query whose column, table filter or link reads the row version of a table that holds a row `restamped` notes: refused,
    /// as `restamped_find`.
    pub(crate) fn restamped_query(&self, def: &al_ir::Query) -> Result<(), Signal> {
        if self.restamped_rows.is_empty() && self.tx.open.row_versions.pending_inserts.is_empty() {
            return Ok(());
        }
        let reads = |item: usize, pos: usize| {
            self.engine
                .tables
                .get(def.items[item].table)
                .and_then(|t| t.as_deref())
                .is_some_and(|t| self.shows_row_version(t, pos))
        };
        let mut items = std::collections::BTreeSet::new();
        for c in &def.columns {
            if c.field.is_some_and(|p| reads(c.item, p)) {
                items.insert(c.item);
            }
        }
        for (i, item) in def.items.iter().enumerate() {
            if item.table_filter.iter().any(|c| reads(i, c.field)) {
                items.insert(i);
            }
            for (k, &(own, linked)) in item.link.iter().enumerate() {
                if reads(i, own) {
                    items.insert(i);
                }
                if let Some(o) = item.link_items.get(k).copied().or(item.parent)
                    && reads(o, linked)
                {
                    items.insert(o);
                }
            }
        }
        for i in items {
            let Some(t) = self.engine.tables.get(def.items[i].table).and_then(|t| t.as_deref()) else {
                continue;
            };
            if self.holds_restamped(&self.cx, t, true)? {
                return unsupported(format!(
                    "a query on SystemRowVersion of a table ({}) with rows the open transaction inserted before a row version sent ahead of them: BC gives them their row version when it sends them, this runtime the Insert's",
                    t.name
                ));
            }
        }
        Ok(())
    }
}
