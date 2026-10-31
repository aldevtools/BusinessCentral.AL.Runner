//! Test isolation: what the test runner resets between the test methods of a test codeunit.
//!
//! BC's test runner codeunits carry a `TestIsolation` property. The corpus nightly pins two of its three
//! values; all three and a runner without the property are the probe of corpus writer SD (tests/al-language-isolation-probe,
//! runners 61300, 61301, 61304 and 61305, measured on real BC 27.5, 28.5 and 29.0 on Windows containers).
//!
//! | Value | Database between test methods | Pinned by |
//! |---|---|---|
//! | `Codeunit` | kept, and undone when the codeunit ends | the whole corpus runs under it (Microsoft's `Test Runner - Isol. Codeunit`, 130450); `T5` of the isolation probe fails under it |
//! | `Function` | returned to its state after `OnRun` | isolation probe `T5` and `T6` |
//! | `Disabled` | nothing is undone, not even across codeunits, but a failed test's writes since its last `Commit()` | corpus PR 735 (IsolProbeWriter, IsolProbeReader, SdIsolCommitError, SdIsolCommitReader under runner 61304) |
//!
//! BC's TestRunnerHub (the corpus CI on BC 28 and later, for a codeunit without handlers) takes the isolation from the
//! test codeunit's `RequiredTestIsolation` instead (`RunOptions::required_isolation`). There a `Commit()` inside a test
//! is an inner commit: Function still returns the next test to the start, Codeunit still undoes everything when the
//! codeunit ends, and Disabled keeps everything for the later codeunits (corpus
//! codeunit/lt-qu-commit-isolation/QuCommitIsolation.al:25-39).
//!
//! Under every value the test codeunit's instance is reused across its methods: AL globals and
//! single-instance codeunits keep their values from one test to the next (probe `T2` to `T4`), and
//! `OnRun` runs once on that instance (`T7`). That is why a test codeunit, never a single test
//! method, is the **unit** of scheduling: methods of one codeunit see each other's AL state, so
//! they cannot run in parallel on forks of it. Only the database is reset per method, and that is
//! a restore of a persistent-map root (ADR-004), O(1).
//!
//! A unit starts from a fork of the shared install baseline ([`al_vm::Baseline`], also O(1)) and
//! shares nothing mutable with another unit; the parallel scheduler (not built yet) runs units on
//! threads and needs nothing else from this module.

use al_vm::Session;

/// The `TestIsolation` property of a test runner codeunit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TestIsolation {
    /// Nothing is rolled back between tests or test codeunits.
    Disabled,
    /// The database is rolled back when the test codeunit ends. The default of this runner,
    /// because every corpus oracle was recorded under `Test Runner - Isol. Codeunit`.
    #[default]
    Codeunit,
    /// The database is rolled back to its state after `OnRun` when each test method ends.
    Function,
}

impl TestIsolation {
    /// The value a `TestIsolation = X;` property or a `--isolation` flag names.
    pub fn parse(text: &str) -> Option<TestIsolation> {
        match text.trim().trim_matches(['"', '\'']).to_ascii_lowercase().as_str() {
            "disabled" => Some(TestIsolation::Disabled),
            "codeunit" => Some(TestIsolation::Codeunit),
            "function" => Some(TestIsolation::Function),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            TestIsolation::Disabled => "Disabled",
            TestIsolation::Codeunit => "Codeunit",
            TestIsolation::Function => "Function",
        }
    }
}

/// The isolation a test runner codeunit of the compiled program asks for: its `TestIsolation`
/// property. The runner's AL (it calls the platform's test tool) is never run; only the property
/// is read, because this harness is the runner (corpus tests/al-language-isolation-probe, whose
/// `-testRunnerCodeunitId` runners differ from each other in nothing but this property).
///
/// A runner without the property runs as `Disabled`, as Microsoft's documentation says and the probe measured.
pub fn runner_isolation(lowered: &al_sema::lower::Lowered, runner: u32) -> Result<TestIsolation, String> {
    let Some(o) = lowered
        .program
        .objects
        .heads()
        .iter()
        .find(|o| o.is_kind(al_ir::ObjKind::Codeunit) && o.id == Some(runner))
    else {
        return Err(format!("test runner codeunit {runner} is not in the compiled apps"));
    };
    let property = |name: &str| {
        o.properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    };
    if !property("Subtype").is_some_and(|v| v.trim().eq_ignore_ascii_case("TestRunner")) {
        return Err(format!(
            "codeunit {runner} {} is not a test runner (Subtype = TestRunner)",
            o.name
        ));
    }
    match property("TestIsolation") {
        Some(v) => TestIsolation::parse(v)
            .ok_or_else(|| format!("test runner {runner} {}: unknown TestIsolation `{v}`", o.name)),
        // A runner without the property gave exactly the rows of the Disabled runner on BC 27.5, 28.5 and 29.0 (corpus PR
        // 735, tests/al-language-isolation-probe/src/IsolProbeRunnerNoProperty.Codeunit.al, runner 61305 against 61304;
        // measured on Windows containers by writer SD; the nightly does not run it yet).
        None => Ok(TestIsolation::Disabled),
    }
}

