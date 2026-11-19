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
    /// A handle whose own observations never trip the stop.
    listens_only: bool,
}

impl std::fmt::Debug for FailureStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FailureStop")
            .field("tripped", &self.is_set())
            .field("scope", &self.scope)
            .field("counts", &self.counts.is_some())
            .field("listens_only", &self.listens_only)
            .finish()
    }
}

impl FailureStop {
    pub fn new(scope: StopScope) -> FailureStop {
        FailureStop {
            tripped: Arc::new(AtomicBool::new(false)),
            scope,
            counts: None,
            listens_only: false,
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
            listens_only: true,
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

    /// The one rule of which outcome trips a stop.
    pub fn trips(_outcome: &Outcome) -> bool {
        false
    }

    /// Hears the outcome of test `test` of codeunit `codeunit` as soon as it is known.
    pub(crate) fn observe(&self, _codeunit: u32, _test: &str, _outcome: &Outcome) {}
}
