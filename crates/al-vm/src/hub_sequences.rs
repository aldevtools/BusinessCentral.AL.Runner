//! Which number sequences the later tests of a codeunit may not use under BC's TestRunnerHub with
//! RequiredTestIsolation = Function (al-testrun `isolation::after_test`, al-runner issue 9250).
//!
//! BC does not answer stably what a test leaves of a sequence the codeunit's OnRun inserted during the hub's run: test b
//! reads it as missing now and then, on BC 29.0 without a commit after OnRun (corpus issue 1595: SeqNextHubFunction.al
//! E4b, E5b, E1C) and on BC 28.5 even with commits (corpus issue 1614: E2b, O1b). The file's header at corpus master
//! says so (SeqNextHubFunction.al:59-64; cells E2 :232, E4 :409, E5 :503 and E1C, which reads like E2 for all companies
//! without a commit, all refused here). What a test leaves of a sequence the codeunit changed is not measured for any
//! other shape either: OnRun's Next, Range, Restart or Delete on a sequence there before the codeunit began, or a test's
//! change to one there before or to one it inserted. So after each test, every later NumberSequence call on a sequence
//! OnRun changed (any call but Exists and Current), or on one the test changed, is refused (`Session::refuse_unmeasured_sequence`); main kept them as the test left them, which was wrong for the
//! Restart and Delete cells E4 and E5. Entry points: `begin_hub_sequence_log`, `refuse_hub_sequences`.

use crate::Session;
use std::collections::BTreeSet;

type Key = (String, String);

/// The NumberSequence changes of a test codeunit under the hub's Function isolation ([`Session::begin_hub_sequence_log`]).
#[derive(Debug, Default)]
pub(crate) struct Log {
    /// The sequences the codeunit's OnRun inserted or changed (every NumberSequence method that writes: Insert, Next,
    /// Range, Restart, Delete).
    on_run_changed: BTreeSet<Key>,
    /// The sequences changed since the checkpoint after OnRun or since the last test ended.
    changed: BTreeSet<Key>,
    /// The checkpoint after OnRun is taken: the tests run.
    in_tests: bool,
}

impl Session {
    /// A test codeunit under BC's TestRunnerHub with RequiredTestIsolation = Function begins (before its OnRun): its
    /// NumberSequence changes are logged until the codeunit ends (`Session::clear_unmeasured`).
    pub fn begin_hub_sequence_log(&mut self) {
        self.unmeasured.hub_log = Some(Log::default());
    }

    /// The checkpoint after OnRun is taken: the tests begin here.
    pub(crate) fn hub_log_checkpoint(&mut self) {
        if let Some(log) = &mut self.unmeasured.hub_log {
            log.in_tests = true;
            log.changed.clear();
        }
    }

    /// Logs a NumberSequence call `method` on `key` that changes the sequence.
    pub(crate) fn hub_log_call(&mut self, method: &'static str, key: &Key) {
        let Some(log) = &mut self.unmeasured.hub_log else {
            return;
        };
        if let "Insert" | "Next" | "Range" | "Restart" | "Delete" = method {
            let set = if log.in_tests {
                &mut log.changed
            } else {
                &mut log.on_run_changed
            };
            set.insert(key.clone());
        }
    }

    /// A test ended (after its rollback): every later call of the codeunit on a sequence OnRun changed or the test
    /// changed is refused (`why`: the refusal text of a key). The next test begins here.
    pub fn refuse_hub_sequences(&mut self, why: impl Fn(&Key) -> String) {
        let Some(log) = &mut self.unmeasured.hub_log else {
            return;
        };
        let keys: BTreeSet<Key> = log.on_run_changed.union(&log.changed).cloned().collect();
        log.changed.clear();
        for key in keys {
            self.refuse_unmeasured_sequence(&key, why(&key));
        }
    }
}
