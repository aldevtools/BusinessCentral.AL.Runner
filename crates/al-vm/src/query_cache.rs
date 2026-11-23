//! The query cache BC's page background tasks read through, and the test session with them (al-runner issue 8467; corpus
//! pull request 1457, merged, not pinned yet: tests/al-language/pages/background-tasks/BgTaskQueryCache.al, measured on Business Central 29.1).
//!
//! A worker's read that repeats an earlier read answers what the earlier read answered, also after the test session changed
//! the rows without committing. Two kinds of read are measured, and only they are kept here:
//! - a row: `Get` with every primary-key field, and `FindFirst` whose filters set every primary-key field to one value, by
//!   `SetRange` or by `SetFilter` (filter group 0, the primary key, ascending): they answer alike (:367, :389, :566, :586;
//!   open corpus pull request 1495, the same file, :1834 and :1850 for `SetFilter`);
//! - a group: `FindFirst`, `FindLast`, `Count` and `IsEmpty` of AL code (a Record or a RecordRef; not a page's own read,
//!   :2099), keyed by the read, the sort direction and every filter (`group_key`): the group alone (:641, :658, :675, :692),
//!   a field outside the key and two fields (:870, :891), and in PR 1495 FindLast, IsEmpty and FindFirst after `SetFilter`
//!   (:1238, :1255, :1274) and all four reads of every shape of :1680-:1794, :2673-:2737, :3356-:3384, :4188-:4205,
//!   :5554 and :5564 (no filter or up to 100000, the key set twice :5581, filter groups 0 to 255 and the OR group, filter
//!   group 1 in a worker, a descending sort, ranges, `SetFilter` of values, wildcards, the comparison operators, `@` alone,
//!   an empty `FILTER()`, an `&` inside an `|`, a FlowFilter field, :3472), and FindFirst and FindLast with FlowField
//!   filters next to one value on the key (:3437, :3447, :4215-:4248, :5571-:5609). One value set by `SetRange`,
//!   by `SetFilter`, by a range from the value to itself or by `FieldRef.SetRange` is one filter (:917, :1802, :1818).
//!   `Find('+')`, `Find('-')`, `FindSet`, and any of the four after `SetLoadFields`, read the rows as they are (:1999-:2083,
//!   :2722);
//! - a calculation (al-runner issue 8677): `CalcFields` of a Record, keyed by its fields in their order, the row and the
//!   record's FlowFilters (`calc_key`, :6147-:6255), and `CalcSums` of a Record, keyed like a group read and by its fields
//!   (:6273-:6285).
//!
//! A row entry holds until a commit writes that row (:430, :804 against :741, :762, :784; also when no commit held the row
//! before or after, an insert and a delete each committed: PR 1495 :1207; a commit of a ModifyAll or DeleteAll of another row
//! of the table ends it or keeps it by the target's minor, :7035, :7042, `al_version::bulk_commit_ends_cached_row`; after a
//! Truncate, and on a minor no cell measured, a read it would answer otherwise than the table is refused), or until a
//! worker's `SelectLatestVersion` of the table or of every table (:6730, :6737, :6786, :6845; another table's keeps it, :6744;
//! the test session's changes no answer, :6771, :6778, :6807, :6826, :6863, :6880); a group entry until a commit writes the
//! table (:823, :845, :870, :891 against :451; :1907, :1924, :1980), or a table its FlowField filters read (:3465, :4259). A
//! rollback changes neither (:983, :1016, :1029), and a found row, a missing row and a count are all
//! kept (:604, :624, :692). The test session's own read fills the cache while that session has not written the table since
//! its last commit (:500, :541, :938; a test page's read of a line too, :2262, :2284, :2317; a group read in every shape but
//! filter group 1, :6498, :1866, :1886, :2337-:2367, :2751, :2770, :3482-:3514; not after it wrote the table, :959, :1944,
//! :2063), and a worker's first read
//! then answers what the test read. The test session's own read answers from an entry a worker filled
//! (:983), which is refused here. The cache starts empty in every test codeunit (:1044, :1067). A table with a Blob or a
//! Media field is cached as any other (:6898, :6918), and CalcFields of a Blob of a row the cache answered with another row
//! is refused. The cache keeps no notes at all in a program whose own apps run no page background task
//! (`Session::query_cache_on`). Every other read keeps reading the rows as they are, as on main.

use crate::records::Rec;
use crate::{Session, Signal};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::{AccessContext, Store, TableDef};
use al_version::BulkWrite;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::sync::Arc;

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

