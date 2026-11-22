//! The query cache BC's page background tasks read through, and the test session with them (al-runner issue 8467; corpus
//! pull request 1457, merged, not pinned yet: tests/al-language/pages/background-tasks/BgTaskQueryCache.al, measured on Business Central 29.1).
//!
//! A worker's read that repeats an earlier read answers what the earlier read answered, also after the test session changed
//! the rows without committing. Two kinds of read are measured, and only they are kept here:
//! - a row: `Get` with every primary-key field, and `FindFirst` with exactly one `SetRange` to one value on every primary-key
//!   field (filter group 0, the primary key, ascending): the two answer alike (:367, :389, :566, :586);
//! - a group's first row and count: `FindFirst` and `Count` whose every filter is a `SetRange` to one value on a stored field
//!   (filter group 0): on the first primary-key field (:641, :658, :675, :692), on a field outside the key and on two fields
//!   (:870, :891), the filter set by `SetRange` to the value twice, by `SetFilter` of a range from the value to itself or by
//!   `FieldRef.SetRange` alike (:917).
//!
//! A row entry holds until a commit writes that row (:430, :804 against :741, :762, :784; also when no commit held the row
//! before or after, an insert and a delete each committed: open corpus pull request 1495, the same file :1207; a commit of a
//! ModifyAll or DeleteAll of another row of the table ends it or keeps it by the target's minor, :7035, :7042,
//! `al_version::bulk_commit_ends_cached_row`; after a Truncate, and on a minor no cell measured, a read it would answer
//! otherwise than the table is refused), or until a worker's `SelectLatestVersion` of the table or of every table (:6730,
//! :6737, :6786, :6845; another table's keeps it, :6744; the test session's changes no answer, :6771, :6778, :6807, :6826,
//! :6863, :6880); a group entry until a commit writes the table (:823, :845, :870, :891 against :451). A rollback changes neither (:983, :1016, :1029), and a found row, a
//! missing row and a count are all kept (:604, :624, :692). The test session's own read of a form fills the cache while that
//! session has not written the table since its last commit (:500, :541, :938; not after it wrote the table, :959), and a
//! worker's first read then answers what the test read; the test session's own read answers from an entry a worker filled
//! (:983), which is refused here. The cache starts empty in every test codeunit (:1044, :1067). Every other read keeps reading
//! the rows as they are, as on main; BC answers a repeated FindLast, IsEmpty and FindFirst after a SetFilter from the cache too
//! (open corpus pull request 1495, :1238, :1255, :1274), which is not modelled yet. A table with a Blob or a Media field is
//! cached as any other (:6898, :6918), and CalcFields of a Blob of a row the cache answered with another row is refused. The
//! cache keeps no notes at all in a program whose own apps run no page background task (`Session::query_cache_on`).

use crate::records::Rec;
use crate::{Session, Signal};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::filter::FilterExpr;
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::{AccessContext, Store, TableDef};
use al_version::BulkWrite;
use std::collections::{HashMap, HashSet};
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

/// Whether code of an app compiled for the run calls `CurrPage.EnqueueBackgroundTask` or `TestPage.RunPageBackgroundTask`
/// (`Session::query_cache_on`), worked out once per engine (`Engine::derived`).
struct BgTaskCode(bool);

