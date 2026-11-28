//! [`DemoWritten`]: the demo tables (`Engine::demo_tables`) the running test's own code wrote to, as the transaction
//! leaves them. A write that a rollback undoes leaves the table as it was before (al-runner issue 9277; asserterror undoes
//! the writes since the last Commit, codeunits/TestScopeIsolationContracts.al:81-101 and codeunits/run-onrun/
//! QuCommitIsolation.al:28, X07), so the demo-row rules of `records/op_inner.rs` read it as unwritten again. Install code
//! does not mark a table here; a Count or IsEmpty of a demo table the running test has not written is refused there.

use std::collections::BTreeSet;

/// The written demo tables now and at the last commit; `Session::commit_transaction` and `Session::rollback_transaction`
/// move them, `Session::start_test_method` clears both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DemoWritten {
    now: BTreeSet<u32>,
    committed: BTreeSet<u32>,
}

impl DemoWritten {
    /// The running test's own code wrote demo table `object` (object index).
    pub(crate) fn note(&mut self, object: u32) {
        self.now.insert(object);
    }

    /// Whether a write of demo table `object` by the running test stands.
    pub(crate) fn contains(&self, object: u32) -> bool {
        self.now.contains(&object)
    }

    /// A new session's value: no write now and none a rollback would put back.
    pub(crate) fn is_empty(&self) -> bool {
        self.now.is_empty() && self.committed.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn commit(&mut self) {
        self.committed = self.now.clone();
    }

    pub(crate) fn rollback(&mut self) {
        self.now = self.committed.clone();
    }
}
