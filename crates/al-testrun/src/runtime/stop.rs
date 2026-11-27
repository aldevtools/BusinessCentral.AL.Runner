//! The stops of a run (tooling, ADR-009: no AL code observes them): which rows end the later tests of their codeunit
//! and the later entries of their scope, and the reason those tests are given. `RunOptions::stop_after_first_failure`
//! (al-runner issue 8210) stops at a failure, `RunOptions::stop_after_timeout` (issue 9213) at a row the time limit
//! stopped; with both, at the first row either names. The scope is a pass's `stop_group` (issue 8210). `Pass::stop` holds one
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
    /// `RunOptions::stop_after_timeout`: a row the time limit stopped stops.
    timeout: bool,
}

impl Stop {
    /// The stop of a pass whose entry has the group `group`; `None` when no stop is on.
    pub(crate) fn of(failure: bool, timeout: bool, group: Option<&str>) -> Option<Stop> {
        let scope = group.map_or(StopScope::Request, |g| StopScope::Group(Arc::from(g)));
        (failure || timeout).then_some(Stop {
            scope,
            failure,
            timeout,
        })
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
            // Issue 9213: "The trigger is exactly a row whose outcome is a time-limit `notRun`: the CPU limit, the wall
            // backstop, the Sleep budget, or an OnRun cut." An OnRun cut names the trigger, not a test that did not
            // cause the timeout (its rows all carry the OnRun reason).
            Outcome::NotRun(m) if self.timeout && super::run::is_time_limit(m) => {
                let what = match m.contains(super::run::ON_RUN_PLACE) {
                    true => format!("the trigger Codeunit{codeunit}.OnRun"),
                    false => format!("the test Codeunit{codeunit}.{test}"),
                };
                Some(format!(
                    "{what} was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of {later} after a timeout"
                ))
            }
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
        let request = Stop::of(true, false, None).unwrap();
        assert_eq!(
            request
                .after(&Outcome::Fail("x".into()), 50110, "FailsFirst")
                .as_deref(),
            Some(
                "the test Codeunit50110.FailsFirst failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of a later entry after a failure"
            )
        );
        let grouped = Stop::of(true, false, Some("m1")).unwrap();
        assert!(
            grouped
                .after(&Outcome::Fail("x".into()), 1, "T")
                .is_some_and(|m| m.ends_with("of a later entry of its stopGroup `m1` after a failure"))
        );
        assert_eq!(request.after(&Outcome::Pass, 1, "T"), None);
        assert_eq!(Stop::of(false, false, Some("m1")), None, "no stop, no scope");
    }

    /// A time-limit row as `run::time_limit_reason` words it, for a test method (`place` empty) or the OnRun trigger.
    fn time_limit_row(place: &str) -> Outcome {
        Outcome::NotRun(format!(
            "the test used more than the time limit of 1 s of wall time and was stopped (--test-timeout-secs){place}; BC may have ended it differently or not at all"
        ))
    }

    /// Issue 9213: a row the time limit stopped stops, naming the test, or `Codeunit<id>.OnRun` for an OnRun cut.
    #[test]
    fn a_timeout_stops_naming_the_test_or_the_onrun_trigger() {
        let stop = Stop::of(false, true, None).unwrap();
        assert_eq!(
            stop.after(&time_limit_row(""), 50720, "B_TimesOut").as_deref(),
            Some(
                "the test Codeunit50720.B_TimesOut was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of a later entry after a timeout"
            )
        );
        let on_run = time_limit_row(" in the test codeunit's OnRun trigger, before its tests");
        assert_eq!(
            stop.after(&on_run, 50723, "H_Passes").as_deref(),
            Some(
                "the trigger Codeunit50723.OnRun was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of a later entry after a timeout"
            )
        );
        let grouped = Stop::of(false, true, Some("m1")).unwrap();
        assert!(
            grouped
                .after(&time_limit_row(""), 1, "T")
                .is_some_and(|m| m.ends_with("of a later entry of its stopGroup `m1` after a timeout"))
        );
        // Each trigger alone keeps its behavior.
        assert_eq!(stop.after(&Outcome::Fail("x".into()), 1, "T"), None);
        assert_eq!(
            Stop::of(true, false, None).unwrap().after(&time_limit_row(""), 1, "T"),
            None
        );
    }

    /// Issue 9213: never a refusal or a row a stop skipped. No stop text reads as a time-limit row, or the outputs would
    /// turn a stopped row into a timeout row and a stop would trigger on its own rows.
    #[test]
    fn a_refusal_or_a_stopped_row_does_not_stop() {
        let both = Stop::of(true, true, None).unwrap();
        let texts = [
            both.after(&time_limit_row(""), 1, "T").unwrap(),
            both.after(
                &time_limit_row(" in the test codeunit's OnRun trigger, before its tests"),
                1,
                "T",
            )
            .unwrap(),
            both.after(&Outcome::Fail("x".into()), 1, "T").unwrap(),
            skipped_after_timeout("T"),
            "needs a .NET host; none is configured".to_string(),
        ];
        for t in texts {
            assert!(!super::super::run::is_time_limit(&t), "{t}");
            assert_eq!(both.after(&Outcome::NotRun(t.clone()), 1, "U"), None, "{t}");
        }
    }
}