impl BgTaskCode {
    fn of(program: &al_ir::Program) -> BgTaskCode {
        let ids = [
            al_ir::BuiltinId::of("Page.EnqueueBackgroundTask#1"),
            al_ir::BuiltinId::of("TestPage.RunPageBackgroundTask#1"),
        ];
        let heads = program.objects.heads();
        let found = heads.iter().enumerate().any(|(i, h)| {
            !h.uncompiled
                && h.routine_count > 0
                && program.apps.get(h.app as usize).is_some_and(|a| !a.dependency)
                && program.objects[i].routines.iter().any(|r| {
                    let mut hit = false;
                    al_ir::types::for_each_expr_post(&r.body, &mut |e| {
                        if let al_ir::Expr::Builtin { id, .. } = e {
                            hit |= ids.contains(&Some(*id));
                        }
                    });
                    hit
                })
        });
        BgTaskCode(found)
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

    /// Inserts `value` at `at` unless an entry is there: whether it was not.
    fn insert_new(&mut self, at: &At, value: T) -> bool {
        let bucket = self.0.entry(at.bucket()).or_default();
        if bucket.iter().any(|(k, _)| same_rows(k, &at.values)) {
            return false;
        }
        bucket.push((at.values.clone(), value));
        true
    }
}

/// What a commit had left of a row when the cache took it: the row's SystemRowVersion (`None`: no committed row), the
/// table's commit stamp, how many commits carried a ModifyAll, a DeleteAll and a Truncate of the table (by
/// `BulkWrite as usize`), how many commits wrote the row itself by Insert, Modify or Delete, and how many SelectLatestVersion
/// calls of a worker covered the table.
#[derive(Clone, Copy, PartialEq)]
struct Committed {
    version: Option<i64>,
    stamp: Option<u64>,
    bulk: [u64; 3],
    writes: u64,
    selects: u64,
}

/// Whether an entry taken at `then` still holds `now`.
#[derive(PartialEq)]
enum Holds {
    Yes,
    No,
    /// No committed row then or now, and a commit wrote the table since: whether a commit wrote this row in between (an
    /// insert, then a delete) is not known; or a commit of a bulk write whose effect on the target is not measured.
    Unknown,
}

/// Whether an entry taken at `then` still holds `now`; `bulk_ends`: per `BulkWrite`, whether a commit of that write of another
/// row of the table ends it on the target (`al_version::bulk_commit_ends_cached_row`). A commit that wrote the row ends it, also
/// when no commit held the row before or after (an insert and a delete, each committed: corpus PR 1495,
/// BgTaskQueryCache.al:1207), and so does a worker's SelectLatestVersion of the table (:6730, :6737, :6786, :6845; another
/// table's, :6744, does not).
fn holds(then: Committed, now: Committed, bulk_ends: [Option<bool>; 3]) -> Holds {
    if then.version != now.version || then.writes != now.writes || then.selects != now.selects {
        return Holds::No;
    }
    let bulk = (0..3).filter(|&k| then.bulk[k] != now.bulk[k]).map(|k| bulk_ends[k]);
    let mut unknown = false;
    for ends in bulk {
        match ends {
            Some(true) => return Holds::No,
            Some(false) => {}
            None => unknown = true,
        }
    }
    if unknown {
        Holds::Unknown
    } else if then.stamp == now.stamp || now.version.is_some() {
        Holds::Yes
    } else {
        Holds::Unknown
    }
}

/// What a commit had left of a table when the cache took a group read: the table's commit stamp, and how many SelectLatestVersion
/// calls of a worker covered the table. A worker's SelectLatestVersion before a group read is not measured (the reads answered
/// otherwise at two read sites, the comment at BgTaskQueryCache.al:6534); the test session's changes no group answer (:6863,
/// :6880).
#[derive(Clone, Copy, PartialEq)]
struct GroupThen {
    stamp: Option<u64>,
    selects: u64,
}

#[derive(Clone)]
enum GroupAnswer {
    First(Option<Vec<Value>>),
    Count(usize),
}

/// What the cache holds for one codeunit run (`Session::reset_query_cache` empties it).
#[derive(Default)]
pub(crate) struct QueryCache {
    /// Whether the program has code that runs a page background task (`Session::query_cache_on`); `None` until asked.
    on: Option<bool>,
    /// The rows workers read.
    rows: Entries<(Committed, Option<Vec<Value>>)>,
    /// The groups workers read.
    groups: Entries<(GroupThen, GroupAnswer)>,
    /// The rows the test session read while it had not written their table.
    test_rows: Entries<Committed>,
    /// The groups the test session read while it had not written their table.
    test_groups: Entries<GroupThen>,
    /// The next load into a partial record loads a row the cache answered with another row than the table holds; set back
    /// at the end of every record method (`Session::end_record_op_for_query_cache`).
    replay_load: bool,
    /// The commits that carried a ModifyAll, a DeleteAll or a Truncate, by table and write
    /// (`Session::note_commit_for_query_cache`).
    bulk_commits: HashMap<(u32, BulkWrite), u64>,
    /// The commits that wrote a row by Insert, Modify or Delete, by row (`Session::note_commit_for_query_cache`).
    row_commits: Entries<u64>,
    /// A worker's SelectLatestVersion calls without a table, and with one, by table (`Session::note_select_latest_version`).
    selects_all: u64,
    selects: HashMap<u32, u64>,
    /// The rows of such loads (`Buffer::fetched`, shared by copies of the record, replaced by its next fetch): a load of a field
    /// `SetLoadFields` left out is refused (BC loads it from the cached row, :714; the runner would load it from the table).
    replayed: Vec<Weak<Vec<Value>>>,
    /// The rows a worker's read answered with another row than the table holds: CalcFields of a Blob of one is refused there
    /// (`Session::refuse_blob_of_replayed`).
    replayed_rows: Entries<()>,
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
        (GroupAnswer::First(x), GroupAnswer::First(y)) => same_row(x, y),
        (GroupAnswer::Count(x), GroupAnswer::Count(y)) => x == y,
        _ => false,
    }
}

