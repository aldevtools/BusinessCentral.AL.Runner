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
    /// `read`, `built` or `off` (`IndexState::key`): how the open of the test data got the backup's kept index. `None`:
    /// no test data, or a source without an index. Describes the open this process did, so a daemon repeats it.
    pub test_data_index: Option<&'static str>,
}

impl RequestTimings {
    /// The phases of a finished run `p`, with the setup parts and the coverage time the run measured itself, and the
    /// test data it opened with its backup path (`run/test_data.rs` `open_backup`).
    pub fn of(
        setup: SetupParts,
        p: &al_testrun::runtime::Phases,
        coverage: std::time::Duration,
        test_data: Option<&(al_host::testdata::TestData, std::path::PathBuf)>,
    ) -> RequestTimings {
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
            test_data_index: test_data.and_then(|(d, _)| d.provider.index_state()).map(|s| s.key()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn of_copies_each_phase_and_takes_the_warm_compile_over_the_compile_when_it_is_set() {
        let ms = Duration::from_millis;
        let mut p = al_testrun::runtime::Phases {
            compile: ms(5),
            engine: ms(6),
            engine_kept: true,
            baseline: ms(7),
            baseline_source: Some("kept"),
            tests: ms(8),
            ..Default::default()
        };
        let t = RequestTimings::of(SetupParts::default(), &p, ms(9), None);
        assert_eq!(
            (t.compile, t.engine, t.baseline, t.tests, t.coverage),
            (ms(5), ms(6), ms(7), ms(8), ms(9))
        );
        assert!(t.engine_kept);
        assert_eq!(t.baseline_source, Some("kept"));
        assert_eq!(t.test_data_index, None);
        p.warm_compile = Some(ms(1));
        assert_eq!(
            RequestTimings::of(SetupParts::default(), &p, ms(9), None).compile,
            ms(1)
        );
    }
}