/// Where a unit's database returns to when a test method ends. Cheap: the clone of a persistent
/// map. Taken once per unit, after `OnRun` (probe `T6`: the rollback target is the state after
/// `OnRun`, not before it).
pub struct Checkpoint(al_data::memory::MemStore, al_vm::StorageCheckpoint);

impl Checkpoint {
    /// The session's committed data now.
    pub fn take(session: &mut Session) -> Checkpoint {
        session.commit_transaction();
        session.media_checkpoint();
        let storage = session.storage_checkpoint();
        Checkpoint(al_data::SnapshotStore::snapshot(session.store_mut()), storage)
    }

    /// Discards everything since: the open transaction, and any commit the test made. The isolated storage and the
    /// number sequences stay as the test left them (BC's TestRunnerHub under RequiredTestIsolation = Function; not
    /// measured there).
    pub fn restore(&self, session: &mut Session) {
        session.rollback_transaction();
        session.crm_restored();
        al_data::SnapshotStore::restore(session.store_mut(), self.0.clone());
        session.restore_media_checkpoint();
    }

    /// [`Checkpoint::restore`] as a test runner codeunit with TestIsolation = Function does it: the isolated storage and
    /// the number sequences go back too. Measured for a new key's `IsolatedStorage.Set` and a new sequence's
    /// `NumberSequence.Insert` (corpus PR 735, tests/al-language-isolation-probe/src/IsolProbeAfterRollback.Codeunit.al
    /// S7-S9 fail under runner 61300), while AutoIncrement counters and row versions are not reset (S5, S6 pass: the
    /// store's counters are not part of the snapshot). `None`: neither changed; `Some(false)`: the test changed a key or a sequence that was there
    /// before it, which the probe does not measure.
    pub fn restore_for_runner(&self, session: &mut Session) -> Option<bool> {
        self.restore(session);
        session.restore_storage_checkpoint(&self.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_three_values_in_any_case_and_refuses_the_rest() {
        assert_eq!(TestIsolation::parse("Function"), Some(TestIsolation::Function));
        assert_eq!(TestIsolation::parse(" codeunit "), Some(TestIsolation::Codeunit));
        assert_eq!(TestIsolation::parse("DISABLED"), Some(TestIsolation::Disabled));
        assert_eq!(TestIsolation::parse("Test"), None);
    }

    fn lowered(src: &str) -> al_sema::lower::Lowered {
        al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        )
        .1
    }

    // corpus tests/al-language-isolation-probe/src/IsolProbeRunnerFunction.Codeunit.al:13 and
    // IsolProbeRunnerCodeunit.Codeunit.al:11.
    #[test]
    fn a_runners_isolation_is_its_property() {
        let l = lowered(
            "codeunit 61300 F { Subtype = TestRunner; TestIsolation = Function; }
             codeunit 61301 C { Subtype = TestRunner; TestIsolation = Codeunit; }
             codeunit 61304 D { Subtype = TestRunner; TestIsolation = Disabled; }
             codeunit 61305 N { Subtype = TestRunner; }
             codeunit 61306 T { Subtype = Test; }",
        );
        assert_eq!(runner_isolation(&l, 61300), Ok(TestIsolation::Function));
        assert_eq!(runner_isolation(&l, 61301), Ok(TestIsolation::Codeunit));
        assert_eq!(runner_isolation(&l, 61304), Ok(TestIsolation::Disabled));
        // IsolProbeRunnerNoProperty.Codeunit.al (corpus PR 735): the rows of the Disabled runner.
        assert_eq!(runner_isolation(&l, 61305), Ok(TestIsolation::Disabled));
        assert!(runner_isolation(&l, 61306).unwrap_err().contains("not a test runner"));
        assert!(
            runner_isolation(&l, 1)
                .unwrap_err()
                .contains("not in the compiled apps")
        );
    }

    #[test]
    fn the_default_is_codeunit_isolation() {
        assert_eq!(TestIsolation::default(), TestIsolation::Codeunit);
    }
}