const TEST_READ_REFUSAL: &str = "a read of the test session that a page background task's earlier read answers from the query \
                                 cache otherwise than the table (BgTaskQueryCache.al:983, not modelled)";

impl Session {
    /// The test session enters a test codeunit: the cache starts empty (:1044, :1067).
    pub(crate) fn reset_query_cache(&mut self) {
        self.pages.query_cache = QueryCache::default();
    }

    /// Whether the cache keeps notes at all: only when code of an app compiled for the run (not a dependency app, ADR-013) calls
    /// `CurrPage.EnqueueBackgroundTask` or `TestPage.RunPageBackgroundTask` (review of PR 8490, MED 4: no cost for a program
    /// that runs no page background task). Dependency apps are not walked: their objects are decoded only when a run touches
    /// them (ADR-054). Without such code, a worker of a dependency app's page reads the rows as they are, as on main.
    fn query_cache_on(&mut self) -> bool {
        if let Some(on) = self.pages.query_cache.on {
            return on;
        }
        let program = self.engine.program.clone();
        let on = self.engine.derived(|| BgTaskCode::of(&program)).0;
        self.pages.query_cache.on = Some(on);
        on
    }

    /// The open transaction commits: the tables it wrote by ModifyAll, DeleteAll or Truncate count one more such commit, and so
    /// does each row it wrote by Insert, Modify or Delete.
    pub(crate) fn note_commit_for_query_cache(&mut self) {
        use crate::state::TxWriteKind as K;
        if self.tx.open.writes.is_empty() || !self.query_cache_on() {
            return;
        }
        let cache = &mut self.pages.query_cache;
        let mut tables: HashSet<(u32, BulkWrite)> = HashSet::new();
        let mut rows: Entries<()> = Entries::default();
        for w in &self.tx.open.writes {
            let kind = match w.kind {
                K::ModifyAll => BulkWrite::ModifyAll,
                K::DeleteAll => BulkWrite::DeleteAll,
                K::Truncate => BulkWrite::Truncate,
                K::Insert | K::Modify | K::Delete => {
                    let at = At {
                        table: w.table,
                        company: w.company.clone(),
                        values: w.pk.clone(),
                    };
                    if rows.insert_new(&at, ()) {
                        let n = cache.row_commits.get(&at).copied().unwrap_or(0);
                        cache.row_commits.insert(&at, n + 1);
                    }
                    continue;
                }
                _ => continue,
            };
            if tables.insert((w.table, kind)) {
                *cache.bulk_commits.entry((w.table, kind)).or_default() += 1;
            }
        }
    }

