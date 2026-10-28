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
//!
//! Database Inserts are buffered, and each record method is also given its cost under the buffer rule that
//! session/lt-bl-sql-counters/SkSqlBudget.al measured ([`SqlCounters::cost`]); a segment of methods that all have a cost is
//! sized by their sum, whatever came before it, as long as the buffer was known throughout.

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
    /// The record and its table are of the shape SkSqlBudget.al:14-29 measured ([`CopyShape`]).
    copy: CopyShape,
    /// What the method cost under the buffer rule ([`SqlCounters::cost`]).
    cost: Cost,
}

/// The cost of one record method under the buffer rule (SkSqlBudget.al).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Cost {
    /// (statements, rows read).
    Exact(i64, i64),
    /// The statements; the rows read are not pinned.
    Statements(i64),
    /// At least this many statements; the rows read are not pinned.
    StatementsAtLeast(i64),
    #[default]
    Unknown,
}

/// What the VM knows about a record method beyond the method itself (`Session::record_op_counted`).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OpInfo {
    /// The record and its table are of a shape whose buffered Inserts the corpus measured
    /// (`Session::sql_buffer_shape`).
    pub(crate) buffer_shape: bool,
    /// The method asked the table's Global Triggers setup (the session's first write to the table), with whether the setup
    /// enabled OnDatabaseInsert and whether it enabled anything else; `None` when it did not ask.
    pub(crate) first_write: Option<(bool, bool)>,
    /// The AL code that called the method is in a dependency app (Microsoft's), not the app under test.
    pub(crate) dependency_code: bool,
}

/// Whether a record of a database table is of the shape whose copy sizes SkSqlBudget.al measured: a table of the fields
/// Code[20], Integer, Text[50], Integer with the first two as its primary key, and a record with no state set but, for
/// `Grouped`, one filter on the first field (the reporter's `SetRange("Group Code", ...)`, SkSqlBudget.al:44-45).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CopyShape {
    #[default]
    No,
    Plain,
    Grouped,
}

/// Whether inserts into a database table may still be buffered: BC sends Inserts in batches, a Count of the table sends
/// what is left (SkSqlBudget.al:8-10), and the next measurement counts what an earlier one left buffered (:273).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Buffer {
    /// None: a Commit, a transaction with no database Insert, or a Count of the only table inserted into since.
    Empty,
    /// Inserts into this table only, since the buffer was last known empty, with how many of them are not sent yet.
    Pending(u32, u32),
    #[default]
    Unknown,
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
    SetRange,
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
        RecordOp::SetRange => Kind::SetRange,
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
    /// No database Insert was buffered at the read ([`Buffer::Empty`]).
    flushed: bool,
    /// The chain this one started from across a change whose lower bound alone is pinned (0: none).
    floor: u32,
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
    /// What database Inserts may be buffered, as far as the logged record methods tell.
    buffer: Buffer,
    /// The store operations outside logged record methods when `buffer` was last brought up to date: more since then may
    /// have inserted rows.
    buffer_outside: u64,
    /// Global Triggers events running ([`SqlCounters::mute_begin`]): their AL code is not logged.
    mute: u32,
    /// AL calls made inside Global Triggers events, all told.
    muted_calls: u64,
    /// Global Triggers events whose AL code wrote, read in the app under test, or touched the store outside a record
    /// method, all told.
    muted_dirty: u64,
    /// Store operations of the outermost record methods run by the AL code of Global Triggers events, all told.
    muted_rec_ops: u64,
    /// Record methods of Global Triggers events that make the event dirty, all told.
    muted_bad: u64,
    /// Record methods running when the outermost muted event started.
    mute_open: u32,
    /// Database tables an Insert into was logged for: a later Insert into one is not the session's first write to it.
    inserted: std::collections::HashSet<u32>,
    /// Database tables written by a logged record method since the last logged read of them.
    written_since_read: std::collections::HashSet<u32>,
}

/// What ran between two reads of a counter.
enum Change {
    /// A pinned size.
    Exact(i64),
    /// At least this size; nothing else is pinned.
    AtLeast(i64),
    /// Only the sign is pinned: increased or not.
    Sign(bool),
}

