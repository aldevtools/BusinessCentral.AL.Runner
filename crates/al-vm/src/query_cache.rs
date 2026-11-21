//! The query cache BC's page background tasks read through, and the test session with them (al-runner issue 8467; corpus
//! pull request 1457, merged, not pinned yet: tests/al-language/pages/background-tasks/BgTaskQueryCache.al, measured on Business Central 29.1).
//!
//! A worker's read that repeats an earlier read answers what the earlier read answered, also after the test session changed
//! the rows without committing. Three kinds of read are measured, and only they are kept here:
//! - a row: `Get` with every primary-key field, and `FindFirst` with exactly one `SetRange` to one value on every primary-key
//!   field (filter group 0, the primary key, ascending): the two answer alike (:367, :389, :566, :586);
//! - a group's first row and count: `FindFirst` and `Count` whose every filter is a `SetRange` to one value on a stored field
//!   (filter group 0): on the first primary-key field (:641, :658, :675, :692), on a field outside the key and on two fields
//!   (:870, :891), the filter set by `SetRange` to the value twice, by `SetFilter` of a range from the value to itself or by
//!   `FieldRef.SetRange` alike (:917);
//! - the `Filtered` forms of open corpus pull request 1495 (the same file), a read of AL code on a Record whose one filter is
//!   on the first field of a primary key of more than one field (filter group 0): `FindLast` (:1276) and `IsEmpty` (:1293)
//!   after `SetRange` to one value, and `FindFirst` after `SetFilter` of one value (:1312). Each keys entries of its own.
//!
//! A row entry holds until a commit writes that row (:430, :804 against :741, :762, :784; also when no commit held the row
//! before or after, an insert and a delete each committed: open corpus pull request 1495, the same file :1207; a commit of a
//! ModifyAll or DeleteAll of the table ends it or keeps it by the target's minor, `al_version::bulk_commit_ends_cached_row`,
//! and elsewhere a read it would answer otherwise than the table is refused); a group entry until a commit
//! writes the table (:823, :845, :870, :891 against :451). A rollback changes neither (:983, :1016, :1029), and a found row, a
//! missing row and a count are all kept (:604, :624, :692). The test session's own read of a form fills the cache while that
//! session has not written the table since its last commit (:500, :541, :938; not after it wrote the table, :959), and a
//! worker's first read then answers what the test read; the test session's own read answers from an entry a worker filled
//! (:983), which is refused here. The cache starts empty in every test codeunit (:1044, :1067). For the `Filtered` forms only
//! the repeated read is measured: the test session's reads fill nothing, and a worker's first read whose answer changed since
//! the test session made it is refused; a commit of the table between two reads refuses the read whose answer it changed; and
//! a SetFilter and a SetRange `FindFirst` of one group are refused where the other's entry answers otherwise than the table.
//! Every other read keeps reading the rows as they are, as on main.

use crate::records::Rec;
use crate::{Session, Signal};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::filter::FilterExpr;
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::{AccessContext, Store, TableDef};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// Entries by table (object, company) and a text of their key values that may be shared by other keys; the key values
/// themselves tell them apart (`Value::identical`).
struct Entries<T>(HashMap<Bucket, Vec<(Vec<Value>, T)>>);

/// A table (object, company) and a text of key values (`At::bucket`).
type Bucket = (u32, Option<String>, String);

impl<T> Default for Entries<T> {
    fn default() -> Self {
        Entries(HashMap::new())
    }
}

/// Where an entry is: its table, and the values of its key (a row's primary key, or a group read's kind and filters).
struct At {
    table: u32,
    company: Option<String>,
    values: Vec<Value>,
}

impl At {
    fn bucket(&self) -> Bucket {
        let text = self
            .values
            .iter()
            .map(|v| v.describe())
            .collect::<Vec<_>>()
            .join("\u{1}");
        (self.table, self.company.clone(), text)
    }
}

