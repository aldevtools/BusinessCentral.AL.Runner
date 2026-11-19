//! `stopAtFirstFailure` of `al-runner serve` (upstream issue 8210): a stop that a failed test trips, after which no
//! further test starts and the tests not reached are not run, with the stop's reason. A running test finishes. The
//! one owner of which outcome trips it and of its two reason texts.

use crate::Outcome;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What a tripped stop ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopScope {
    /// Every later test of the request.
    Request,
    /// Every later test of the entries of the failing one's group.
    Group,
}

/// Which tests' failures trip a stop: codeunit id and test name.
pub type Counts = Arc<dyn Fn(u32, &str) -> bool + Send + Sync>;

/// The reason of the tests not run once a `Request` stop tripped.
pub const REASON_REQUEST: &str =
    "an earlier test of this request failed, and stopAtFirstFailure does not run the rest of the request";
/// The reason of the tests not run once a `Group` stop tripped.
pub const REASON_GROUP: &str =
    "an earlier test of this entry's group failed, and stopAtFirstFailure does not run the rest of the group";

/// One stop; its clones and the handles `counting` and `listening` make share its one flag.
#[derive(Clone)]
pub struct FailureStop {
    tripped: Arc<AtomicBool>,
    scope: StopScope,
    /// `None`: every test's failure trips it.
    counts: Option<Counts>,
    /// Whether this handle's own observations may trip the stop (`listening` clears it).
    may_trip: bool,
}

impl std::fmt::Debug for FailureStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailureStop")
            .field("tripped", &self.is_set())
            .field("scope", &self.scope)
            .field("counts", &self.counts.is_some())
            .field("may_trip", &self.may_trip)
            .finish()
    }
}

impl FailureStop {
    pub fn new(scope: StopScope) -> FailureStop {
        FailureStop {
            tripped: Arc::new(AtomicBool::new(false)),
            scope,
            counts: None,
            may_trip: true,
        }
    }

    /// The same stop (one flag), tripped only by the tests `counts` picks.
    pub fn counting(&self, counts: Counts) -> FailureStop {
        FailureStop {
            counts: Some(counts),
            ..self.clone()
        }
    }

    /// The same stop, never tripped through this handle (a unit whose result is not final).
    pub(crate) fn listening(&self) -> FailureStop {
        FailureStop {
            may_trip: false,
            ..self.clone()
        }
    }

    /// Whether the stop tripped.
    pub fn is_set(&self) -> bool {
        self.tripped.load(Ordering::SeqCst)
    }

    /// The reason of the tests not run once the stop tripped.
    pub fn reason(&self) -> &'static str {
        match self.scope {
            StopScope::Request => REASON_REQUEST,
            StopScope::Group => REASON_GROUP,
        }
    }

    /// The one rule of which outcome trips a stop: a failed test (a `test` line with status `fail`). A test that is not
    /// run (a refusal, a time limit, a runtime panic) does not.
    pub fn trips(outcome: &Outcome) -> bool {
        matches!(outcome, Outcome::Fail(_))
    }

    /// Hears the outcome of test `test` of codeunit `codeunit` as soon as it is known (`events::report`).
    pub(crate) fn observe(&self, codeunit: u32, test: &str, outcome: &Outcome) {
        if self.may_trip && Self::trips(outcome) && self.counts.as_ref().is_none_or(|c| c(codeunit, test)) {
            self.tripped.store(true, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_failure_trips() {
        assert!(FailureStop::trips(&Outcome::Fail("x".into())));
        assert!(!FailureStop::trips(&Outcome::Pass));
        assert!(!FailureStop::trips(&Outcome::NotRun("refused".into())));
    }

    #[test]
    fn a_failure_trips_the_stop_and_every_handle_sees_it() {
        let stop = FailureStop::new(StopScope::Request);
        let counted = stop.counting(Arc::new(|_, _| true));
        let listener = stop.listening();
        counted.observe(1, "T", &Outcome::Pass);
        counted.observe(1, "T", &Outcome::NotRun("refused".into()));
        assert!(!stop.is_set());
        counted.observe(1, "T", &Outcome::Fail("x".into()));
        assert!(stop.is_set() && counted.is_set() && listener.is_set());
    }

    #[test]
    fn a_listening_handle_never_trips_the_stop() {
        let stop = FailureStop::new(StopScope::Group);
        stop.listening().observe(1, "T", &Outcome::Fail("x".into()));
        assert!(!stop.is_set());
    }

    #[test]
    fn a_failure_counting_does_not_pick_does_not_trip() {
        let stop = FailureStop::new(StopScope::Request);
        let counted = stop.counting(Arc::new(|cu, t: &str| cu == 2 && t == "Picked"));
        counted.observe(1, "Picked", &Outcome::Fail("x".into()));
        counted.observe(2, "Other", &Outcome::Fail("x".into()));
        assert!(!stop.is_set());
        counted.observe(2, "Picked", &Outcome::Fail("x".into()));
        assert!(stop.is_set());
    }

    #[test]
    fn the_reason_is_the_scope_s() {
        assert_eq!(FailureStop::new(StopScope::Request).reason(), REASON_REQUEST);
        assert_eq!(FailureStop::new(StopScope::Group).reason(), REASON_GROUP);
    }
}
