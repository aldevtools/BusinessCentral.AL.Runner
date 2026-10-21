//! `SessionInformation.SqlStatementsExecuted` and `SqlRowsRead` (session/lt-bl-sql-counters/SqlCounters.al). BC's counts are not
//! deterministic, and the corpus pins only properties: both are BigIntegers that are not negative (:74, :86), two reads with nothing
//! between them are equal (:105), operations on a temporary record change neither (:161), and on a table (:121) `Get` and a
//! `FindSet` loop raise both, `Count` and `Insert` change neither, `Modify` raises both and `Delete` raises the statements only.
//!
//! The runtime counts one statement and one row for each raise. Each operation the corpus pins adds its change; any other read or
//! write that reaches the database (the store's access count moves outside a pinned operation) makes the change unknown, and a
//! later read in the same test is refused. The first read of a test is answered: nothing it could be compared with in the test
//! came before it.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

#[derive(Debug, Default)]
pub(crate) struct SqlCounters {
    statements: i64,
    rows: i64,
    /// The store's access count when the counters were last known to be exact.
    seen: u64,
    /// Whether this test has read a counter.
    pub(crate) read_in_test: bool,
}

/// How a pinned operation changes the counters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Change {
    statements: i64,
    rows: i64,
    /// The change holds even when the operation ran AL code (a trigger or a subscriber): it raises both counters, and more
    /// statements keep them raised. A change that leaves a counter as it was does not hold then.
    holds_with_calls: bool,
}

/// The change of record method `method` with `args` on a table (not a temporary record), when SqlCounters.al:121-158 pins it:
/// `Get` and `FindSet()` with `Next()` raise both, `Count()` and `Insert()` change neither, `Modify()` raises both and `Delete()`
/// raises the statements. Other methods and other arguments (`Insert(true)`, `FindSet(true)`) are not pinned.
pub(crate) fn pinned_change(method: RecordOp, args: &[Value]) -> Option<Change> {
    let no_trigger = matches!(args, [] | [Value::Boolean(false)]);
    let (statements, rows, holds_with_calls) = match method {
        RecordOp::Get => (1, 1, true),
        RecordOp::FindSet if args.is_empty() => (1, 1, true),
        RecordOp::Next if args.is_empty() => (0, 0, false),
        RecordOp::Count if args.is_empty() => (0, 0, false),
        RecordOp::Insert if no_trigger => (0, 0, false),
        RecordOp::Modify if no_trigger => (1, 1, true),
        RecordOp::Delete if no_trigger => (1, 0, false),
        _ => return None,
    };
    Some(Change {
        statements,
        rows,
        holds_with_calls,
    })
}

impl Session {
    /// After a pinned operation: its change applies when nothing unknown reached the store before it, it succeeded (a `Get`
    /// or `FindSet` that found nothing is not pinned), and it ran no AL code unless its change holds with it.
    pub(crate) fn sql_after_op(&mut self, change: Change, before: (u64, u64), r: &Result<Value, Signal>) {
        let succeeded = matches!(r, Ok(v) if !matches!(v, Value::Boolean(false)));
        let calls_ok = change.holds_with_calls || self.calls_made == before.1;
        if before.0 == self.sql.seen && succeeded && calls_ok {
            self.sql.statements += change.statements;
            self.sql.rows += change.rows;
            self.sql.seen = self.store.accesses();
        }
    }

    /// `SqlStatementsExecuted()` (`statements`) or `SqlRowsRead()`.
    pub(crate) fn sql_counter(&mut self, statements: bool) -> Result<Value, Signal> {
        let now = self.store.accesses();
        if now != self.sql.seen && self.sql.read_in_test {
            return Err(Signal::Unsupported(format!(
                "SessionInformation.{} after a database operation whose change of the counter is not pinned (session/lt-bl-sql-counters/SqlCounters.al pins Get, FindSet, Next, Count, Insert, Modify and Delete on a table, without triggers)",
                if statements {
                    "SqlStatementsExecuted"
                } else {
                    "SqlRowsRead"
                }
            )));
        }
        self.sql.seen = now;
        self.sql.read_in_test = true;
        Ok(Value::BigInteger(if statements {
            self.sql.statements
        } else {
            self.sql.rows
        }))
    }
}
