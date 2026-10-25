//! The model behind `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` (issue 5580).
//!
//! The corpus pins the sign of a counter's change between two reads, and its size only for the shapes [`sized`] lists
//! (session/lt-bl-sql-counters/SqlCounters.al). So a read gives a tainted value (`Value::SqlCount`,
//! `al_builtins::sqlcount`), and between two reads of a counter the VM classifies what ran (a segment):
//!
//! - nothing that touched the store: no change (SqlCounters.al:105-119);
//! - on a temporary record, exactly one of Get, FindSet with Next to the end, Count, Insert, Modify, Delete: no change
//!   (SqlCounters.al:161-201);
//! - on a database table, one step of the sequence SqlCounters.al:122-159 measured after a Commit, with the steps before it
//!   exactly as measured since that Commit: Get, FindSet with Next to the end, Count, Insert, Modify and Delete of the inserted row.
//!
//! A classified segment adds one to the counter for each pinned increase; any other segment (an operation or an order no test
//! pins, AL code run by a trigger or subscriber, store access outside a record method, a Commit or a rollback) starts a new
//! epoch: reads of different epochs do not compare, so a test that would observe the unknown change is refused.
//!
//! The size of the change is pinned for nine shapes, each measured right after a Commit (no store access between the Commit
//! and the first read) on a table of one Integer key field and one Text field that held exactly the rows the shape needs,
//! for N = 1, 2, 5, 10, 20 (SqlCounters.al:204-395, [`sized`]). Such a segment adds its size and keeps the read's chain, so
//! the difference of two reads of one chain is a plain number; a segment classified only by its sign starts a new chain.
//! Logging starts with the session's first read: what ran before it is not known, so a context that reaches back past it is
//! not classified.

use al_builtins::record_op::RecordOp;
use al_builtins::value::{SqlCount, SqlCounter, SqlForm, Value};

/// A record method that ran while the counters were active.
#[derive(Debug, Clone)]
struct Event {
    kind: Kind,
    temp: bool,
    table: u32,
    /// Get and FindSet found a row, Next moved, Insert / Modify / Delete wrote without error.
    ok: bool,
    /// Insert, Modify, Delete with RunTrigger.
    trigger: bool,
    /// AL code ran inside (a trigger, a subscriber), or the method ran inside another record method.
    nested: bool,
    /// The primary key, for Get, Insert, Modify and Delete.
    key: Option<Vec<Value>>,
    /// The rows of the table before the method, when the record and its table are of the shape [`sized`] knows: a table
    /// of one Integer key field and one Text field, a record with no filter, key, order, load or isolation set.
    rows_before: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Get,
    FindSet,
    Next,
    Count,
    Insert,
    Modify,
    Delete,
    Init,
    DeleteAll,
    Other,
}

fn kind(m: RecordOp) -> Kind {
    match m {
        RecordOp::Get => Kind::Get,
        RecordOp::FindSet => Kind::FindSet,
        RecordOp::Next => Kind::Next,
        RecordOp::Count => Kind::Count,
        RecordOp::Insert => Kind::Insert,
        RecordOp::Modify => Kind::Modify,
        RecordOp::Delete => Kind::Delete,
        RecordOp::Init => Kind::Init,
        RecordOp::DeleteAll => Kind::DeleteAll,
        _ => Kind::Other,
    }
}

/// Where a counter was last read.
#[derive(Debug, Clone, Default)]
struct Cursor {
    read: bool,
    n: i64,
    epoch: u32,
    /// The absolute index of the first event after the read.
    at: usize,
    commits: u64,
    resets: u64,
    outside: u64,
    /// The read came right after a Commit: no store access since.
    clean: bool,
    /// Reads of one chain differ by pinned sizes only.
    chain: u32,
}

