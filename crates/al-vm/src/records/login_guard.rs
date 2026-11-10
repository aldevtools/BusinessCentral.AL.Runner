//! The guard of a test session that runs without the login (al-runner issues 7343, 7574 and 7605; hard rule 4).
//!
//! BC raises the login before the first test of every test session, so the rows an app under test's login subscriber
//! adds are there (events/platform-triggers/LgnLoginChecklistTests.al:137-138). The runner raises it only for a test
//! codeunit whose static reach says it can see those rows (`al-testrun` `login.rs`), because Microsoft's login code
//! cannot run on every database. That rule is a guess about what the test will read. Where it says no, the front end
//! hands over the tables the login subscribers add rows to and the test's own calls cannot insert into; the first read
//! of one of them by the test session is refused. A wrong "no" then costs a test that is not run, never a read of a
//! table without the rows BC has there.

use super::*;

/// What decides whether a read of a table is refused before the session's own code wrote it.
#[derive(Default)]
pub(crate) struct ReadGuards {
    /// The `Engine::demo_tables` the running test's own code (not install code) wrote to (`record_op_inner`).
    pub(crate) demo_written: std::collections::HashSet<u32>,
    /// The tables a read of which is refused because the session runs without the login.
    pub(crate) login: std::collections::BTreeSet<u32>,
}

impl Session {
    /// The session runs without the login: a read of one of `tables` (table object indexes) is refused from now on.
    /// An empty set (the login was raised, or no app under test subscribes to it) guards nothing.
    pub fn guard_login_tables(&mut self, tables: std::collections::BTreeSet<u32>) {
        self.guards.login = tables;
    }

    /// Refuses a read of a guarded table. Writes are let through: what a write needs it reads through the store, and a
    /// table the test's own calls insert into is not guarded in the first place. A temporary record reads no row of the
    /// table. Install code runs before the guard is set.
    pub(crate) fn login_guard_read(&self, method: RecordOp, rec: &Rec) -> Result<(), Signal> {
        if self.guards.login.is_empty() {
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
                | RecordOp::IsEmpty
                | RecordOp::Count
                | RecordOp::CountApprox
                | RecordOp::CalcSums
        );
        let r = rec.borrow();
        if !reads || r.temporary || self.install_scope || !self.guards.login.contains(&r.table) {
            return Ok(());
        }
        let name = self
            .engine
            .program
            .objects
            .get(r.table as usize)
            .map_or("?", |o| o.name.as_str());
        unsupported(format!(
            "{} on {name}: the login was not raised for this test codeunit, and a login subscriber of an app under test adds rows to this table that are not there (the test's reach did not show that it reads them)",
            method.name()
        ))
    }
}
