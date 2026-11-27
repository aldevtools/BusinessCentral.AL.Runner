//! The stops of a run (tooling, ADR-009: no AL code observes them): which rows end the later tests of their codeunit
//! and the later entries of their scope, and the reason those tests are given. `RunOptions::stop_after_first_failure`
//! (al-runner issue 8210) stops at a failure. The scope is a pass's `stop_group` (issue 8210). `Pass::stop` holds one
//! `Stop` per pass; `corpus::pass_stopped` reads it between passes, `run::run_codeunit_inner` after each test.

use crate::Outcome;
use std::sync::Arc;

/// The scope of a pass's stop (issue 8210): `corpus::pass_stopped` compares scopes by equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StopScope {
    /// The entry has no `stop_group` (or there are no entries): a stop ends every later entry without one (al-runner
    /// issue 8210).
    Request,
    /// The entry's `EntryRun::stop_group`: a stop ends the later entries of this group only.
    Group(Arc<str>),
}

/// The stop of one pass: its scope, and which rows trigger it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stop {
    pub(crate) scope: StopScope,
    /// `RunOptions::stop_after_first_failure`: a `Fail` row stops.
    failure: bool,
}

impl Stop {
    /// The stop of a pass whose entry has the group `group`; `None` when no stop is on.
    pub(crate) fn of(failure: bool, group: Option<&str>) -> Option<Stop> {
        let scope = group.map_or(StopScope::Request, |g| StopScope::Group(Arc::from(g)));
        failure.then_some(Stop { scope, failure })
    }

    /// The reason the later tests of the codeunit, and of the later entries of `scope`, are not run after the row of test
    /// `test` of codeunit `codeunit` ended with `outcome`; `None` when that row does not stop.
    pub(crate) fn after(&self, outcome: &Outcome, codeunit: u32, test: &str) -> Option<String> {
        let later = match &self.scope {
            StopScope::Request => "a later entry".to_string(),
            StopScope::Group(g) => format!("a later entry of its stopGroup `{g}`"),
        };
        match outcome {
            // Without a group it is al-runner issue 8210's text, byte for byte; with one it names the group.
            Outcome::Fail(_) if self.failure => Some(format!(
                "the test Codeunit{codeunit}.{test} failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of {later} after a failure"
            )),
            _ => None,
        }
    }
}

/// The reason of a test `RunOptions::skip_after_timeout` (al-runner issue 7517) does not run, after its codeunit's test
/// `test` was stopped at the time limit.
pub(crate) fn skipped_after_timeout(test: &str) -> String {
    format!(
        "an earlier test of this codeunit ({test}) was stopped at the time limit, and --skip-after-timeout does not run the rest of the codeunit"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_stops_with_the_text_of_issue_8210() {
        let request = Stop::of(true, None).unwrap();
        assert_eq!(
            request
                .after(&Outcome::Fail("x".into()), 50110, "FailsFirst")
                .as_deref(),
            Some(
                "the test Codeunit50110.FailsFirst failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of a later entry after a failure"
            )
        );
        let grouped = Stop::of(true, Some("m1")).unwrap();
        assert!(
            grouped
                .after(&Outcome::Fail("x".into()), 1, "T")
                .is_some_and(|m| m.ends_with("of a later entry of its stopGroup `m1` after a failure"))
        );
        assert_eq!(request.after(&Outcome::Pass, 1, "T"), None);
        assert_eq!(Stop::of(false, Some("m1")), None, "no stop, no scope");
    }
}