/// The session's counter state.
#[derive(Debug, Default)]
pub(crate) struct SqlCounters {
    /// A counter was read: record methods are logged from here on.
    pub(crate) active: bool,
    log: Vec<Event>,
    /// The absolute index of `log[0]`.
    base: usize,
    /// The absolute index of the first event after the last Commit seen while active.
    commit_mark: Option<usize>,
    resets: u64,
    /// Store operations done inside logged record methods.
    inside: u64,
    /// Record methods running.
    open: u32,
    stmt: Cursor,
    rows: Cursor,
    /// The store operations of this thread right after the last Commit (`al_data::memory::store_ops`), kept while inactive
    /// too: a measurement may start with the session's first read.
    ops_at_commit: Option<u64>,
    /// The last chain handed out.
    chains: u32,
}

/// What ran between two reads of a counter.
enum Change {
    /// A pinned size.
    Exact(i64),
    /// Only the sign is pinned: increased or not.
    Sign(bool),
}

/// What a record method noted on entry (`SqlCounters::enter`).
pub(crate) struct Entry {
    ops: u64,
    calls: u64,
    outer: bool,
}

/// The most events the log holds. Past it the log is dropped (`SqlCounters::reset`): a test that reads a counter once and
/// then runs many record methods does not keep them all.
const LOG_CAP: usize = 4096;

/// One step of the sequence SqlCounters.al:122-159 measured, with the signs of (statements, rows read) it gave.
const TABLE_STEPS: [(Kind, bool, bool); 6] = [
    (Kind::Get, true, true),      // :131-132, :152
    (Kind::FindSet, true, true),  // :134-137, :153
    (Kind::Count, false, false),  // :139-140, :154
    (Kind::Insert, false, false), // :142-145, :155
    (Kind::Modify, true, true),   // :147-149, :156
    (Kind::Delete, true, false),  // :151-152, :157
];

impl SqlCounters {
    /// A record method starts.
    pub(crate) fn enter(&mut self, calls: u64) -> Entry {
        let outer = self.open == 0;
        self.open += 1;
        Entry {
            ops: al_data::memory::store_ops(),
            calls,
            outer,
        }
    }