/// What a record method noted on entry (`SqlCounters::enter`).
pub(crate) struct Entry {
    ops: u64,
    calls: u64,
    outer: bool,
    muted_calls: u64,
    muted_dirty: u64,
}

/// What [`SqlCounters::mute_begin`] noted.
pub(crate) struct Muted {
    ops: u64,
    calls: u64,
    rec_ops: u64,
    bad: u64,
}

/// Database Inserts a segment may hold and still be sized by the buffer rule: the most one measurement of SkSqlBudget.al
/// made (:284, :497).
const BUFFER_RULE_MAX_INSERTS: usize = 20;
/// A buffered Insert sends the buffer when it holds this many rows: one statement for the five (SkSqlBudget.al:264-276,
/// :494-497; SqlCounters.al:323-332).
const BATCH: u32 = 5;
/// A Count sends the rows still buffered one statement each, for up to this many (SkSqlBudget.al:449 three rows,
/// :495 two, :496 one), then counts with one statement that reads one row.
const COUNT_SENDS_MAX: u32 = 3;

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
            muted_calls: self.muted_calls,
            muted_dirty: self.muted_dirty,
        }
    }

    /// A Global Triggers event starts (its setup or a database event, raised inside a record method). Its AL code is not
    /// logged: what it does is part of the record method's cost. The buffer rule pins that cost for the code
    /// SkSqlBudget.al:396-449 ran: Base Application's GlobalTriggerManagement, which reads setup tables, and the test's
    /// subscribers, which read only the RecordRef. So the event stays clean only while its code reads records in
    /// dependency apps alone and touches the store no other way.
    pub(crate) fn mute_begin(&mut self, calls: u64) -> Option<Muted> {
        if !self.active {
            return None;
        }
        if self.mute == 0 {
            self.mute_open = self.open;
        }
        self.mute += 1;
        Some(Muted {
            ops: al_data::memory::store_ops(),
            calls,
            rec_ops: self.muted_rec_ops,
            bad: self.muted_bad,
        })
    }

    /// The event of [`SqlCounters::mute_begin`] ended.
    pub(crate) fn mute_end(&mut self, m: Option<Muted>, calls: u64) {
        let Some(m) = m else { return };
        self.mute -= 1;
        if self.mute == 0 {
            self.muted_calls += calls - m.calls;
            let ops = al_data::memory::store_ops() - m.ops;
            if ops != self.muted_rec_ops - m.rec_ops || self.muted_bad != m.bad {
                self.muted_dirty += 1;
            }
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
        copy: CopyShape,
        result: &Result<Value, al_builtins::native::Signal>,
        info: OpInfo,
    ) {
        self.open -= 1;
        if e.outer {
            self.inside += al_data::memory::store_ops() - e.ops;
        }
        if self.mute > 0 {
            // AL code of a Global Triggers event ([`SqlCounters::mute_begin`]).
            let writes = m.writes()
                || matches!(
                    m,
                    RecordOp::LockTable
                        | RecordOp::AddLink
                        | RecordOp::DeleteLink
                        | RecordOp::DeleteLinks
                        | RecordOp::CopyLinks
                );
            if !temp && (writes || !info.dependency_code) {
                self.muted_bad += 1;
            }
            if self.open == self.mute_open {
                self.muted_rec_ops += al_data::memory::store_ops() - e.ops;
            }
            return;
        }
        let kind = kind(m);
        let ok = ok_of(kind, result);
        let trigger = matches!(kind, Kind::Insert | Kind::Modify | Kind::Delete | Kind::DeleteAll)
            && !matches!(args.first(), None | Some(Value::Boolean(false)));
        let nested = !e.outer || calls != e.calls;
        // AL code ran inside only in Global Triggers events, and it touched no store.
        let global_only =
            e.outer && self.muted_dirty == e.muted_dirty && calls - e.calls == self.muted_calls - e.muted_calls;
        let cost = self.cost(kind, temp, table, ok, trigger, nested && !global_only, result, info);
        self.log.push(Event {
            kind,
            temp,
            table,
            ok,
            trigger,
            nested,
            key,
            rows_before,
            copy,
            cost,
        });
        if self.log.len() > LOG_CAP {
            self.reset();
        }
    }

    /// The cost of a record method under the buffer rule, and what it does to the buffer.
    ///
    /// - On a temporary record, Get, FindSet, Next, Count, Insert, Modify, Delete, Init and SetRange cost nothing
    ///   (SqlCounters.al:161-201; the FindSet loops of SkSqlBudget.al:36-55, :467-471).
    /// - SetRange and Init cost nothing (SkSqlBudget.al:44, :474; SqlCounters.al:236).
    /// - An Insert is buffered; the one that fills the buffer to [`BATCH`] rows sends it with one statement
    ///   (SkSqlBudget.al:264-276, :494-497). The session's first write to a table, an Insert that asks the table's Global
    ///   Triggers setup, costs one statement more when the setup enables nothing (SkSqlBudget.al:446 "plain first") and at
    ///   least two more when it enables OnDatabaseInsert alone (:447-448); later Inserts cost the same with OnDatabaseInsert
    ///   enabled (:449 "gt second 0/0").
    /// - A Count of the table with rows, after a write to it, sends the rows still buffered one statement each, then counts
    ///   with one statement that reads one row (SkSqlBudget.al:449, :493-497; SqlCounters.al:327-332, :388-392).
    ///
    /// Everything else (another table or record shape, a RunTrigger write, AL code that touched the store, a second table
    /// with rows buffered, more buffered rows than measured) has no cost here, and leaves the buffer unknown when it may
    /// have changed it.
    #[allow(clippy::too_many_arguments)]
    fn cost(
        &mut self,
        kind: Kind,
        temp: bool,
        table: u32,
        ok: bool,
        trigger: bool,
        nested: bool,
        result: &Result<Value, al_builtins::native::Signal>,
        info: OpInfo,
    ) -> Cost {
        if temp {
            let free = matches!(
                kind,
                Kind::Get
                    | Kind::FindSet
                    | Kind::Next
                    | Kind::Count
                    | Kind::Insert
                    | Kind::Modify
                    | Kind::Delete
                    | Kind::Init
                    | Kind::SetRange
            );
            return if free && !nested && !trigger && (ok || kind == Kind::Next) {
                Cost::Exact(0, 0)
            } else {
                Cost::Unknown
            };
        }
        let written = self.written_since_read.contains(&table);
        match kind {
            Kind::Insert | Kind::Modify | Kind::Delete | Kind::DeleteAll if ok => {
                self.written_since_read.insert(table);
            }
            Kind::SetRange | Kind::Init => {}
            _ => {
                self.written_since_read.remove(&table);
            }
        }
        let before = self.buffer;
        self.buffer = match (before, kind) {
            _ if nested || trigger || !ok => Buffer::Unknown,
            (Buffer::Empty, Kind::Insert) => Buffer::Pending(table, 1),
            (Buffer::Pending(t, n), Kind::Insert) if t == table => Buffer::Pending(t, (n + 1) % BATCH),
            // A Count sends the table's buffered inserts (SkSqlBudget.al:8-10).
            (Buffer::Pending(t, _), Kind::Count) if t == table => Buffer::Empty,
            // Methods that neither read nor write the table.
            (b, Kind::SetRange | Kind::Init) => b,
            (Buffer::Empty, Kind::Get | Kind::FindSet | Kind::Next | Kind::Count) => Buffer::Empty,
            _ => Buffer::Unknown,
        };
        if nested || trigger || !ok {
            return Cost::Unknown;
        }
        let pending = match before {
            Buffer::Empty => Some(0),
            Buffer::Pending(t, n) if t == table => Some(n),
            _ => None,
        };
        match kind {
            Kind::SetRange | Kind::Init => Cost::Exact(0, 0),
            Kind::Insert => {
                let first = !self.inserted.contains(&table);
                self.inserted.insert(table);
                let (Some(n), true) = (pending, info.buffer_shape) else {
                    return Cost::Unknown;
                };
                let sent = i64::from(n + 1 == BATCH);
                match (first, info.first_write) {
                    (false, None) => Cost::Exact(sent, 0),
                    (true, Some((false, false))) => Cost::Statements(sent + 1),
                    (true, Some((true, false))) => Cost::StatementsAtLeast(sent + 2),
                    // A write before it asked the setup, a setup this Insert did not ask (no subscribers), or a setup that
                    // enables other database triggers: not measured.
                    _ => Cost::Unknown,
                }
            }
            // Only a Count after a write to the table: a Count after a Get and a FindSet of it, with no write between,
            // changed neither counter (SqlCounters.al:139-140, :154).
            Kind::Count => {
                let rows = matches!(result, Ok(Value::Integer(n)) if *n > 0);
                match pending {
                    Some(n) if n <= COUNT_SENDS_MAX && rows && written && info.buffer_shape => {
                        Cost::Exact(i64::from(n) + 1, 1)
                    }
                    _ => Cost::Unknown,
                }
            }
            _ => Cost::Unknown,
        }
    }

    /// The size of the segment `from..to` under the buffer rule, where every method in it has a cost.
    fn buffer_rule(&self, counter: SqlCounter, from: usize, to: usize) -> Option<Change> {
        let seg = self.events(from, to);
        if seg.iter().filter(|e| !e.temp && e.kind == Kind::Insert).count() > BUFFER_RULE_MAX_INSERTS {
            return None;
        }
        let (mut sum, mut bound) = (0, false);
        for e in seg {
            sum += match (e.cost, counter) {
                (Cost::Exact(s, _), SqlCounter::Statements) | (Cost::Statements(s), SqlCounter::Statements) => s,
                (Cost::Exact(_, r), SqlCounter::RowsRead) => r,
                (Cost::StatementsAtLeast(s), SqlCounter::Statements) => {
                    bound = true;
                    s
                }
                _ => return None,
            };
        }
        Some(if bound {
            Change::AtLeast(sum)
        } else {
            Change::Exact(sum)
        })
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
        // A Commit sends every buffered insert (SkSqlBudget.al:276-280: a copy right after a Commit is measured alone).
        self.buffer = Buffer::Empty;
        self.buffer_outside = al_data::memory::store_ops() - self.inside;
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
        // Inserts buffered in the transaction are gone with it: none is sent later.
        self.buffer = Buffer::Empty;
        self.buffer_outside = al_data::memory::store_ops() - self.inside;
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
    ///
    /// `nothing_inserted`: the transaction has inserted no row into a database table (so nothing is buffered).
    pub(crate) fn read(&mut self, counter: SqlCounter, commits: u64, nothing_inserted: bool) -> Value {
        let outside = al_data::memory::store_ops() - self.inside;
        if !self.active {
            self.active = true;
            self.base = 0;
            self.log.clear();
            self.commit_mark = None;
            self.buffer = if nothing_inserted {
                Buffer::Empty
            } else {
                Buffer::Unknown
            };
            self.buffer_outside = outside;
        }
        if self.buffer_outside != outside {
            // Store access no logged record method made (it may have inserted rows).
            self.buffer = if nothing_inserted && self.buffer == Buffer::Empty {
                Buffer::Empty
            } else {
                Buffer::Unknown
            };
            self.buffer_outside = outside;
        }
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
        } else if let Some(change) = self.buffer_rule(counter, cur.at, now) {
            Some(change)
        } else if let Some(size) = cur.clean.then(|| sized(self.events(cur.at, now))).flatten() {
            Some(Change::Exact(pick(counter, size)))
        } else if let Some(size) = cur.flushed.then(|| sized_copy(self.events(cur.at, now))).flatten() {
            Some(Change::Exact(pick(counter, size)))
        } else {
            self.classify(cur.at, now)
                .map(|signs| Change::Sign(pick(counter, signs)))
        };
        let resets = self.resets;
        let clean = self.ops_at_commit == Some(al_data::memory::store_ops());
        let flushed = self.buffer == Buffer::Empty;
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
            Some(Change::AtLeast(size)) => {
                cur.n += size;
                cur.floor = cur.chain;
                cur.chain = fresh;
            }
            Some(Change::Sign(up)) => {
                cur.n += i64::from(up);
                cur.chain = fresh;
                cur.floor = 0;
            }
            None => {
                cur.epoch += 1;
                cur.chain = fresh;
                cur.floor = 0;
            }
        }
        cur.clean = clean;
        cur.flushed = flushed;
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
                floor: cur.floor,
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

/// Whether a record method succeeded: Get and FindSet found a row, Next moved, the others returned without error.
fn ok_of(kind: Kind, result: &Result<Value, al_builtins::native::Signal>) -> bool {
    match (kind, result) {
        (Kind::Next, Ok(Value::Integer(n))) => *n != 0,
        (Kind::Next, _) => false,
        (Kind::Count, Ok(_)) => true,
        (_, Ok(Value::Boolean(b))) => *b,
        (_, Ok(_)) => true,
        (_, Err(_)) => false,
    }
}

/// The pinned size (statements, rows read) of a copy of a temporary record into a database table, started with no insert
/// buffered (SkSqlBudget.al:104-133, :211-298): a FindSet loop over one temporary record, N times an Insert of a new row
/// into one table of [`CopyShape`] between its steps, with or without a SetRange on the first field and a Count after it:
/// N = 1, 2, 5, 10, 20 gives 0/0, 0/0, 1/0, 2/0, 4/0 without the Count, 2/1, 3/1, 2/1, 3/1, 5/1 with it (the sizes of
/// SqlCounters.al:323-334). Anything else is not sized.
fn sized_copy(seg: &[Event]) -> Option<(i64, i64)> {
    let first = seg.first()?;
    if first.kind != Kind::FindSet || !first.temp || seg.iter().any(|e| e.nested || e.trigger) {
        return None;
    }
    // FindSet, then Insert and Next N times.
    let mut n = 0;
    let mut i = 1;
    let mut inserted: Vec<&Event> = Vec::new();
    loop {
        let (ins, next) = (seg.get(i)?, seg.get(i + 1)?);
        let insert_ok = ins.kind == Kind::Insert
            && !ins.temp
            && ins.ok
            && ins.copy == CopyShape::Plain
            && ins.key.is_some()
            && inserted.first().is_none_or(|f| f.table == ins.table)
            && inserted
                .iter()
                .all(|p| !al_builtins::ops::compare_keys(p.key.as_ref().unwrap(), ins.key.as_ref().unwrap()));
        if !insert_ok || next.kind != Kind::Next || !next.temp || next.table != first.table {
            return None;
        }
        inserted.push(ins);
        n += 1;
        i += 2;
        if !next.ok {
            break;
        }
    }
    let table = inserted[0].table;
    let at = SIZED_N.iter().position(|&m| m == n)?;
    match &seg[i..] {
        [] => Some((INSERT_STATEMENTS[at], 0)),
        [set, count]
            if set.kind == Kind::SetRange
                && count.kind == Kind::Count
                && [set, count].iter().all(|e| !e.temp && e.table == table && e.ok)
                && set.copy == CopyShape::Plain
                && count.copy == CopyShape::Grouped =>
        {
            Some((INSERT_COUNT_STATEMENTS[at], 1))
        }
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
        let first = c.read(SqlCounter::Statements, 0, true);
        for _ in 0..(3 * LOG_CAP) {
            let e = c.enter(0);
            c.leave(
                e,
                0,
                RecordOp::Get,
                true,
                1,
                &[],
                None,
                None,
                CopyShape::No,
                &Ok(Value::Boolean(true)),
                OpInfo::default(),
            );
        }
        assert!(c.log.len() <= LOG_CAP);
        let Value::SqlCount(a) = first else { panic!() };
        let Value::SqlCount(b) = c.read(SqlCounter::Statements, 0, true) else {
            panic!()
        };
        assert_ne!(a.form, b.form, "a new epoch after the log was dropped");
    }
}
