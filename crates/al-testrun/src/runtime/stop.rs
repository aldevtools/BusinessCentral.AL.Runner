//! The stops of a run (tooling, ADR-009: no AL code observes them): which rows end the later tests of their codeunit
//! and the later entries of their scope, and the reason those tests are given. `RunOptions::stop_after_first_failure`
//! (al-runner issue 8210) stops at a failure, `RunOptions::stop_after_timeout` (issue 9213) at a row the time limit
//! stopped; with both, at the first row either names. The scope is a pass's `stop_group` (issue 8210). `Pass::stop` holds one
//! `Stop` per pass. `run::run_codeunit_inner` asks it after each test (and after an OnRun cut) and keeps the reason of the
//! first row that triggered it on the codeunit's scratch (`Scratch::stop_trigger`); `corpus::pass_stopped` reads that
//! record between passes and never matches the rows' texts again. One owner of which row triggered.

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
    /// `test` of codeunit `codeunit` ended with the judged `outcome`; `hit_time_limit`: the test began and its outcome
    /// before the trust judging was a time-limit row. `None` when that row does not stop.
    pub(crate) fn after(&self, outcome: &Outcome, hit_time_limit: bool, codeunit: u32, test: &str) -> Option<String> {
        match outcome {
            // Without a group it is al-runner issue 8210's text, byte for byte; with one it names the group. A failure
            // the judging does not trust is a `NotRun` row and does not stop: whether the test fails on BC is what is in
            // doubt.
            Outcome::Fail(_) if self.failure => Some(format!(
                "the test Codeunit{codeunit}.{test} failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of {} after a failure",
                self.later()
            )),
            // Issue 9213: "The trigger is exactly a row whose outcome is a time-limit `notRun`: the CPU limit, the wall
            // backstop, the Sleep budget, or an OnRun cut." The fact before the judging, as for `skip_after_timeout`: a
            // timeout the judging does not trust still reached the limit here (trust is about whether BC would have
            // got there, not about whether the limit fired), so it stops too (review of the pull request for 9213).
            _ if self.timeout && hit_time_limit => Some(self.timed_out(&format!("the test Codeunit{codeunit}.{test}"))),
            _ => None,
        }
    }

    /// The reason after the time limit stopped the OnRun trigger of codeunit `codeunit` (issue 9213): it names the
    /// trigger, not a test that did not cause the timeout (the codeunit's rows all carry the OnRun reason).
    pub(crate) fn after_on_run_cut(&self, codeunit: u32) -> Option<String> {
        (self.timeout).then(|| self.timed_out(&format!("the trigger Codeunit{codeunit}.OnRun")))
    }

    fn timed_out(&self, what: &str) -> String {
        format!(
            "{what} was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of {} after a timeout",
            self.later()
        )
    }

    /// The later entries the stop ends: every one, or those of its group.
    fn later(&self) -> String {
        match &self.scope {
            StopScope::Request => "a later entry".to_string(),
            StopScope::Group(g) => format!("a later entry of its stopGroup `{g}`"),
        }
    }
}