/// What commits had left when the cache took a group read: the commit stamps of the table and of the tables its FlowField
/// filters read, and how many SelectLatestVersion calls of a worker covered the table. A worker's SelectLatestVersion before a
/// group read is not measured (the reads answered otherwise at two read sites, the comment at BgTaskQueryCache.al:6534); the
/// test session's changes no group answer (:6863, :6880).
#[derive(Clone, PartialEq)]
struct GroupThen {
    stamps: Vec<Option<u64>>,
    selects: u64,
}

#[derive(Clone)]
enum GroupAnswer {
    /// The row a FindFirst or FindLast found.
    Row(Option<Vec<Value>>),
    Count(usize),
    /// Whether IsEmpty found the group empty.
    Empty(bool),
    /// The values a CalcFields or a CalcSums calculated, in the order of its fields.
    Values(Vec<Value>),
}

/// A group read: the method that reads (PR 1495 :1680-:1794 read all four).
#[derive(Clone, Copy)]
enum Form {
    First,
    Last,
    Count,
    Empty,
    /// CalcSums (:6273-:6285).
    Sums,
}

impl Form {
    fn kind(self) -> &'static str {
        match self {
            Form::First => "first",
            Form::Last => "last",
            Form::Count => "count",
            Form::Empty => "empty",
            Form::Sums => "sums",
        }
    }

    /// The AL method that reads as this form.
    fn method(self) -> RecordOp {
        match self {
            Form::First => RecordOp::FindFirst,
            Form::Last => RecordOp::FindLast,
            Form::Count => RecordOp::Count,
            Form::Empty => RecordOp::IsEmpty,
            Form::Sums => RecordOp::CalcSums,
        }
    }
}

/// Whether the cells set a group read's filters in filter group `group`: 0, 2 (:1698), 3 and 5 (:2685), 4 (:1704), 6 (:1766), 7
/// (:2692), 8 and 10 (:3377), 100 and 255 (:3384), each of 2-255 (:3370), and the cross-column OR group (:2698); filter group 1,
/// the global one, only on a worker's record that set it itself while the test session held none (:3391, `own_group1`: the
/// test session's read in it and a record that reads another record's group 1 are not measured). BC drops a filter set in a
/// group past 255 (record/crud/RcAFilterInfo.al FilterGroup_Numbers_Past_The_Range_BQ), which the runner does not model.
fn cached_filter_group(group: i32, own_group1: bool) -> bool {
    group == al_builtins::filter::OR_GROUP || matches!(group, 0 | 2..=255) || group == 1 && own_group1
}

/// The most filters a cell's group read had: one on each of 502 fields in filter groups 0, 2 and on (:5564; 26215 and 60000
/// at :5554, 7650 at :3370, 7710 and values of their own at :4188 and :4196, 26214 at :4205, no limit found); no filter
/// at all is measured too (:2737).
const MAX_CACHED_FILTERS: usize = 100000;

/// What the cache holds for one codeunit run (`Session::reset_query_cache` empties it).
#[derive(Default)]
pub(crate) struct QueryCache {
    /// Whether the program has code that runs a page background task (`Session::query_cache_on`); `None` until asked.
    on: Option<bool>,
    /// The rows workers read.
    rows: Entries<(Committed, Option<Vec<Value>>)>,
    /// The groups workers and the test session read, with the commit stamps then of the table and of the tables its FlowField
    /// filters read, and the worker's SelectLatestVersion calls on the table.
    groups: Entries<(GroupThen, GroupAnswer)>,
    /// The rows the test session read while it had not written their table.
    test_rows: Entries<Committed>,
    /// The read of AL code running (`mark_al_record_read`): only it reads a group or a calculation through the cache.
    pub(crate) al_record_read: Option<RecordOp>,
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
        (GroupAnswer::Row(x), GroupAnswer::Row(y)) => same_row(x, y),
        (GroupAnswer::Count(x), GroupAnswer::Count(y)) => x == y,
        (GroupAnswer::Empty(x), GroupAnswer::Empty(y)) => x == y,
        (GroupAnswer::Values(x), GroupAnswer::Values(y)) => same_rows(x, y),
        _ => false,
    }
}