impl<T> Entries<T> {
    fn get(&self, at: &At) -> Option<&T> {
        self.0
            .get(&at.bucket())?
            .iter()
            .find(|(k, _)| same_rows(k, &at.values))
            .map(|(_, v)| v)
    }

    fn insert(&mut self, at: &At, value: T) {
        let bucket = self.0.entry(at.bucket()).or_default();
        match bucket.iter_mut().find(|(k, _)| same_rows(k, &at.values)) {
            Some(slot) => slot.1 = value,
            None => bucket.push((at.values.clone(), value)),
        }
    }
}

/// What a commit had left of a row when the cache took it: the row's SystemRowVersion (`None`: no committed row), the
/// table's commit stamp, how many commits carried a ModifyAll, DeleteAll or Truncate of the table, and how many commits wrote
/// the row itself by Insert, Modify or Delete.
#[derive(Clone, Copy, PartialEq)]
struct Committed {
    version: Option<i64>,
    stamp: Option<u64>,
    bulk: u64,
    writes: u64,
}

/// Whether an entry taken at `then` still holds `now`.
#[derive(PartialEq)]
enum Holds {
    Yes,
    No,
    /// No committed row then or now, and a commit wrote the table since: whether a commit wrote this row in between (an
    /// insert, then a delete) is not known.
    Unknown,
}

/// Whether an entry taken at `then` still holds `now`; `bulk_ends`: whether a commit of a ModifyAll of another row of the table
/// ends it on the target (`al_version::bulk_commit_ends_cached_row`). A commit that wrote the row ends it, also when no commit
/// held the row before or after (an insert and a delete, each committed: corpus PR 1495, BgTaskQueryCache.al:1207).
fn holds(then: Committed, now: Committed, bulk_ends: Option<bool>) -> Holds {
    if then.version != now.version || then.writes != now.writes {
        Holds::No
    } else if then.bulk != now.bulk && bulk_ends != Some(false) {
        if bulk_ends == Some(true) {
            Holds::No
        } else {
            Holds::Unknown
        }
    } else if then.stamp == now.stamp || now.version.is_some() {
        Holds::Yes
    } else {
        Holds::Unknown
    }
}

#[derive(Clone)]
enum GroupAnswer {
    /// The row a FindFirst or FindLast found.
    Row(Option<Vec<Value>>),
    Count(usize),
    /// Whether IsEmpty found the group empty.
    Empty(bool),
}

/// The read forms of open corpus pull request 1495 that BC answers from the query cache, each keying entries of its own: one
/// filter, in filter group 0, on the first field of a primary key of more than one field (a group, not a row).
#[derive(Clone, Copy, PartialEq)]
enum Filtered {
    /// `FindLast` after `SetRange` of the field to one value (:1276).
    Last,
    /// `IsEmpty` after it (:1293).
    Empty,
    /// `FindFirst` after `SetFilter` of the field to one value, no wildcard (:1312).
    SetFilterFirst,
}

/// The kind a `FindFirst` group read keys its entries by (`group_filter_key`), and the kind of its `SetFilter` twin.
const FIRST: &str = "first";
const SETFILTER_FIRST: &str = "setfilter first";

impl Filtered {
    fn kind(self) -> &'static str {
        match self {
            Filtered::Last => "last",
            Filtered::Empty => "empty",
            Filtered::SetFilterFirst => SETFILTER_FIRST,
        }
    }
}

