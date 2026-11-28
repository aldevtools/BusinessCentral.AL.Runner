//! Rows whose SystemRowVersion is the row version of their Insert, not the one BC gives them (`crate::state::RestampedRows`):
//! where they are noted, and the reads that would show it, which are refused.
//!
//! BC gives the rows an Insert wrote their row version when it sends them (a read or a write of their table, a Commit), after
//! every row version handed out before: corpus PR 1509 (merged, not pinned), session-system/row-version-writes/
//! RowVersionWrites.al:392 InsertTwoTables, `row/other` greater for Row.Insert; Other.Insert; Other.Get; Row.Get. This runtime
//! stamps them at the Insert, so a row whose Insert row version is at or below one sent before it holds a lower value than BC's.
//! `Database.LastUsedRowVersion` still moves as BC's (`Session::note_read_for_row_versions`); a read of the field of such a row,
//! and a find or query that filters or sorts on it, are refused. Other reads of those rows run, in install code too.
//! One shape runs: a row whose Insert buffer's SystemRowVersion the code read before the send keeps that value on BC, so a read
//! of its field gives the Insert's value (`RestampedRows::insert_value_read`, assumption `restamped-row-version-insert-value`,
//! corpus PR 1613, RowVersionReReadShapes.al `Run`). Without that read BC gives the row its version at the send, above the
//! other table's rows (1613 :889-:915, :958, :971; 1509 :420): those reads keep the refusal.
//!
//! inserted rows sent after a higher row version (`send_pending_insert`, `Session::restamped_rows`,
//! carried in the Baseline) and the refusals of what shows their SystemRowVersion (`restamped_read`,
//! `restamped_find`, `restamped_query`).

use super::*;
use crate::state::{RestampedRows, SentBy};
use al_builtins::assumptions::{Assumption, assume};
use al_data::AccessContext;

/// The field id of SystemRowVersion.
const ROW_VERSION: u32 = 2_000_000_005;

impl Session {
    /// The re-reads corpus PR 1613 sends the row with (RowVersionReReadShapes.al `ReRead`): Get, FindFirst, FindSet and
    /// Find('-') through a Record variable, RecordRef.FindFirst (`note_read_for_row_versions`, `send_pending_insert`), of a
    /// buffer `r` with no filter and no mark filter on the primary key, as `ReRead` declares them.
    pub(crate) fn measured_send_read(&self, method: RecordOp, find_form: Option<&str>, r: &RecordState) -> bool {
        (r.filters.is_empty(), r.key, r.marked_only) == (true, 0, false)
            && matches!(
                (method, self.through_recordref, find_form),
                (RecordOp::Get | RecordOp::FindFirst | RecordOp::FindSet, false, _)
                    | (RecordOp::Find, false, Some("-"))
                    | (RecordOp::FindFirst, true, _)
            )
    }

    /// Takes entry `at` of `RowVersionTx::pending_inserts` out as sent now and returns the last row version its Inserts handed
    /// out. Its rows at or below a row version sent before them (the last Commit's or the open transaction's) are noted in
    /// `restamped_rows`, with `insert_value_read` in the shape corpus PR 1613 measures (RowVersionReReadShapes.al `Run`, sends
    /// `other`, `othercommit`): one row, its Insert buffer's SystemRowVersion read (`note_insert_value_read`), only the one
    /// measured Modify raised the row version sent (`SentBy::Modify`), and a `Commit()` statement or a measured read sends it
    /// (`measured`).
    pub(crate) fn send_pending_insert(&mut self, at: usize, measured: bool) -> i64 {
        let rv = &mut self.tx.open.row_versions;
        let (table, company, first, last) = rv.pending_inserts.remove(at);
        rv.insert_buffers.retain(|(o, c, _, _)| (*o, c) != (table, &company));
        let to = last.min(self.tx.committed_row_version.max(rv.sent));
        if to >= first {
            let read = rv.insert_values_read.contains(&(table, company, first));
            self.restamped_rows.push(RestampedRows {
                table,
                from: first - 1,
                to,
                insert_value_read: (measured, rv.sent_by, first == last, read) == (true, SentBy::Modify, true, true),
            });
        }
        last
    }

    /// A Commit sends every inserted row nothing sent yet (`send_pending_insert`). Only an AL `Commit()` statement is a
    /// measured send (corpus PR 1613, RowVersionReReadShapes.al send `othercommit`); a test's end, a TestPage or a
    /// `Codeunit.Run` commit is not.
    pub(crate) fn send_pending_inserts(&mut self) {
        let measured = self.access_notes.commit_by == Some(crate::CommitBy::Statement);
        while !self.tx.open.row_versions.pending_inserts.is_empty() {
            self.send_pending_insert(0, measured);
        }
    }