    /// `Database.SelectLatestVersion` (`table`: the overload with a table number). In a worker the reads of the rows of the
    /// table (every table without one) that follow go to the table (:6730, :6737, :6786, :6845; another table's changes nothing,
    /// :6744). In the test session it changes no answer of a worker's read (:6771, :6778, :6807, :6826, :6863, :6880).
    pub(crate) fn note_select_latest_version(&mut self, table: Option<u32>) {
        if !self.in_worker() || !self.query_cache_on() {
            return;
        }
        let cache = &mut self.pages.query_cache;
        match table {
            Some(t) => *cache.selects.entry(t).or_default() += 1,
            None => cache.selects_all += 1,
        }
    }

    fn selects_of(&self, table: u32) -> u64 {
        let cache = &self.pages.query_cache;
        cache.selects_all + cache.selects.get(&table).copied().unwrap_or(0)
    }

    fn bulk_ends(&self) -> [Option<bool>; 3] {
        let target = self.engine.target.as_ref();
        [BulkWrite::ModifyAll, BulkWrite::DeleteAll, BulkWrite::Truncate]
            .map(|kind| al_version::bulk_commit_ends_cached_row(target, kind))
    }

    fn in_worker(&self) -> bool {
        self.pages.bg_worker.is_some()
    }

    /// A record method ended: a load the cache's answer was for did not follow (review of PR 8490, LOW 10).
    pub(crate) fn end_record_op_for_query_cache(&mut self) {
        self.pages.query_cache.replay_load = false;
    }

