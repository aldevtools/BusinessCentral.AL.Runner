//! The model behind `SessionInformation.SqlStatementsExecuted()` and `SqlRowsRead()` (issue 5580).
//!
//! The corpus pins the sign of a counter's change between two reads, never a count
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
    /// The primary key, for Insert, Modify and Delete.
    key: Option<Vec<Value>>,
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
        let trigger = matches!(kind, Kind::Insert | Kind::Modify | Kind::Delete)
            && !matches!(args.first(), None | Some(Value::Boolean(false)));
        self.log.push(Event {
            kind,
            temp,
            table,
            ok,
            trigger,
            nested: !e.outer || calls != e.calls,
            key,
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
        matches!(kind(m), Kind::Insert | Kind::Modify | Kind::Delete)
    }

    /// A Commit made the data durable.
    pub(crate) fn committed(&mut self) {
        if self.active {
            self.commit_mark = Some(self.base + self.log.len());
            self.trim();
        }
    }

    /// A rollback: what the table held since the Commit is gone, and with it the context.
    pub(crate) fn rolled_back(&mut self) {
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
        let change = if cur.read && cur.commits == commits && cur.resets == self.resets && cur.outside == outside {
            self.classify(cur.at, now).map(|(s, r)| match counter {
                SqlCounter::Statements => s,
                SqlCounter::RowsRead => r,
            })
        } else {
            None
        };
        let resets = self.resets;
        let cur = match counter {
            SqlCounter::Statements => &mut self.stmt,
            SqlCounter::RowsRead => &mut self.rows,
        };
        match change {
            Some(up) => cur.n += i64::from(up),
            None => cur.epoch += 1,
        }
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
            c.leave(e, 0, RecordOp::Get, true, 1, &[], None, &Ok(Value::Boolean(true)));
        }
        assert!(c.log.len() <= LOG_CAP);
        let Value::SqlCount(a) = first else { panic!() };
        let Value::SqlCount(b) = c.read(SqlCounter::Statements, 0) else {
            panic!()
        };
        assert_ne!(a.form, b.form, "a new epoch after the log was dropped");
    }
}