    /// A record method ended.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn leave(
        &mut self,
        e: Entry,
        calls: u64,
        m: RecordOp,
        temp: bool,
        table: u32,
        args: &[Value],
        key: Option<Vec<Value>>,
        rows_before: Option<usize>,
        result: &Result<Value, al_builtins::native::Signal>,
    ) {
        self.open -= 1;
        if e.outer {
            self.inside += al_data::memory::store_ops() - e.ops;
        }
        let kind = kind(m);
        let ok = match (kind, result) {
            (Kind::Next, Ok(Value::Integer(n))) => *n != 0,
            (Kind::Next, _) => false,
            (Kind::Count, Ok(_)) => true,
            (_, Ok(Value::Boolean(b))) => *b,
            (_, Ok(_)) => true,
            (_, Err(_)) => false,
        };
        let trigger = matches!(kind, Kind::Insert | Kind::Modify | Kind::Delete | Kind::DeleteAll)
            && !matches!(args.first(), None | Some(Value::Boolean(false)));
        self.log.push(Event {
            kind,
            temp,
            table,
            ok,
            trigger,
            nested: !e.outer || calls != e.calls,
            key,
            rows_before,
        });
        if self.log.len() > LOG_CAP {
            self.reset();
        }
    }

    /// Forgets the log: reads before this no longer compare with reads after it.
    fn reset(&mut self) {
        self.resets += 1;
        self.base += self.log.len();
        self.log.clear();
        self.commit_mark = None;
    }

    /// Whether `m` needs its primary key noted.
    pub(crate) fn wants_key(m: RecordOp) -> bool {
        matches!(kind(m), Kind::Get | Kind::Insert | Kind::Modify | Kind::Delete)
    }

    /// A Commit made the data durable.
    pub(crate) fn committed(&mut self) {
        self.ops_at_commit = Some(al_data::memory::store_ops());
        if self.active {
            self.commit_mark = Some(self.base + self.log.len());
            self.trim();
        }
    }

    /// A rollback: what the table held since the Commit is gone, and with it the context.
    pub(crate) fn rolled_back(&mut self) {
        self.ops_at_commit = None;
        self.resets += 1;
        self.commit_mark = None;
    }

    /// Drops the events no cursor and no context needs any more.
    fn trim(&mut self) {
        let at = |c: &Cursor| if c.read { c.at } else { usize::MAX };
        let keep = [at(&self.stmt), at(&self.rows), self.commit_mark.unwrap_or(usize::MAX)]
            .into_iter()
            .min()
            .unwrap_or(0);
        if keep > self.base {
            let n = (keep - self.base).min(self.log.len());
            self.log.drain(..n);
            self.base += n;
        }
    }

    /// A read of `counter`.
    pub(crate) fn read(&mut self, counter: SqlCounter, commits: u64) -> Value {
        if !self.active {
            self.active = true;
            self.base = 0;
            self.log.clear();
            self.commit_mark = None;
        }
        let outside = al_data::memory::store_ops() - self.inside;
        let now = self.base + self.log.len();
        let cur = match counter {
            SqlCounter::Statements => &self.stmt,
            SqlCounter::RowsRead => &self.rows,
        };
        let change = if !(cur.read && cur.commits == commits && cur.resets == self.resets && cur.outside == outside) {
            None
        } else if cur.at == now {
            // Nothing between the reads (SqlCounters.al:105-119).
            Some(Change::Exact(0))
        } else if let Some(size) = cur.clean.then(|| sized(self.events(cur.at, now))).flatten() {
            Some(Change::Exact(pick(counter, size)))
        } else {
            self.classify(cur.at, now)
                .map(|signs| Change::Sign(pick(counter, signs)))
        };
        let resets = self.resets;
        let clean = self.ops_at_commit == Some(al_data::memory::store_ops());
        let fresh = {
            self.chains = self.chains.wrapping_add(1);
            self.chains
        };
        let cur = match counter {
            SqlCounter::Statements => &mut self.stmt,
            SqlCounter::RowsRead => &mut self.rows,
        };
        match change {
            Some(Change::Exact(size)) => cur.n += size,
            Some(Change::Sign(up)) => {
                cur.n += i64::from(up);
                cur.chain = fresh;
            }
            None => {
                cur.epoch += 1;
                cur.chain = fresh;
            }
        }
        cur.clean = clean;
        cur.read = true;
        cur.at = now;
        cur.commits = commits;
        cur.resets = resets;
        cur.outside = outside;
        let v = Value::SqlCount(SqlCount {
            n: cur.n,
            form: SqlForm::Read {
                counter,
                epoch: cur.epoch,
                chain: cur.chain,
            },
        });
        self.trim();
        v
    }

    fn events(&self, from: usize, to: usize) -> &[Event] {
        &self.log[from - self.base..to - self.base]
    }

    /// The signs (statements, rows read) of the change between the events `from..to`, where the pins decide them.
    fn classify(&self, from: usize, to: usize) -> Option<(bool, bool)> {
        let seg = self.events(from, to);
        if seg.is_empty() {
            return Some((false, false));
        }
        if seg.iter().any(|e| e.nested || e.trigger) {
            return None;
        }
        if seg.iter().all(|e| e.temp) {
            return (seg.iter().all(|e| e.table == seg[0].table) && one_step(seg).is_some()).then_some((false, false));
        }
        // A database table: the events since the Commit, this segment last, are the measured sequence up to this step.
        let mark = self.commit_mark?;
        if mark > from {
            return None;
        }
        let all = self.events(mark, to);
        if all
            .iter()
            .any(|e| e.nested || e.trigger || e.temp || e.table != seg[0].table)
        {
            return None;
        }
        let mut at = 0;
        let mut inserted: Option<&Vec<Value>> = None;
        for (want, stmt, rows) in TABLE_STEPS {
            let len = step_len(&all[at..], want)?;
            let step = &all[at..at + len];
            match want {
                Kind::Insert => inserted = step[0].key.as_ref(),
                Kind::Modify | Kind::Delete => match (step[0].key.as_ref(), inserted) {
                    (Some(a), Some(b)) if al_builtins::ops::compare_keys(a, b) => {}
                    _ => return None,
                },
                _ => {}
            }
            let start = mark + at;
            at += len;
            if at == all.len() {
                // This segment must be exactly the last step.
                return (start == from).then_some((stmt, rows));
            }
        }
        None
    }
}

