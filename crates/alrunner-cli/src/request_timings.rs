//! Where a server request spent its time, by phase (al-runner issue 5856). `run.rs` `test_inner` fills one per request
//! (`RequestTimings::of`); `serve.rs` `timings_json` sends it as the summary's `timings`.

use crate::setup_timings::SetupParts;

/// Where a server request spent its time, by phase (al-runner issue 5856).
#[derive(Debug, Clone, Default)]
pub struct RequestTimings {
    /// Before the compile, by part (al-runner issue 8205); `SetupParts::total` is `setupMs`.
    pub setup: SetupParts,
    /// Finding the kept compile, updating it, or compiling again.
    pub compile: std::time::Duration,
    /// Building the engine the tests run on from the compiled program, or setting up a kept one.
    pub engine: std::time::Duration,
    /// The request ran on the engines an earlier request over the same compile kept.
    pub engine_kept: bool,
    /// Setting up the install baseline: running the install code, or reading it.
    pub baseline: std::time::Duration,
    /// `ran`, `cache` or `kept` (`Phases::baseline_source`); `None`: no baseline.
    pub baseline_source: Option<&'static str>,
    /// Running the test codeunits, every pass.
    pub tests: std::time::Duration,
    /// Building the coverage document.
    pub coverage: std::time::Duration,
    /// Per pass (one per entry of the request): the threads' baseline copy and the pass's wall time.
    pub passes: Vec<al_testrun::runtime::PassTime>,
}

impl RequestTimings {
    /// The phases of a finished run `p`, with the setup parts and the coverage time the run measured itself.
    pub fn of(setup: SetupParts, p: &al_testrun::runtime::Phases, coverage: std::time::Duration) -> RequestTimings {
        RequestTimings {
            setup,
            compile: p.warm_compile.unwrap_or(p.compile),
            engine: p.engine,
            engine_kept: p.engine_kept,
            baseline: p.baseline,
            baseline_source: p.baseline_source,
            tests: p.tests,
            coverage,
            passes: p.passes.clone(),
        }
    }
}