/// What the cache holds for one codeunit run (`Session::reset_query_cache` empties it).
#[derive(Default)]
pub(crate) struct QueryCache {
    /// The rows workers read.
    rows: Entries<(Committed, Option<Vec<Value>>)>,
    /// The groups workers read, with the table's commit stamp then.
    groups: Entries<(Option<u64>, GroupAnswer)>,
    /// The rows the test session read while it had not written their table.
    test_rows: Entries<Committed>,
    /// The groups the test session read while it had not written their table: the table's commit stamp then.
    test_groups: Entries<Option<u64>>,
    /// What the test session's reads of a `Filtered` form answered. They fill no entry: whether BC's do is not measured.
    test_filtered: Entries<GroupAnswer>,
    /// The read running is AL code's on a Record variable (`mark_al_record_read`): only it reads a `Filtered` form.
    pub(crate) al_record_read: bool,
    /// The next load into a partial record loads a row the cache answered with another row than the table holds.
    replay_load: bool,
    /// The commits that carried a ModifyAll, DeleteAll or Truncate, by table (`Session::note_commit_for_query_cache`).
    bulk_commits: HashMap<u32, u64>,
    /// The commits that wrote a row by Insert, Modify or Delete, by row (`Session::note_commit_for_query_cache`).
    row_commits: Entries<u64>,
    /// The rows of such loads (`Buffer::fetched`, shared by copies of the record, replaced by its next fetch): a load of a field
    /// `SetLoadFields` left out is refused (BC loads it from the cached row, :714; the runner would load it from the table).
    replayed: Vec<Weak<Vec<Value>>>,
}

fn same_rows(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
}