/// A filter's expression as key values (`Session::group_key`), tag first; `None` for a kind no cell measures. The kinds the
/// cells set (PR 1495): one value (:1680, also by `SetFilter`, a placeholder and a range to itself, :1728, :917), a range and an
/// open one (:1710, :1780), `<>`, `<`, `<=`, `>`, `>=` and a pattern not matched, with and without `@` (:1780, :2713), a
/// pattern with and without `@` (:1741), `@` alone and an empty `FILTER()` of a view (:2713), `|` and `&` of those (:1735,
/// :1780), and an `&` inside an `|` (:2705). A wildcard written as a value (`=a*`) is an error on BC, not a read. `<>@` alone
/// on its own (:3421): its first reads keep no row, and after an insert of a line whose value is not empty, which a read of the
/// database keeps (al-builtins `filter_at_sign`, AtSignAloneReads.al:698), the worker still answers the first answer. Other
/// nestings are not measured.
fn encode_filter(expr: &FilterExpr, level: Nesting, out: &mut Vec<Value>) -> Option<()> {
    use FilterExpr as F;
    let tag = |t: &str| Value::Text(t.into());
    match expr {
        F::Eq(v) => out.extend([tag("="), v.clone()]),
        F::Range(Some(a), Some(b)) if a.identical(b) => out.extend([tag("="), a.clone()]),
        F::Range(a, b) if a.is_some() || b.is_some() => {
            out.push(tag(".."));
            for end in [a, b] {
                match end {
                    Some(v) => out.extend([tag("to"), v.clone()]),
                    None => out.push(tag("open")),
                }
            }
        }
        F::Ne(v) => out.extend([tag("<>"), v.clone()]),
        F::Lt(v) => out.extend([tag("<"), v.clone()]),
        F::Le(v) => out.extend([tag("<="), v.clone()]),
        F::Gt(v) => out.extend([tag(">"), v.clone()]),
        F::Ge(v) => out.extend([tag(">="), v.clone()]),
        F::Like(p, ci) => out.extend([tag("like"), Value::Text(p.as_str().into()), Value::Boolean(*ci)]),
        F::NotLike(p, ci) => out.extend([tag("not like"), Value::Text(p.as_str().into()), Value::Boolean(*ci)]),
        F::AtSignAlone(false) => out.push(tag("@")),
        F::AtSignAlone(true) if matches!(level, Nesting::Top) => out.push(tag("<>@")),
        F::EmptyViewValue => out.push(tag("empty view value")),
        F::Or(items) | F::And(items) => {
            let or = matches!(expr, F::Or(_));
            let items_level = match (level, or) {
                (Nesting::Top, true) => Nesting::InOr,
                (Nesting::Top, false) | (Nesting::InOr, false) => Nesting::Simple,
                _ => return None,
            };
            out.extend([tag(if or { "|" } else { "&" }), Value::Integer(items.len() as i32)]);
            for item in items {
                encode_filter(item, items_level, out)?;
            }
        }
        _ => return None,
    }
    Some(())
}

/// Where a filter expression sits in the expression `encode_filter` keys: the whole filter, an item of a top-level `|`, or an
/// item no compound may be.
#[derive(Clone, Copy, PartialEq)]
enum Nesting {
    Top,
    InOr,
    Simple,
}

/// Whether a group read with filters on FlowFields (`flow_fields`, of its `filters`) is keyed: a FindFirst or FindLast whose
/// filters set each primary-key field to one value by one filter, or by two of that value in one group or in two (:5571), by
/// SetRange, SetFilter or a range to itself, in a filter group other than the OR group (:3437, :3447; 1, 3, 5, 7, 8, 100 and
/// 255 at :4226), the fields of a two-field key in one group or in two (:4733). Its other filters may be anything `group_key`
/// takes (:4215, :4226, :4771). Each FlowField filter
/// is on a FlowField whose CalcFormula reads its own source table alone (`calc_reads_one_table`), whatever its method (:4248,
/// a lookup filtered on its own FlowFilter and one a table extension declares :4781), of a kind `flow_filter_measured`
/// takes, in a filter group other than the OR group (0, 2 and 5 at :4226, 1, 7, 100 and 255 at :4771). The entry also ends
/// with a commit of the tables the FlowFields read (`Session::flow_field_sources`, :3465, :4259). Count and IsEmpty read the
/// rows as they are (:3437, :3514, :4215; `cached_count`'s `plain`), and so do all four with the FlowField filter alone
/// (:3428, :3514), next to a range on the key, a filter outside the key, a part of a two-field key (:3456), the key in the OR
/// group or a key filter of two values (:4237), a case-insensitive wildcard on the FlowField (:4756), and a FlowField whose condition is a
/// FlowField of its source table (:4792). `source`: a table's schema by object index.
fn flow_field_find<'a>(
    schema: &al_ir::Table,
    t: &TableDef,
    filters: &[&FieldFilter],
    flow_fields: &[&FieldFilter],
    form: Form,
    source: impl Fn(usize) -> Option<&'a al_ir::Table>,
) -> bool {
    // Each key field's filters are its one value, outside the OR group: one filter, or the same value in two groups (:5571).
    let key = t.keys[0].iter().all(|k| {
        let on: Vec<&FieldFilter> = filters.iter().copied().filter(|f| f.field == *k).collect();
        let first = on.first().and_then(|f| one_value(&f.expr));
        matches!(on.len(), 1 | 2)
            && on.iter().all(|f| {
                f.group != al_builtins::filter::OR_GROUP
                    && one_value(&f.expr).zip(first).is_some_and(|(a, b)| a.identical(b))
            })
    });
    let flow = |f: &FieldFilter| {
        f.group != al_builtins::filter::OR_GROUP
            && flow_filter_measured(&f.expr)
            && (schema.fields[f.field].calc.as_ref()).is_some_and(|c| calc_reads_one_table(schema, source(c.table), c))
    };
    matches!(form, Form::First | Form::Last) && key && flow_fields.iter().all(|f| flow(f))
}