    /// Whether `rec` reads as every measured read does: a database table of the session's company (one with a Blob or a Media
    /// field too, :6898, :6918), the primary key, ascending, no marks, the default ReadIsolation and SecurityFiltering, no
    /// SetAutoCalcFields, no LockTable of its table, and (with `all_fields`) no `SetLoadFields`; and whether the program runs
    /// page background tasks at all (`query_cache_on`).
    fn cached_form(&mut self, rec: &Rec, t: &TableDef, all_fields: bool) -> Result<bool, Signal> {
        if !self.query_cache_on() {
            return Ok(false);
        }
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
        let selects = self.selects_of(t.object);
        let cache = &self.pages.query_cache;
        let bulk = [BulkWrite::ModifyAll, BulkWrite::DeleteAll, BulkWrite::Truncate]
            .map(|kind| cache.bulk_commits.get(&(t.object, kind)).copied().unwrap_or(0));
        Ok(Committed {
            version,
            stamp,
            bulk,
            writes: cache
                .row_commits
                .get(&Self::at(rec, t, pk.to_vec()))
                .copied()
                .unwrap_or(0),
            selects,
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

    /// What `Count` of `rec` that counted `fresh` answers (`plain`: no FlowField filter).
    pub(crate) fn cached_count(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        method: RecordOp,
        plain: bool,
        fresh: usize,
    ) -> Result<usize, Signal> {
        if method != RecordOp::Count || !plain || !self.cached_form(rec, t, false)? {
            return Ok(fresh);
        }
        let Some(values) = Self::group_filter_key(rec, t, "count") else {
            return Ok(fresh);
        };
        match self.cached_group(rec, cx, t, values, GroupAnswer::Count(fresh))? {
            GroupAnswer::Count(n) => Ok(n),
            GroupAnswer::First(_) => Ok(fresh),
        }
    }

    /// What a `FindFirst` of `rec` that found `fresh` answers: through the cache as a row or a group read, when it is one.
    pub(crate) fn cached_find_first(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        method: RecordOp,
        fresh: Option<Vec<Value>>,
    ) -> Result<Option<Vec<Value>>, Signal> {
        if method != RecordOp::FindFirst || !self.cached_form(rec, t, true)? {
            return Ok(fresh);
        }
        if let Some(pk) = Self::row_filter_key(rec, t) {
            return self.cached_row(rec, cx, t, &pk, fresh);
        }
        let Some(values) = Self::group_filter_key(rec, t, "first") else {
            return Ok(fresh);
        };
        match self.cached_group(rec, cx, t, values, GroupAnswer::First(fresh.clone()))? {
            GroupAnswer::First(row) => {
                let at = row
                    .as_ref()
                    .map(|r| Self::at(rec, t, t.keys[0].iter().map(|&f| r[f].clone()).collect()));
                self.answered(t, at.as_ref(), &row, &fresh);
                Ok(row)
            }
            GroupAnswer::Count(_) => Ok(fresh),
        }
    }

    /// A worker's read answers `answer` (a row of table `t` at `at`) where the table holds `fresh`: flagged, and noted for the
    /// load that follows and for a CalcFields of a Blob of the row.
    fn answered(&mut self, t: &TableDef, at: Option<&At>, answer: &Option<Vec<Value>>, fresh: &Option<Vec<Value>>) {
        let replayed = self.in_worker() && !same_row(answer, fresh);
        if replayed {
            assume(Assumption::BackgroundTaskQueryCache);
            if let Some(at) = at
                && !self.engine.table_facts[t.object as usize].blob_fields.is_empty()
            {
                self.pages.query_cache.replayed_rows.insert(at, ());
            }
        }
        self.pages.query_cache.replay_load = replayed && answer.is_some();
    }

    /// CalcFields of a Blob of the row of `rec` with primary key `pk`, in a worker: refused when the cache answered a read of
    /// that row with another row than the table holds (review of PR 8490, MED 2): BC's CalcFields of a Blob in a worker waits for
    /// the rows the test codeunit wrote (the comment at BgTaskQueryCache.al:6534, not modelled), the runner would read the
    /// table's Blob next to the cached fields.
    pub(crate) fn refuse_blob_of_replayed(&self, rec: &Rec, t: &TableDef, pk: &[Value]) -> Result<(), Signal> {
        if self.in_worker()
            && self
                .pages
                .query_cache
                .replayed_rows
                .get(&Self::at(rec, t, pk.to_vec()))
                .is_some()
        {
            return unsupported(
                "CalcFields of a Blob of a row a page background task's read answered from the query cache (not modelled)",
            );
        }
        Ok(())
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
        self.answered(t, Some(&at), &answer, &fresh);
        Ok(answer)
    }

    const UNKNOWN_ROW: &str = "a page background task's read of a row whose query cache entry a later commit of the table may \
                               have ended (a commit of a ModifyAll, DeleteAll or Truncate, or of the table when no commit held \
                               the row; not measured)";

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
        let now = GroupThen {
            stamp: self.store_for(rec)?.committed_stamp(cx, t),
            selects: self.selects_of(t.object),
        };
        let written = self.store_for(rec)?.written_since_commit(cx, t);
        let cache = &self.pages.query_cache;
        let entry = cache
            .groups
            .get(&at)
            .filter(|(then, _)| then.stamp == now.stamp)
            .map(|(then, a)| (then.selects == now.selects, a.clone()));
        if !self.in_worker() {
            if written {
                return Ok(fresh);
            }
            if entry.as_ref().is_some_and(|(_, a)| !same_group(a, &fresh)) {
                return unsupported(TEST_READ_REFUSAL);
            }
            self.pages.query_cache.test_groups.insert(&at, now);
            return Ok(fresh);
        }
        let answer = match entry {
            Some((true, a)) => a,
            Some((false, a)) if !same_group(&a, &fresh) => {
                return unsupported(
                    "a group read of a page background task after its SelectLatestVersion, which the query cache would answer \
                     otherwise than the table (not measured)",
                );
            }
            Some((false, _)) => fresh.clone(),
            // The test session read the group before it wrote the table: what BC answers then is not measured.
            None if written && cache.test_groups.get(&at) == Some(&now) => {
                return unsupported(
                    "a group read of a page background task after the test session read the group and then wrote the table (the \
                     query cache's answer is not measured, BgTaskQueryCache.al:938 measures a row)",
                );
            }
            None => fresh.clone(),
        };
        self.pages.query_cache.groups.insert(&at, (now, answer.clone()));
        // A FindFirst is flagged by `answered`.
        if matches!(answer, GroupAnswer::Count(_)) && !same_group(&answer, &fresh) {
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