    /// Keeps the Record buffer `rec` whose Insert into `table` handed out the row version of an entry of `pending_inserts` that
    /// holds that one row (`note_write`; `RowVersionTx::insert_buffers`).
    pub(crate) fn note_row_version_buffer(
        &mut self,
        rec: &Rc<RefCell<RecordState>>,
        table: u32,
        company: &Option<String>,
    ) {
        let rv = &mut self.tx.open.row_versions;
        let one = rv
            .pending_inserts
            .iter()
            .any(|(o, c, first, last)| (*o, c, *first, *last) == (table, company, rv.last, rv.last));
        if one {
            rv.insert_buffers
                .push((table, company.clone(), rv.last, Rc::downgrade(rec)));
        }
    }

    /// Notes a read of field `pos` of the Record buffer `r` of `rc` (`interp.rs`) when the field is SystemRowVersion of a row
    /// of `pending_inserts` whose row version no row version sent before it reaches, and `rc` is the buffer whose Insert
    /// handed it out (`insert_buffers`; a `:=` or Copy copy or a by-value parameter is another buffer): the Before read of
    /// corpus PR 1613 (RowVersionReReadShapes.al `Run`, `Before := Row.SystemRowVersion` right after `Row.Insert`).
    pub(crate) fn note_insert_value_read(&mut self, rc: &Rc<RefCell<RecordState>>, r: &RecordState, pos: usize) {
        if r.temporary || self.tx.open.row_versions.pending_inserts.is_empty() {
            return;
        }
        let Some(t) = self.engine.tables.get(r.table as usize).and_then(|t| t.as_deref()) else {
            return;
        };
        if t.fields.get(pos).map(|f| f.id) != Some(ROW_VERSION) {
            return;
        }
        let Some(&Value::BigInteger(v)) = r.fields.get(pos) else {
            return;
        };
        let current = self.tx.committed_row_version.max(self.tx.open.row_versions.sent);
        let rv = &mut self.tx.open.row_versions;
        let pending = rv
            .pending_inserts
            .iter()
            .any(|(o, c, first, last)| *o == t.object && *c == r.company && *first <= v && v <= *last);
        let own = rv
            .insert_buffers
            .iter()
            .any(|(o, c, at, b)| (*o, c, *at, b.as_ptr()) == (t.object, &r.company, v, Rc::as_ptr(rc)));
        if (pending, own, v > current) == (true, true, true) {
            let key = (t.object, r.company.clone(), v);
            if !rv.insert_values_read.contains(&key) {
                rv.insert_values_read.push(key);
            }
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

    /// Why a read of field `pos` of the buffer `r` is refused: the field holds the row version of a row `restamped` notes. Read
    /// by a Record field (`interp.rs`) and by `FieldRef.Value` (`refs.rs`). A row noted with `insert_value_read` runs: its
    /// SystemRowVersion read in any buffer (a Get, FindFirst, FindSet, Find('-') or Next re-read, RecordRef.FindFirst or
    /// GetTable and `Field(0).Value`, a `:=` or Copy of a Get buffer) gives the Insert's value, as corpus PR 1613 measures.
    pub(crate) fn restamped_read(&self, r: &RecordState, pos: usize) -> Option<String> {
        if r.temporary || self.restamped_rows.is_empty() {
            return None;
        }
        let t = self.engine.tables.get(r.table as usize)?.as_deref()?;
        let version = t.fields.iter().position(|f| f.id == ROW_VERSION)?;
        let Some(&Value::BigInteger(v)) = r.fields.get(version) else {
            return None;
        };
        let measured = (pos == version).then(|| self.insert_value_read(t.object, v));
        if measured == Some(true) && self.restamped(t.object, v, false) {
            assume(Assumption::RestampedRowVersionInsertValue);
            return None;
        }
        (self.restamped(t.object, v, false) && self.shows_row_version(t, pos)).then(|| {
            format!(
                "{} of a row of {} that the open transaction inserted before a row version sent ahead of it: BC gives the row its row version when it sends it, this runtime the Insert's",
                t.fields[pos].name, t.name
            )
        })
    }

    /// Whether every noted range of `table` that holds row version `v` is one row noted with `insert_value_read`.
    fn insert_value_read(&self, table: u32, v: i64) -> bool {
        let mut holding = self
            .restamped_rows
            .iter()
            .filter(|x| x.table == table && x.from < v && v <= x.to);
        holding.clone().next().is_some() && holding.all(|x| x.insert_value_read)
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