/// The length of the pinned step `want` at the start of `evs`: one Get that found a row, one FindSet that found a row with
/// Next to the end (at least one Next, the last one not moving), one Count, or one Insert, Modify or Delete without
/// RunTrigger that wrote.
fn step_len(evs: &[Event], want: Kind) -> Option<usize> {
    let first = evs.first()?;
    if first.kind != want || !first.ok {
        return None;
    }
    if want != Kind::FindSet {
        return Some(1);
    }
    let mut i = 1;
    while evs.get(i).is_some_and(|e| e.kind == Kind::Next && e.ok) {
        i += 1;
    }
    match evs.get(i) {
        Some(e) if e.kind == Kind::Next && !e.ok => Some(i + 1),
        _ => None,
    }
}

/// The part of a (statements, rows read) pair that is `counter`'s.
fn pick<T>(counter: SqlCounter, (statements, rows): (T, T)) -> T {
    match counter {
        SqlCounter::Statements => statements,
        SqlCounter::RowsRead => rows,
    }
}

/// The N the sizes were measured for (SqlCounters.al:261-267).
const SIZED_N: [usize; 5] = [1, 2, 5, 10, 20];
/// Statements of N inserts, and of N inserts and a Count, into an empty table (SqlCounters.al:323-332): inserts are buffered.
const INSERT_STATEMENTS: [i64; 5] = [0, 0, 1, 2, 4];
const INSERT_COUNT_STATEMENTS: [i64; 5] = [2, 3, 2, 3, 5];