/// The one value of `SetRange(F, V)`, `SetFilter(F, V)` or a range from the value to itself (:1834, :1850).
fn one_value(e: &FilterExpr) -> Option<&Value> {
    match e {
        FilterExpr::Range(Some(a), Some(b)) if a.identical(b) => Some(a),
        FilterExpr::Eq(a) => Some(a),
        _ => None,
    }
}

/// The kinds of a FlowField filter the cells measure: one value, a range, `>`, `<>` and a `|` of values (:4215), `>=`, `<`,
/// `<=` and an `&` of comparisons (:4743), an empty `FILTER()` of a view (:4800), open ranges and an `&` inside a `|`
/// (:5588) and a case-sensitive wildcard (:5599). A case-insensitive wildcard reads the rows as they are (:4756); other compounds
/// and patterns not matched are not measured. `@` alone on a lookup FlowField (:5609) waits for the "@ alone" PR, which reads
/// it as the database does: until then it reads the rows as they are.
fn flow_filter_measured(e: &FilterExpr) -> bool {
    use FilterExpr as F;
    let comparisons = |e: &FilterExpr| matches!(e, F::And(items) if items.iter().all(|i| matches!(i, F::Lt(_) | F::Le(_) | F::Gt(_) | F::Ge(_))));
    match e {
        F::Eq(_)
        | F::Range(..)
        | F::Like(_, false)
        | F::Ne(_)
        | F::Lt(_)
        | F::Le(_)
        | F::Gt(_)
        | F::Ge(_)
        | F::EmptyViewValue => true,
        F::Or(items) => items.iter().all(|i| one_value(i).is_some() || comparisons(i)),
        F::And(_) => comparisons(e),
        _ => false,
    }
}

/// One value or a `|` of values: the filters a measured `CalcFields` or `CalcSums` has (:6219, :6285).
fn values_only(e: &FilterExpr) -> bool {
    match e {
        FilterExpr::Or(items) => items.iter().all(|i| one_value(i).is_some()),
        e => one_value(e).is_some(),
    }
}

/// The formulas a `CalcFields` of the FlowField `calc` of a row of `row` reads, when the cells measure it: `calc` of any method
/// but negated (count, sum, min, max, average, exist, lookup, :6147-:6183), reading its source table with no FlowField of
/// the source (a condition on one reads the table, :6267), and with a `field()` condition on a count FlowField of the row
/// that reads its own source table alone, whose formula follows (:6255, :6261). Its other conditions, and the inner formula's,
/// are constants, filters, `field()` of a stored field or of a FlowFilter, and `field(filter())` of a FlowFilter (:6219).
/// `field(filter())` of a stored field, the row's value read as a filter text, is not measured (code review of c00e40f2f;
/// BC keys it by the text, the neighbor app 8677-bg-cache-calcfields): it keeps main's answer.
fn calc_formulas<'a>(
    row: &'a al_ir::Table,
    calc: &'a al_ir::CalcFormula,
    source: impl Fn(usize) -> Option<&'a al_ir::Table>,
) -> Option<Vec<&'a al_ir::CalcFormula>> {
    let src = source(calc.table)?;
    if calc.negate || calc.field.is_some_and(|f| flow_field(src, f)) {
        return None;
    }
    let flow_filter = |x: usize| row.fields.get(x).is_some_and(|d| d.flow_filter);
    let plain = |v: &al_ir::CalcValue| match *v {
        al_ir::CalcValue::Const(_) | al_ir::CalcValue::Filter(_) => true,
        al_ir::CalcValue::Field(x) => !flow_field(row, x),
        al_ir::CalcValue::FieldFilter(x) => flow_filter(x),
        al_ir::CalcValue::UpperLimit(_) => false,
    };
    let mut formulas = vec![calc];
    for c in &calc.conds {
        if flow_field(src, c.field) {
            return None;
        }
        match c.value {
            ref v if plain(v) => {}
            al_ir::CalcValue::Field(x) => {
                let inner = row.fields.get(x)?.calc.as_ref()?;
                let one = inner.method == al_ir::CalcMethod::Count && !inner.negate;
                if !one
                    || !calc_reads_one_table(row, source(inner.table), inner)
                    || !inner.conds.iter().all(|c| plain(&c.value))
                {
                    return None;
                }
                formulas.push(inner);
            }
            _ => return None,
        }
    }
    Some(formulas)
}

