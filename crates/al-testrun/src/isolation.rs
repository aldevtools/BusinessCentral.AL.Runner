//! Test isolation: what the test runner resets between the test methods of a test codeunit.
//!
//! BC's test runner codeunits carry a `TestIsolation` property. The corpus pins two of its three
//! values; the third is refused (docs/probes-needed.md).
//!
//! | Value | Database between test methods | Pinned by |
//! |---|---|---|
//! | `Codeunit` | kept, and undone when the codeunit ends | the whole corpus runs under it (Microsoft's `Test Runner - Isol. Codeunit`, 130450); `T5` of the isolation probe fails under it |
//! | `Function` | returned to its state after `OnRun` | isolation probe `T5` and `T6` |
//! | `Disabled` | nothing is undone, not even across codeunits | not pinned for a runner: refused |
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
/// A runner without the property is refused: Microsoft's documentation says its default is
/// `Disabled`, and nothing in the corpus pins that.
pub fn runner_isolation(lowered: &al_sema::lower::Lowered, runner: u32) -> Result<TestIsolation, String> {
    let Some(o) = lowered
        .program
        .objects
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
        None => Err(format!(
            "test runner {runner} {} sets no TestIsolation: its default is not pinned",
            o.name
        )),
    }
}

/// Where a unit's database returns to when a test method ends. Cheap: the clone of a persistent
/// map. Taken once per unit, after `OnRun` (probe `T6`: the rollback target is the state after
/// `OnRun`, not before it).
pub struct Checkpoint(al_data::memory::MemStore);

impl Checkpoint {
    /// The session's committed data now.
    pub fn take(session: &mut Session) -> Checkpoint {
        session.commit_transaction();
        session.media_checkpoint();
        Checkpoint(al_data::SnapshotStore::snapshot(session.store_mut()))
    }

    /// Discards everything since: the open transaction, and any commit the test made.
    pub fn restore(&self, session: &mut Session) {
        session.rollback_transaction();
        al_data::SnapshotStore::restore(session.store_mut(), self.0.clone());
        session.restore_media_checkpoint();
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
        assert!(
            runner_isolation(&l, 61305)
                .unwrap_err()
                .contains("default is not pinned")
        );
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