/// What stops the whole request under the stops that are on (`failure`: `RunOptions::stop_after_first_failure`,
/// `timeout`: `RunOptions::stop_after_timeout`), for serve's refusal of a mix of grouped and ungrouped entries
/// (issues 8210, 9213). With neither on it is issue 8210's word: that refusal comes before the check of the stop.
pub fn stop_triggers(failure: bool, timeout: bool) -> &'static str {
    match (failure, timeout) {
        (true, true) => "a failure or a timeout",
        (false, true) => "a timeout",
        _ => "a failure",
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
                .after(&Outcome::Fail("x".into()), false, 50110, "FailsFirst")
                .as_deref(),
            Some(
                "the test Codeunit50110.FailsFirst failed before it, and stopAfterFirstFailure (--stop-after-first-failure) runs no test of its codeunit or of a later entry after a failure"
            )
        );
        let grouped = Stop::of(true, false, Some("m1")).unwrap();
        assert!(
            grouped
                .after(&Outcome::Fail("x".into()), false, 1, "T")
                .is_some_and(|m| m.ends_with("of a later entry of its stopGroup `m1` after a failure"))
        );
        assert_eq!(request.after(&Outcome::Pass, false, 1, "T"), None);
        assert_eq!(Stop::of(false, false, Some("m1")), None, "no stop, no scope");
    }

    #[test]
    fn the_triggers_name_the_stops_that_are_on() {
        assert_eq!(stop_triggers(true, false), "a failure");
        assert_eq!(stop_triggers(false, true), "a timeout");
        assert_eq!(stop_triggers(true, true), "a failure or a timeout");
        assert_eq!(
            stop_triggers(false, false),
            "a failure",
            "issue 8210's text when no stop is on"
        );
    }

    /// A time-limit row as `run::time_limit_reason` words it.
    fn time_limit_row() -> Outcome {
        Outcome::NotRun(
            "the test used more than the time limit of 1 s of wall time and was stopped (--test-timeout-secs); BC may have ended it differently or not at all".into(),
        )
    }

    /// Issue 9213: a test that hit the time limit stops, named; an OnRun cut names `Codeunit<id>.OnRun`.
    #[test]
    fn a_timeout_stops_naming_the_test_or_the_onrun_trigger() {
        let stop = Stop::of(false, true, None).unwrap();
        assert_eq!(
            stop.after(&time_limit_row(), true, 50720, "B_TimesOut").as_deref(),
            Some(
                "the test Codeunit50720.B_TimesOut was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of a later entry after a timeout"
            )
        );
        assert_eq!(
            stop.after_on_run_cut(50723).as_deref(),
            Some(
                "the trigger Codeunit50723.OnRun was stopped at the time limit before it, and stopAfterTimeout (--stop-after-timeout) runs no test of its codeunit or of a later entry after a timeout"
            )
        );
        let grouped = Stop::of(false, true, Some("m1")).unwrap();
        assert!(
            grouped
                .after(&time_limit_row(), true, 1, "T")
                .is_some_and(|m| m.ends_with("of a later entry of its stopGroup `m1` after a timeout"))
        );
        // Each trigger alone keeps its behavior.
        assert_eq!(stop.after(&Outcome::Fail("x".into()), false, 1, "T"), None);
        let failure = Stop::of(true, false, None).unwrap();
        assert_eq!(failure.after(&time_limit_row(), true, 1, "T"), None);
        assert_eq!(failure.after_on_run_cut(1), None);
    }

    /// Issue 9213 (review of the pull request): the trigger is the fact before the trust judging. A timeout judged not
    /// trusted still stops; a failure judged not trusted (a `NotRun` row) does not stop the failure stop.
    #[test]
    fn the_timeout_trigger_is_the_row_before_the_judging() {
        let both = Stop::of(true, true, None).unwrap();
        let untrusted =
            Outcome::NotRun("an earlier test ...; this timeout is not trusted: the test used more ...".into());
        assert!(
            both.after(&untrusted, true, 1, "T")
                .is_some_and(|m| m.contains("stopAfterTimeout"))
        );
        let untrusted_failure = Outcome::NotRun("an earlier test ...; this failure is not trusted: boom".into());
        assert_eq!(both.after(&untrusted_failure, false, 1, "T"), None);
    }

    /// Issue 9213: never a refusal or a row a stop left out. No stop text reads as a time-limit row, or the outputs would
    /// turn a left-out row into a timeout row.
    #[test]
    fn a_refusal_or_a_stopped_row_does_not_stop() {
        let both = Stop::of(true, true, None).unwrap();
        let texts = [
            both.after(&time_limit_row(), true, 1, "T").unwrap(),
            both.after_on_run_cut(1).unwrap(),
            both.after(&Outcome::Fail("x".into()), false, 1, "T").unwrap(),
            skipped_after_timeout("T"),
            "needs a .NET host; none is configured".to_string(),
        ];
        for t in texts {
            assert!(!super::super::run::is_time_limit(&t), "{t}");
            assert_eq!(both.after(&Outcome::NotRun(t.clone()), false, 1, "U"), None, "{t}");
        }
    }
}