/// Whether field `f` of `t` is a FlowField (neither stored nor a FlowFilter), or not a field of `t`.
fn flow_field(t: &al_ir::Table, f: usize) -> bool {
    t.fields.get(f).is_none_or(|d| !d.stored && !d.flow_filter)
}

/// Whether FlowField `calc` of a row of `row` reads only its source table: it aggregates no FlowField of the source and has
/// no condition on one (`tally_source_rows` computes those per source row), and no `field()` condition names a FlowField of
/// the row (`calc_filters` computes it first). Such a FlowField reads tables the entry's commit stamps do not hold, and no
/// cell measures whether a commit of them ends the entry, so its reads keep main's answer.
fn calc_reads_one_table(row: &al_ir::Table, source: Option<&al_ir::Table>, calc: &al_ir::CalcFormula) -> bool {
    let Some(source) = source else {
        return false;
    };
    calc.field.is_none_or(|f| !flow_field(source, f))
        && calc.conds.iter().all(|c| {
            !flow_field(source, c.field)
                && match c.value {
                    al_ir::CalcValue::Field(x) | al_ir::CalcValue::UpperLimit(x) | al_ir::CalcValue::FieldFilter(x) => {
                        !flow_field(row, x)
                    }
                    _ => true,
                }
        })
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

    /// `SelectLatestVersion(Table)` with the argument `arg`: an Integer that names a table ends that table's entries, one that
    /// names no table ends none (another table's changes nothing, :6744). A Char, a Byte, an Option or an Enum, which the
    /// compiler takes for the Integer (an Enum with warning AL0603) and which reaches here as itself (al-runner issue 8879), is
    /// refused in a worker: no cell measures which table it names.
    pub(crate) fn note_select_latest_version_of(&mut self, arg: Option<&Value>) -> Result<(), Signal> {
        match arg {
            Some(Value::Integer(n)) => {
                if let Some(t) = self.table_object(i64::from(*n)) {
                    self.note_select_latest_version(Some(t as u32));
                }
                Ok(())
            }
            _ if !self.in_worker() || !self.query_cache_on() => Ok(()),
            _ => unsupported(
                "SelectLatestVersion(Table) in a page background task with a value that is no Integer (a Char, a Byte, an \
                 Option or an Enum; not measured)",
            ),
        }
    }

    fn selects_of(&self, table: u32) -> u64 {
        let cache = &self.pages.query_cache;
        cache.selects_all + cache.selects.get(&table).copied().unwrap_or(0)
    }

    /// AL code calls `method` on a Record or a RecordRef: marks a FindFirst, FindLast, Count or IsEmpty as AL code's read
    /// (`group_key`; a page's own reads are not, :2099), and a Record's CalcFields and CalcSums (a RecordRef's are not
    /// measured); returns the mark to set back when the call returns.
    pub(crate) fn mark_al_record_read(&mut self, method: RecordOp) -> Option<RecordOp> {
        let read = match method {
            RecordOp::FindFirst | RecordOp::FindLast | RecordOp::Count | RecordOp::IsEmpty => Some(method),
            RecordOp::CalcFields | RecordOp::CalcSums if !self.through_recordref => Some(method),
            _ => None,
        };
        std::mem::replace(&mut self.pages.query_cache.al_record_read, read)
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
    /// field too, :6898, :6918), the primary key, the fields of its sort ascending, no marks, the default ReadIsolation and
    /// SecurityFiltering, no SetAutoCalcFields, no LockTable of its table, and (with `all_fields`) no `SetLoadFields` (:2047,
    /// :2083); and whether the program runs page background tasks at all (`query_cache_on`). A row read also sorts ascending.
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

    /// The primary key a `FindFirst` of `rec` reads as a row: one value on every primary-key field and no other filter.
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
                    (Some(f), None) if f.group == 0 => one_value(&f.expr).cloned(),
                    _ => None,
                }
            })
            .collect()
    }

    /// The key of the group read `form` of `rec`: a read of AL code, its sort direction and every filter (filter group, field,
    /// expression) in group and field order, when the cells measure each: no filter or up to `MAX_CACHED_FILTERS` (:2737,
    /// :3370), in a filter group `cached_filter_group` takes, of the kinds `encode_filter` takes, on stored fields and FlowFilter
    /// fields (:3472), with a FlowField filter only as `flow_field_find` allows. `None`: another read, which reads the rows as
    /// they are.
    fn group_key(&self, rec: &Rec, t: &TableDef, form: Form) -> Option<Vec<Value>> {
        if self.pages.query_cache.al_record_read != Some(form.method()) {
            return None;
        }
        let r = rec.borrow();
        if r.filters.len() > MAX_CACHED_FILTERS {
            return None;
        }
        let schema = self.engine.program.objects[t.object as usize].table.as_ref()?;
        let mut filters: Vec<_> = r.filters.iter().collect();
        filters.sort_by_key(|f| (f.group, f.field));
        let mut values = vec![Value::Text(form.kind().into()), Value::Boolean(r.ascending)];
        let mut flow_fields = Vec::new();
        for f in &filters {
            if !cached_filter_group(f.group, self.global_group1.worker_owns_group1()) {
                return None;
            }
            if !t.fields.get(f.field)?.stored && !schema.fields.get(f.field)?.flow_filter {
                flow_fields.push(*f);
            }
            values.extend([Value::Integer(f.group), Value::Integer(f.field as i32)]);
            encode_filter(&f.expr, Nesting::Top, &mut values)?;
        }
        let source = |i: usize| self.engine.program.objects[i].table.as_ref();
        if !flow_fields.is_empty() && !flow_field_find(schema, t, &filters, &flow_fields, form, source) {
            return None;
        }
        Some(values)
    }

    fn at(rec: &Rec, t: &TableDef, values: Vec<Value>) -> At {
        At {
            table: t.object,
            company: rec.borrow().company.clone(),
            values,
        }
    }

    /// The tables the FlowFields `rec` filters on read (`flow_field_find` keys a read with them), once each, by object id: two
    /// reads with the same key get their commit stamps in the same order, whatever order their filters were set in, as BC
    /// shares the entry (the neighbor TwoFlowFieldFiltersSwapped_NoteAfterFirstRead, MoreNeighbors.al, al-runner issue 8879).
    fn flow_field_sources(&self, rec: &Rec, t: &TableDef) -> Vec<Arc<TableDef>> {
        let Some(schema) = self.engine.program.objects[t.object as usize].table.as_ref() else {
            return Vec::new();
        };
        let r = rec.borrow();
        let mut sources: Vec<Arc<TableDef>> = r
            .filters
            .iter()
            .filter(|f| !t.fields[f.field].stored)
            .filter_map(|f| schema.fields[f.field].calc.as_ref())
            .filter_map(|c| self.engine.tables[c.table].clone())
            .collect();
        sources.sort_by_key(|s| s.object);
        sources.dedup_by_key(|s| s.object);
        sources
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
        if plain && given == t.keys[0].len() && rec.borrow().ascending && self.cached_form(rec, t, false)? {
            return self.cached_row(rec, cx, t, pk, fresh);
        }
        Ok(fresh)
    }

    /// What `Count` or `IsEmpty` of `rec` that counted `fresh` answers (`plain`: no FlowField filter; with one, both read the
    /// rows as they are, BgTaskQueryCache.al:3437, :3514); an `IsEmpty` the cache answers empty counts 0 rows, one it answers
    /// not empty 1.
    pub(crate) fn cached_count(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        method: RecordOp,
        plain: bool,
        fresh: usize,
    ) -> Result<usize, Signal> {
        if !self.query_cache_on() {
            return Ok(fresh);
        }
        let (form, answer) = match method {
            RecordOp::Count => (Form::Count, GroupAnswer::Count(fresh)),
            RecordOp::IsEmpty => (Form::Empty, GroupAnswer::Empty(fresh == 0)),
            _ => return Ok(fresh),
        };
        let Some(values) = self.group_key(rec, t, form).filter(|_| plain) else {
            return Ok(fresh);
        };
        if !self.cached_form(rec, t, true)? {
            // A worker's Count or IsEmpty after SetLoadFields reads the table (:2722), a cell of the entry's open probe.
            if self.in_worker() && rec.borrow().load.request.is_some() && self.cached_form(rec, t, false)? {
                assume(Assumption::BackgroundTaskQueryCache);
            }
            return Ok(fresh);
        }
        let sources = self.flow_field_sources(rec, t);
        Ok(match self.cached_group(rec, cx, t, values, &sources, answer)? {
            GroupAnswer::Count(n) => n,
            GroupAnswer::Empty(empty) if empty != (fresh == 0) => usize::from(!empty),
            _ => fresh,
        })
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
        if !self.query_cache_on() {
            return Ok(fresh);
        }
        let form = match method {
            RecordOp::FindFirst => Form::First,
            RecordOp::FindLast => Form::Last,
            _ => return Ok(fresh),
        };
        let row = match form {
            Form::First if rec.borrow().ascending => Self::row_filter_key(rec, t),
            _ => None,
        };
        let group = match row {
            Some(_) => None,
            None => self.group_key(rec, t, form),
        };
        if (row.is_none() && group.is_none()) || !self.cached_form(rec, t, true)? {
            return Ok(fresh);
        }
        if let Some(pk) = row {
            return self.cached_row(rec, cx, t, &pk, fresh);
        }
        let Some(values) = group else {
            return Ok(fresh);
        };
        let sources = self.flow_field_sources(rec, t);
        match self.cached_group(rec, cx, t, values, &sources, GroupAnswer::Row(fresh.clone()))? {
            GroupAnswer::Row(row) => {
                let at = row
                    .as_ref()
                    .map(|r| Self::at(rec, t, t.keys[0].iter().map(|&f| r[f].clone()).collect()));
                self.answered(t, at.as_ref(), &row, &fresh);
                Ok(row)
            }
            _ => Ok(fresh),
        }
    }

    /// What AL code's `CalcSums` of `positions` on `rec`, which summed `fresh`, answers: through the cache when its filters are
    /// ones the cells measure, filter group 0 on stored fields, each one value or a `|` of values (:6273, :6279, :6285).
    pub(crate) fn cached_calc_sums(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        positions: &[usize],
        fresh: Vec<Value>,
    ) -> Result<Vec<Value>, Signal> {
        if !self.query_cache_on() {
            return Ok(fresh);
        }
        let Some(mut values) = self.group_key(rec, t, Form::Sums) else {
            return Ok(fresh);
        };
        let measured = rec
            .borrow()
            .filters
            .iter()
            .all(|f| f.group == 0 && t.fields[f.field].stored && values_only(&f.expr));
        if !measured || !self.cached_form(rec, t, true)? {
            return Ok(fresh);
        }
        values.extend(positions.iter().map(|&p| Value::Integer(p as i32)));
        match self.cached_group(rec, cx, t, values, &[], GroupAnswer::Values(fresh.clone()))? {
            GroupAnswer::Values(v) => Ok(v),
            _ => Ok(fresh),
        }
    }

    /// What AL code's `CalcFields` of `positions` on `rec`, which calculated `fresh`, answers: through the cache when
    /// `calc_key` keys it and no table its formulas read is virtual (its entry ends only with commits of stored tables; the
    /// neighbor app 8677-bg-cache-calcfields, Platform.al). A worker's repeated call answers its first answer (:6147-:6189,
    /// :6219, :6249, :6255), also after SetLoadFields and Get (:7255); the test session's call fills the entry before it
    /// writes (:6237, :6243).
    pub(crate) fn cached_calc_fields(
        &mut self,
        rec: &Rec,
        positions: &[usize],
        fresh: Vec<Value>,
    ) -> Result<Vec<Value>, Signal> {
        if self.pages.query_cache.al_record_read != Some(RecordOp::CalcFields) || positions.is_empty() {
            return Ok(fresh);
        }
        let table = rec.borrow().table as usize;
        let Some(t) = self.engine.tables[table].clone() else {
            return Ok(fresh);
        };
        if !self.cached_form(rec, &t, false)? {
            return Ok(fresh);
        }
        let Some((values, sources)) = self.calc_key(rec, &t, positions) else {
            return Ok(fresh);
        };
        let store = self.store_for(rec)?;
        if sources.iter().any(|s| store.is_virtual(s)) {
            return Ok(fresh);
        }
        let cx = self.cx_for(rec, &t)?;
        match self.cached_group(rec, &cx, &t, values, &sources, GroupAnswer::Values(fresh.clone()))? {
            GroupAnswer::Values(v) => Ok(v),
            _ => Ok(fresh),
        }
    }

    /// The key of a `CalcFields` of `positions` on `rec` and the tables it reads, when the cells measure it: the number of
    /// fields and the fields in their order (another order, a field more or less read the table, :6195-:6207), the row's
    /// primary key and the values of the row its formulas read by `field()`, and the record's filters, each on a FlowFilter in filter group 0, one value or a
    /// `|` of values (another value reads the table, :6219, :6225). Each FlowField is one `calc_formulas` takes.
    fn calc_key(&self, rec: &Rec, t: &TableDef, positions: &[usize]) -> Option<(Vec<Value>, Vec<Arc<TableDef>>)> {
        let schema = self.engine.program.objects[t.object as usize].table.as_ref()?;
        let source = |i: usize| self.engine.program.objects[i].table.as_ref();
        let r = rec.borrow();
        let mut values = vec![Value::Text("calcfields".into()), Value::Integer(positions.len() as i32)];
        values.extend(positions.iter().map(|&p| Value::Integer(p as i32)));
        values.extend(t.keys[0].iter().map(|&k| r.fields[k].clone()));
        let mut filters: Vec<_> = r.filters.iter().collect();
        filters.sort_by_key(|f| (f.group, f.field));
        for f in filters {
            if f.group != 0 || !schema.fields.get(f.field)?.flow_filter || !values_only(&f.expr) {
                return None;
            }
            values.push(Value::Integer(f.field as i32));
            encode_filter(&f.expr, Nesting::Top, &mut values)?;
        }
        let mut sources = Vec::new();
        for &p in positions {
            for calc in calc_formulas(schema, schema.fields.get(p)?.calc.as_ref()?, source)? {
                sources.push(self.engine.tables[calc.table].clone()?);
                for c in &calc.conds {
                    if let al_ir::CalcValue::Field(x) = c.value
                        && schema.fields.get(x)?.stored
                    {
                        values.push(r.fields[x].clone());
                    }
                }
            }
        }
        Some((values, sources))
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

    /// A group read of `rec` for `values` that the table answers with `fresh`: what the read answers. An entry holds while no
    /// commit wrote the table (:823, :845, :1907, :1980), nor a table of `sources` (the tables its FlowField filters
    /// read, :3465, :4259; the tables a calculation reads, :6231, :6261; a commit of another table keeps it). The test session's read fills it while that session has not written those tables since its last
    /// commit, in every shape `group_key` takes (:6498, :1866, :1886, :2337-:2367, :2751, :2770, :3482-:3514), and after it wrote
    /// the table reads the table (:1944, :2063).
    fn cached_group(
        &mut self,
        rec: &Rec,
        cx: &AccessContext,
        t: &TableDef,
        values: Vec<Value>,
        sources: &[Arc<TableDef>],
        fresh: GroupAnswer,
    ) -> Result<GroupAnswer, Signal> {
        let at = Self::at(rec, t, values);
        let selects = self.selects_of(t.object);
        let store = self.store_for(rec)?;
        let stamps: Vec<Option<u64>> = std::iter::once(t)
            .chain(sources.iter().map(|s| &**s))
            .map(|table| store.committed_stamp(cx, table))
            .collect();
        let written = std::iter::once(t)
            .chain(sources.iter().map(|s| &**s))
            .any(|table| store.written_since_commit(cx, table));
        let now = GroupThen { stamps, selects };
        let entry = self
            .pages
            .query_cache
            .groups
            .get(&at)
            .filter(|(then, _)| then.stamps == now.stamps)
            .map(|(then, a)| (then.selects == now.selects, a.clone()));
        if !self.in_worker() {
            if written {
                return Ok(fresh);
            }
            if entry.as_ref().is_some_and(|(_, a)| !same_group(a, &fresh)) {
                return unsupported(TEST_READ_REFUSAL);
            }
            self.pages.query_cache.groups.insert(&at, (now, fresh.clone()));
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
            Some((false, _)) | None => fresh.clone(),
        };
        self.pages.query_cache.groups.insert(&at, (now, answer.clone()));
        // A FindFirst or FindLast is flagged by `answered`.
        if !matches!(answer, GroupAnswer::Row(_)) && !same_group(&answer, &fresh) {
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