/// The pinned size (statements, rows read) of a segment that started right after a Commit, where the corpus measured it
/// (SqlCounters.al:204-395, `RoMeasure`), with N the rows the table held then (the inserts: the rows inserted):
///
/// - N times Init and Insert into an empty table: 0/0, 0/0, 1/0, 2/0, 4/0; with a Count after them: 2/1, 3/1, 2/1, 3/1, 5/1;
/// - N Gets of distinct rows: N/N; N Gets of one row: 1/1; a FindSet with Next to the end: 1/(N+1);
/// - N times Get and Modify of distinct rows: 2N/2N; Get and Delete: 2N/N; a DeleteAll: 1/0; a Count: 1/1.
///
/// Every event is a method that succeeded, without RunTrigger and without AL code inside, on one table of the measured
/// shape ([`Event::rows_before`]). Anything else (another N, order, mix, table or record state) is not sized.
fn sized(seg: &[Event]) -> Option<(i64, i64)> {
    let first = seg.first()?;
    let rows = first.rows_before?;
    if seg.len() > 2 * SIZED_N[4] + 1
        || seg
            .iter()
            .any(|e| e.rows_before.is_none() || e.table != first.table || e.temp || e.nested || e.trigger)
    {
        return None;
    }
    let index = |n: usize| SIZED_N.iter().position(|&m| m == n);
    let same_key = |a: &Event, b: &Event| match (&a.key, &b.key) {
        (Some(x), Some(y)) => al_builtins::ops::compare_keys(x, y),
        _ => false,
    };
    let distinct = |evs: &[&Event]| {
        evs.iter().all(|e| e.key.is_some())
            && evs
                .iter()
                .enumerate()
                .all(|(i, a)| evs[i + 1..].iter().all(|b| !same_key(a, b)))
    };
    // Only the last Next of a FindSet loop does not move (`step_len` checks it).
    if seg.iter().any(|e| !e.ok && e.kind != Kind::Next) {
        return None;
    }
    if first.kind == Kind::Init {
        let counted = seg.len() % 2 == 1;
        let pairs = if counted { &seg[..seg.len() - 1] } else { seg };
        if rows != 0
            || (counted && seg[seg.len() - 1].kind != Kind::Count)
            || !pairs
                .chunks(2)
                .all(|p| p[0].kind == Kind::Init && p[1].kind == Kind::Insert)
        {
            return None;
        }
        let i = index(pairs.len() / 2)?;
        return Some(if counted {
            (INSERT_COUNT_STATEMENTS[i], 1)
        } else {
            (INSERT_STATEMENTS[i], 0)
        });
    }
    index(rows)?;
    let n = rows as i64;
    match first.kind {
        Kind::Get if seg.len() == rows && seg.iter().all(|e| e.kind == Kind::Get) => {
            let gets: Vec<&Event> = seg.iter().collect();
            if distinct(&gets) {
                Some((n, n))
            } else if seg.iter().all(|e| same_key(e, first)) {
                Some((1, 1))
            } else {
                None
            }
        }
        Kind::Get if seg.len() == 2 * rows => {
            let write = seg[1].kind;
            let pairs_ok = seg
                .chunks(2)
                .all(|p| p[0].kind == Kind::Get && p[1].kind == write && same_key(&p[0], &p[1]));
            let gets: Vec<&Event> = seg.iter().step_by(2).collect();
            if !pairs_ok || !distinct(&gets) {
                return None;
            }
            match write {
                Kind::Modify => Some((2 * n, 2 * n)),
                Kind::Delete => Some((2 * n, n)),
                _ => None,
            }
        }
        Kind::FindSet if seg.len() == rows + 1 && step_len(seg, Kind::FindSet) == Some(seg.len()) => Some((1, n + 1)),
        Kind::DeleteAll if seg.len() == 1 => Some((1, 0)),
        Kind::Count if seg.len() == 1 => Some((1, 1)),
        _ => None,
    }
}

/// The segment is exactly one pinned step (on a temporary record any of them, in any order).
fn one_step(seg: &[Event]) -> Option<()> {
    let k = seg[0].kind;
    if !matches!(
        k,
        Kind::Get | Kind::FindSet | Kind::Count | Kind::Insert | Kind::Modify | Kind::Delete
    ) {
        return None;
    }
    (step_len(seg, k)? == seg.len()).then_some(())
}

/// A builtin given a counter value (as receiver or argument) would read its number: refused, except the Variant type tests
/// (`IsBigInteger`: SqlCounters.al:87-103). Checked only while the counters are active.
#[cold]
#[inline(never)]
pub(crate) fn guard(
    id: al_ir::BuiltinId,
    recv: Option<&Value>,
    args: &[Value],
) -> Result<(), al_builtins::native::Signal> {
    let tainted = |v: &Value| matches!(v, Value::SqlCount(_));
    if args.iter().any(tainted) || (recv.is_some_and(tainted) && !id.name().starts_with("Variant.Is")) {
        return Err(al_builtins::sqlcount::refusal(&format!(
            "{} given the value",
            id.name()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test that reads a counter once and then runs many record methods keeps a bounded log: past the cap the log is
    /// dropped and the reads before it no longer compare.
    #[test]
    fn the_log_stays_bounded() {
        let mut c = SqlCounters::default();
        let first = c.read(SqlCounter::Statements, 0);
        for _ in 0..(3 * LOG_CAP) {
            let e = c.enter(0);
            c.leave(e, 0, RecordOp::Get, true, 1, &[], None, None, &Ok(Value::Boolean(true)));
        }
        assert!(c.log.len() <= LOG_CAP);
        let Value::SqlCount(a) = first else { panic!() };
        let Value::SqlCount(b) = c.read(SqlCounter::Statements, 0) else {
            panic!()
        };
        assert_ne!(a.form, b.form, "a new epoch after the log was dropped");
    }
}
