//! The guard of a test session that runs without the login (al-runner issues 7343, 7574, 7605 and 7717; hard rule 4).
//!
//! BC raises the login before the first test of every test session, so the rows an app under test's login subscriber
//! adds are there (events/platform-triggers/LgnLoginChecklistTests.al:137-138). The runner raises it only for a test
//! codeunit whose static reach says it can see those rows (`al-testrun` `login.rs`), because Microsoft's login code
//! cannot run on every database. That rule is a guess about what the test will read. Where it says no, the front end
//! hands over the tables the login subscribers add rows to, in two sets:
//! - The tables the test's own calls cannot insert into (`guard_login_tables`): the first read of one of them by the
//!   test session is refused, so a wrong "no" costs a test that is not run there. A read is a record method that reads
//!   rows, a FlowField over the table, the TableRelation check and the list caption of a value missing from the table,
//!   and a query dataitem over it.
//! - The tables the test's own calls can insert into (`assume_login_tables`): a read or a write of one of them marks the
//!   assumption `login-rows-not-needed-by-own-inserter`, so a pass there is a flagged pass, and a test that fails after
//!   the mark is reported as not run (`take_login_assumed_hit`). Known wrong in general: such a test that passes may
//!   still fail on BC, where the login's rows are there (a count of the test's own rows, an insert of the login's key).
//!
//! Known limits: a write of a table of the first set is let through (Microsoft's event code may insert at run time), and
//! the app's own Company Triggers subscribers run before the guard is set.

use super::*;

/// What decides whether a read of a table is refused before the session's own code wrote it.
#[derive(Default)]
pub(crate) struct ReadGuards {
    /// The `Engine::demo_tables` the running test's own code (not install code) wrote to (`record_op_inner`).
    pub(crate) demo_written: std::collections::HashSet<u32>,
    /// The tables a read of which is refused because the session runs without the login.
    pub(crate) login: std::collections::BTreeSet<u32>,
    /// The tables the login would add rows to and the test's own calls can insert into: a read or write of one marks
    /// the assumption (`Session::login_guard_table`).
    pub(crate) login_assumed: std::collections::BTreeSet<u32>,
    /// The first table of `login_assumed` the running test read or wrote.
    pub(crate) login_assumed_hit: std::cell::Cell<Option<u32>>,
}

impl ReadGuards {
    /// What the guards forget when a test method starts.
    pub(crate) fn start_test(&mut self) {
        self.demo_written.clear();
        self.login_assumed_hit.set(None);
    }
}

impl Session {
    /// The session runs without the login: a read of one of `tables` (table object indexes) is refused from now on.
    /// An empty set (the login was raised, or no app under test subscribes to it) guards nothing.
    pub fn guard_login_tables(&mut self, tables: std::collections::BTreeSet<u32>) {
        self.guards.login = tables;
    }

    /// The session runs without the login: a read or write of one of `tables` (table object indexes), which the test's own
    /// calls can insert into too, marks the assumption `login-rows-not-needed-by-own-inserter` from now on.
    pub fn assume_login_tables(&mut self, tables: std::collections::BTreeSet<u32>) {
        self.guards.login_assumed = tables;
    }

    /// The name of the first table of [`Session::assume_login_tables`] the running test read or wrote, and forgets it:
    /// a failure after that is not trusted, because the rows the login would have added were not there.
    pub fn take_login_assumed_hit(&self) -> Option<String> {
        let t = self.guards.login_assumed_hit.take()?;
        Some(self.login_table_name(t).to_string())
    }

    fn login_table_name(&self, t: u32) -> &str {
        self.engine
            .program
            .objects
            .get(t as usize)
            .map_or("?", |o| o.name.as_str())
    }

    /// A record method on a table the login would add rows to ([`Session::login_guard_table`]). Writes of a guarded table
    /// are let through: what a write needs it reads through the store, and Microsoft's event code may insert at run time.
    /// Writes of a table the test's own calls can insert into mark the assumption: whether they succeed depends on the
    /// login's rows. A temporary record reads no row of the table.
    pub(crate) fn login_guard_read(&self, method: RecordOp, rec: &Rec) -> Result<(), Signal> {
        if self.guards.login.is_empty() && self.guards.login_assumed.is_empty() {
            return Ok(());
        }
        let reads = matches!(
            method,
            RecordOp::Get
                | RecordOp::GetBySystemId
                | RecordOp::Find
                | RecordOp::FindFirst
                | RecordOp::FindLast
                | RecordOp::FindSet
                | RecordOp::Next
                | RecordOp::IsEmpty
                | RecordOp::Count
                | RecordOp::CountApprox
                | RecordOp::CalcSums
        );
        let writes = matches!(
            method,
            RecordOp::Insert
                | RecordOp::Modify
                | RecordOp::ModifyAll
                | RecordOp::Delete
                | RecordOp::DeleteAll
                | RecordOp::Rename
        );
        let (table, temporary) = {
            let r = rec.borrow();
            (r.table, r.temporary)
        };
        if temporary || !(reads || (writes && self.guards.login_assumed.contains(&table))) {
            return Ok(());
        }
        self.login_guard_table(table, method.name())
    }

    /// A read of the stored rows of table `table` (a table object index) by `what`: refused when the session runs without
    /// the login and a login subscriber adds rows to the table that the test's own calls cannot insert; marks the
    /// assumption when they can. Install code runs before the guard is set. The caller passes no temporary record.
    pub(crate) fn login_guard_table(&self, table: u32, what: &str) -> Result<(), Signal> {
        if self.install_scope {
            return Ok(());
        }
        if self.guards.login_assumed.contains(&table) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::LoginRowsNotNeededByOwnInserter);
            if self.guards.login_assumed_hit.get().is_none() {
                self.guards.login_assumed_hit.set(Some(table));
            }
            return Ok(());
        }
        if !self.guards.login.contains(&table) {
            return Ok(());
        }
        unsupported(format!(
            "{what} on {}: the login was not raised for this test codeunit, and a login subscriber of an app under test adds rows to this table that are not there (the test's reach did not show that it reads them)",
            self.login_table_name(table)
        ))
    }
}