fn same_row(a: &Option<Vec<Value>>, b: &Option<Vec<Value>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => same_rows(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn same_group(a: &GroupAnswer, b: &GroupAnswer) -> bool {
    match (a, b) {
        (GroupAnswer::Row(x), GroupAnswer::Row(y)) => same_row(x, y),
        (GroupAnswer::Count(x), GroupAnswer::Count(y)) => x == y,
        (GroupAnswer::Empty(x), GroupAnswer::Empty(y)) => x == y,
        _ => false,
    }
}

const TEST_READ_REFUSAL: &str = "a read of the test session that a page background task's earlier read answers from the query \
                                 cache otherwise than the table (BgTaskQueryCache.al:983, not modelled)";

impl Session {
    /// The test session enters a test codeunit: the cache starts empty (:1044, :1067), and so does the record of a worker's
    /// first answers (`bg_read_done`).
    pub(crate) fn reset_query_cache(&mut self) {
        self.pages.query_cache = QueryCache::default();
        self.pages.bg_reads.clear();
    }

    /// The open transaction commits: the tables it wrote by ModifyAll, DeleteAll or Truncate count one more such commit, and so
    /// does each row it wrote by Insert, Modify or Delete.
    pub(crate) fn note_commit_for_query_cache(&mut self) {
        use crate::state::TxWriteKind as K;
        let cache = &mut self.pages.query_cache;
        let mut tables: Vec<u32> = self
            .tx
            .open
            .writes
            .iter()
            .filter(|w| matches!(w.kind, K::ModifyAll | K::DeleteAll | K::Truncate))
            .map(|w| w.table)
            .collect();
        tables.sort_unstable();
        tables.dedup();
        for t in tables {
            *cache.bulk_commits.entry(t).or_default() += 1;
        }
        let mut rows: Vec<At> = Vec::new();
        for w in &self.tx.open.writes {
            let at = At {
                table: w.table,
                company: w.company.clone(),
                values: w.pk.clone(),
            };
            if matches!(w.kind, K::Insert | K::Modify | K::Delete)
                && !rows
                    .iter()
                    .any(|r| r.table == at.table && r.company == at.company && same_rows(&r.values, &at.values))
            {
                rows.push(at);
            }
        }
        for at in rows {
            let n = cache.row_commits.get(&at).copied().unwrap_or(0);
            cache.row_commits.insert(&at, n + 1);
        }
    }

    /// AL code calls `method` on a Record or a RecordRef: marks a FindFirst, FindLast or IsEmpty on a Record as AL code's read
    /// (`filtered_key`); returns the mark to set back when the call returns.
    pub(crate) fn mark_al_record_read(&mut self, method: RecordOp) -> bool {
        let read =
            !self.through_recordref && matches!(method, RecordOp::FindFirst | RecordOp::FindLast | RecordOp::IsEmpty);
        std::mem::replace(&mut self.pages.query_cache.al_record_read, read)
    }

    fn bulk_ends(&self) -> Option<bool> {
        al_version::bulk_commit_ends_cached_row(self.engine.target.as_ref())
    }

    fn in_worker(&self) -> bool {
        self.pages.bg_worker.is_some()
    }

    /// Whether `rec` reads as every measured read does: a database table of the session's company, the primary key,
    /// ascending, no marks, the default ReadIsolation and SecurityFiltering, no SetAutoCalcFields, no LockTable of its table, and (with
    /// `all_fields`) no `SetLoadFields`.
    fn cached_form(&mut self, rec: &Rec, t: &TableDef, all_fields: bool) -> Result<bool, Signal> {
        let plain = {
            let r = rec.borrow();
            !r.temporary
                && r.temp_id.is_none()
                && !r.crm
                && r.company.is_none()
                && !r.change_company_called
                && r.key == 0
                && r.adhoc.is_none()
                && r.ascending
                && r.field_descending == 0
                && !r.marked_only
                && r.read_isolation == 0
                && r.security_filtering == r.declared_security_filtering
                && r.auto_calc.is_empty()
                && (!all_fields || r.load.request.is_none())
        };
        // A lock the test session holds on another table (`TxState::lock_tx`) is not the read's: a worker reads outside the
        // test session's transaction (testpage/TestPageBgTaskTx_Tests.al).
        Ok(plain && !self.tx.open.locked_tables.contains(&t.object) && !self.store_for(rec)?.is_virtual(t))
    }

    /// The one value `f` is set to by `SetRange(F, V)` in filter group 0, if it is that.
    fn one_value(f: &al_builtins::filter::FieldFilter) -> Option<&Value> {
        match &f.expr {
            FilterExpr::Range(Some(a), Some(b)) if f.group == 0 && a.identical(b) => Some(a),
            _ => None,
        }
    }

    /// The primary key a `FindFirst` of `rec` reads as a row: one `SetRange` to one value on every primary-key field.
    fn row_filter_key(rec: &Rec, t: &TableDef) -> Option<Vec<Value>> {
        let r = rec.borrow();
        let pk = &t.keys[0];
        if r.filters.len() != pk.len() {
            return None;
        }
        pk.iter()
            .map(|field| {
                let mut on = r.filters.iter().filter(|f| f.field == *field);
                match (on.next(), on.next()) {
                    (Some(f), None) => Self::one_value(f).cloned(),
                    _ => None,
                }
            })
            .collect()
    }

    /// What a group read of `rec` is for: every filter a `SetRange` to one value in filter group 0 on a stored field (on the
    /// first key field :641-692, on a field outside the key and on two fields :870, :891), as (field, value) pairs in field
    /// order, after the read's kind.
    fn group_filter_key(rec: &Rec, t: &TableDef, kind: &str) -> Option<Vec<Value>> {
        let r = rec.borrow();
        if r.filters.is_empty() {
            return None;
        }
        let mut pairs: Vec<(usize, &Value)> = Vec::new();
        for f in &r.filters {
            // A FlowField or FlowFilter is not a column of the query (not measured).
            if pairs.iter().any(|(field, _)| *field == f.field) || !t.fields.get(f.field).is_some_and(|d| d.stored) {
                return None;
            }
            pairs.push((f.field, Self::one_value(f)?));
        }
        pairs.sort_by_key(|(field, _)| *field);
        let mut values = vec![Value::Text(kind.into())];
        values.extend(
            pairs
                .into_iter()
                .flat_map(|(field, v)| [Value::Integer(field as i32), v.clone()]),
        );
        Some(values)
    }

    /// The key of a read of `rec` in the `Filtered` form `form`: a read of AL code on a Record variable (the cells' reads; a
    /// page's own reads and a RecordRef's are not measured) whose one filter, in filter group 0, is on the first field of a
    /// primary key of more than one field, to one value by `SetRange` (`Last`, `Empty`) or by `SetFilter` (`SetFilterFirst`),
    /// after the form's kind. `None`: another read, which reads the rows as they are.
    fn filtered_key(&self, rec: &Rec, t: &TableDef, form: Filtered) -> Option<Vec<Value>> {
        if !self.pages.query_cache.al_record_read {
            return None;
        }
        let r = rec.borrow();
        let [f] = r.filters.as_slice() else {
            return None;
        };
        let pk = &t.keys[0];
        if pk.len() < 2 || f.field != pk[0] {
            return None;
        }
        let value = match (form, &f.expr) {
            (Filtered::SetFilterFirst, FilterExpr::Eq(v)) if f.group == 0 => v,
            (Filtered::Last | Filtered::Empty, _) => Self::one_value(f)?,
            _ => return None,
        };
        Some(vec![
            Value::Text(form.kind().into()),
            Value::Integer(f.field as i32),
            value.clone(),
        ])
    }

    /// The entry a `FindFirst` of the same group by the other filter method keys: SetRange's (`group_filter_key`) for a
    /// SetFilter read and the other way round.
    fn twin(values: &[Value]) -> Option<Vec<Value>> {
        let [Value::Text(kind), field, value] = values else {
            return None;
        };
        let other = match kind.as_str() {
            FIRST => SETFILTER_FIRST,
            SETFILTER_FIRST => FIRST,
            _ => return None,
        };
        Some(vec![Value::Text(other.into()), field.clone(), value.clone()])
    }

    /// Refused when the twin of a `FindFirst` group read at `at` has an entry from this commit stamp of the table that answers
    /// otherwise than `fresh`: whether a SetFilter and a SetRange read of one group share an entry is not measured. A twin's
    /// entry from before a commit of the table answers nothing either way: a commit ends a group entry (:823).
    fn refuse_differing_twin(&self, at: &At, stamp: Option<u64>, fresh: &GroupAnswer) -> Result<(), Signal> {
        let Some(values) = Self::twin(&at.values) else {
            return Ok(());
        };
        let twin = At {
            table: at.table,
            company: at.company.clone(),
            values,
        };
        if self
            .pages
            .query_cache
            .groups
            .get(&twin)
            .is_some_and(|(then, a)| *then == stamp && !same_group(a, fresh))
        {
            return unsupported(
                "a FindFirst by SetFilter and a FindFirst by SetRange of the same group, the earlier one answered otherwise \
                 than the table (whether they share a query cache entry is not measured, corpus PR 1495 \
                 BgTaskQueryCache.al:1312 measures SetFilter alone)",
            );
        }
        Ok(())
    }

    fn at(rec: &Rec, t: &TableDef, values: Vec<Value>) -> At {
        At {
            table: t.object,
            company: rec.borrow().company.clone(),
            values,
        }
    }

    fn committed(&mut self, rec: &Rec, cx: &AccessContext, t: &TableDef, pk: &[Value]) -> Result<Committed, Signal> {
        let store = self.store_for(rec)?;
        let version = store
            .committed_get(cx, t, pk)
            .and_then(|row| crate::stale_rename::row_version(t, &row));
        let stamp = store.committed_stamp(cx, t);
        let cache = &self.pages.query_cache;
        Ok(Committed {
            version,
            stamp,
            bulk: cache.bulk_commits.get(&t.object).copied().unwrap_or(0),
            writes: cache
                .row_commits
                .get(&Self::at(rec, t, pk.to_vec()))
                .copied()
                .unwrap_or(0),
        })
    }

    /// What `Get` of `rec` with `given` values and key `pk` answers: the row the table holds, or through the cache when every
    /// key field was given (`plain`: not by a RecordId, not by a key value the SQL column rounds).
    pub(crate) fn cached_get(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        pk: &[Value],
        given: usize,
        plain: bool,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let fresh = self
            .store_for(rec)?
            .get(cx, t, pk)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        if plain && given == t.keys[0].len() && self.cached_form(rec, t, false)? {
            return self.cached_row(rec, cx, t, pk, fresh);
        }
        Ok(fresh)
    }

    /// What `Count` or `IsEmpty` of `rec` that counted `fresh` answers (`plain`: no FlowField filter); an `IsEmpty` the cache
    /// answers empty counts 0 rows, one it answers not empty 1.
    pub(crate) fn cached_count(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        method: RecordOp,
        plain: bool,
        fresh: usize,
    ) -> Result<usize, Signal> {
        let key = match method {
            RecordOp::Count => Self::group_filter_key(rec, t, "count"),
            RecordOp::IsEmpty => self.filtered_key(rec, t, Filtered::Empty),
            _ => None,
        };
        let Some(values) = key.filter(|_| plain) else {
            return Ok(fresh);
        };
        if !self.cached_form(rec, t, false)? {
            return Ok(fresh);
        }
        if method == RecordOp::IsEmpty {
            return Ok(
                match self.cached_filtered(rec, cx, t, values, GroupAnswer::Empty(fresh == 0))? {
                    GroupAnswer::Empty(empty) if empty != (fresh == 0) => usize::from(!empty),
                    _ => fresh,
                },
            );
        }
        match self.cached_group(rec, cx, t, values, GroupAnswer::Count(fresh))? {
            GroupAnswer::Count(n) => Ok(n),
            _ => Ok(fresh),
        }
    }

    /// What a `FindFirst` or `FindLast` of `rec` that found `fresh` answers: through the cache as a row or a group read, when
    /// it is one.
    pub(crate) fn cached_find(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        method: RecordOp,
        fresh: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let filtered = |form| self.filtered_key(rec, t, form).map(|v| (v, true));
        let (row, group) = match method {
            RecordOp::FindFirst => match Self::row_filter_key(rec, t) {
                Some(pk) => (Some(pk), None),
                None => (
                    None,
                    Self::group_filter_key(rec, t, FIRST)
                        .map(|v| (v, false))
                        .or_else(|| filtered(Filtered::SetFilterFirst)),
                ),
            },
            RecordOp::FindLast => (None, filtered(Filtered::Last)),
            _ => (None, None),
        };
        if (row.is_none() && group.is_none()) || !self.cached_form(rec, t, true)? {
            return Ok(fresh);
        }
        if let Some(pk) = row {
            return self.cached_row(rec, cx, t, &pk, fresh);
        }
        let Some((values, filtered)) = group else {
            return Ok(fresh);
        };
        let answer = if filtered {
            self.cached_filtered(rec, cx, t, values, GroupAnswer::Row(fresh.clone()))?
        } else {
            self.cached_group(rec, cx, t, values, GroupAnswer::Row(fresh.clone()))?
        };
        match answer {
            GroupAnswer::Row(row) => {
                self.answered(&row, &fresh);
                Ok(row)
            }
            _ => Ok(fresh),
        }
    }

    /// A worker's read answers `answer` where the table holds `fresh`: flagged, and noted for the load that follows.
    fn answered(&mut self, answer: &Option<Vec<Value>>, fresh: &Option<Vec<Value>>) {
        let replayed = self.in_worker() && !same_row(answer, fresh);
        if replayed {
            assume(Assumption::BackgroundTaskQueryCache);
        }
        self.pages.query_cache.replay_load = replayed && answer.is_some();
    }

    /// The row with primary key `pk` that a read of `rec` found as `fresh`: what the read answers.
    fn cached_row(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        pk: &[Value],
        fresh: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        let at = Self::at(rec, t, pk.to_vec());
        let now = self.committed(rec, cx, t, pk)?;
        if !self.in_worker() {
            return self.test_row_read(rec, cx, t, &at, now, fresh);
        }
        let cache = &self.pages.query_cache;
        let answer = match cache
            .rows
            .get(&at)
            .map(|(then, row)| (holds(*then, now, self.bulk_ends()), row))
        {
            Some((Holds::Yes, row)) => row.clone(),
            Some((Holds::Unknown, row)) if !same_row(row, &fresh) => return unsupported(Self::UNKNOWN_ROW),
            _ => match cache.test_rows.get(&at).map(|then| holds(*then, now, self.bulk_ends())) {
                Some(Holds::Yes) => self.store_for(rec)?.committed_get(cx, t, pk),
                Some(Holds::Unknown) => return unsupported(Self::UNKNOWN_ROW),
                _ => fresh.clone(),
            },
        };
        self.pages.query_cache.rows.insert(&at, (now, answer.clone()));
        self.answered(&answer, &fresh);
        Ok(answer)
    }

    const UNKNOWN_ROW: &str = "a page background task's read of a row whose query cache entry a later commit of the table may \
                               have ended (a commit of a ModifyAll or DeleteAll, or of the table when no commit held the row; \
                               not measured)";

    /// The test session's own read of a row: it notes the row while the session has not written the table since its last
    /// commit (:500, :541, :938), and is refused where a worker's entry answers otherwise than the table (:983, not modelled).
    fn test_row_read(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        at: &At,
        now: Committed,
        fresh: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        if self.store_for(rec)?.written_since_commit(cx, t) {
            return Ok(fresh);
        }
        if let Some((then, row)) = self.pages.query_cache.rows.get(at)
            && holds(*then, now, self.bulk_ends()) != Holds::No
            && !same_row(row, &fresh)
        {
            return unsupported(TEST_READ_REFUSAL);
        }
        self.pages.query_cache.test_rows.insert(at, now);
        Ok(fresh)
    }

    /// A group read of `rec` for `values` that the table answers with `fresh`: what the read answers.
    fn cached_group(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        values: Vec<Value>,
        fresh: GroupAnswer,
    ) -> Result<GroupAnswer, Signal> {
        let at = Self::at(rec, t, values);
        let stamp = self.store_for(rec)?.committed_stamp(cx, t);
        let written = self.store_for(rec)?.written_since_commit(cx, t);
        let cache = &self.pages.query_cache;
        let entry = cache
            .groups
            .get(&at)
            .filter(|(then, _)| *then == stamp)
            .map(|(_, a)| a.clone());
        if !self.in_worker() {
            if written {
                return Ok(fresh);
            }
            if entry.as_ref().is_some_and(|a| !same_group(a, &fresh)) {
                return unsupported(TEST_READ_REFUSAL);
            }
            self.refuse_differing_twin(&at, stamp, &fresh)?;
            self.pages.query_cache.test_groups.insert(&at, stamp);
            return Ok(fresh);
        }
        let answer = match entry {
            Some(a) => a,
            // The test session read the group before it wrote the table: what BC answers then is not measured.
            None if written && cache.test_groups.get(&at) == Some(&stamp) => {
                return unsupported(
                    "a group read of a page background task after the test session read the group and then wrote the table (the \
                     query cache's answer is not measured, BgTaskQueryCache.al:938 measures a row)",
                );
            }
            None => {
                self.refuse_differing_twin(&at, stamp, &fresh)?;
                fresh.clone()
            }
        };
        self.pages.query_cache.groups.insert(&at, (stamp, answer.clone()));
        // A FindFirst is flagged by `answered`.
        if matches!(answer, GroupAnswer::Count(_)) && !same_group(&answer, &fresh) {
            assume(Assumption::BackgroundTaskQueryCache);
        }
        Ok(answer)
    }

    /// A read of a `Filtered` form of `rec` at `values` that the table answers with `fresh`: what the read answers (corpus PR
    /// 1495, BgTaskQueryCache.al:1276, :1293, :1312). A worker's entry holds until a commit writes the table, and a rollback
    /// keeps it, as for a group read (`cached_group`); a commit of the table between two reads is measured for FindFirst and
    /// Count of a group only (:823, :845), so a read it changed the answer of is refused, and one it did not answers it. The
    /// test session's reads fill no entry; a worker's first read whose answer changed since the test session made it is refused.
    fn cached_filtered(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        values: Vec<Value>,
        fresh: GroupAnswer,
    ) -> Result<GroupAnswer, Signal> {
        let at = Self::at(rec, t, values);
        let stamp = self.store_for(rec)?.committed_stamp(cx, t);
        let own = self.pages.query_cache.groups.get(&at).cloned();
        if !self.in_worker() {
            // After the session wrote the table, its reads see the table, as its group reads do (`cached_group`, :983).
            if !self.store_for(rec)?.written_since_commit(cx, t) && own.is_some_and(|(_, a)| !same_group(&a, &fresh)) {
                return unsupported(TEST_READ_REFUSAL);
            }
            self.pages.query_cache.test_filtered.insert(&at, fresh.clone());
            return Ok(fresh);
        }
        let answer = match own {
            Some((then, a)) if then == stamp => a,
            Some((_, a)) if !same_group(&a, &fresh) => {
                return unsupported(
                    "a page background task's read of a FindLast, IsEmpty or SetFilter FindFirst after a commit of the table \
                     changed its answer (whether the commit ended its query cache entry is measured for FindFirst and Count \
                     of a group only, BgTaskQueryCache.al:823, :845)",
                );
            }
            Some(_) => fresh.clone(),
            None => {
                if self
                    .pages
                    .query_cache
                    .test_filtered
                    .get(&at)
                    .is_some_and(|a| !same_group(a, &fresh))
                {
                    return unsupported(
                        "a page background task's first read of a FindLast, IsEmpty or SetFilter FindFirst whose answer \
                         changed since the test session made the same read (whether that read fills the query cache is \
                         measured for Get, FindFirst and Count only, BgTaskQueryCache.al:500)",
                    );
                }
                self.refuse_differing_twin(&at, stamp, &fresh)?;
                fresh.clone()
            }
        };
        self.pages.query_cache.groups.insert(&at, (stamp, answer.clone()));
        // A FindFirst or FindLast is flagged by `answered`.
        if matches!(answer, GroupAnswer::Empty(_)) && !same_group(&answer, &fresh) {
            assume(Assumption::BackgroundTaskQueryCache);
        }
        Ok(answer)
    }

    /// A row was loaded into `rec`: when the cache answered it with another row than the table holds, its fetched row is
    /// noted for `refuse_load_of_replayed`.
    pub(crate) fn note_replayed_load(&mut self, rec: &Rec) {
        if !std::mem::take(&mut self.pages.query_cache.replay_load) {
            return;
        }
        let r = rec.borrow();
        if let Some(fetched) = r.load.buffer.as_ref().and_then(|b| b.fetched.0.as_ref()) {
            self.pages.query_cache.replayed.push(Rc::downgrade(fetched));
        }
    }

    /// The missing fields of `rec`, fetched as `fetched`, are about to be loaded: refused when the cache answered that fetch
    /// with another row than the table holds (BC loads them from the cached row, :714; the runner would load the table's).
    pub(crate) fn refuse_load_of_replayed(&mut self, fetched: &Rc<Vec<Value>>) -> Result<(), Signal> {
        let replayed = &mut self.pages.query_cache.replayed;
        replayed.retain(|w| w.strong_count() > 0);
        if replayed.iter().any(|w| std::ptr::eq(w.as_ptr(), Rc::as_ptr(fetched))) {
            return unsupported(
                "a field SetLoadFields left out, loaded into a record a page background task's read answered from the query \
                 cache (BgTaskQueryCache.al:714, not modelled)",
            );
        }
        Ok(())
    }
}
