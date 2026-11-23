//! The parts of a run's setup, the time before the compile (al-runner issue 8205): `--timings` prints them as the
//! `setup:` line, a serve summary sends them as `timings.setupParts` beside `setupMs`, their sum. `run.rs` `test_inner`
//! ends each part with `Laps::lap`; a part reached twice (the test data: the backup named on the command line, then the
//! one chosen once the dependency apps are known) adds up.
//!
//! The parts of a run's setup before the compile (al-runner issue 8205): `Part`, `Laps` (`run.rs`
//! `test_inner` ends each part with `lap`; a part reached twice adds up), `SetupParts` (`total` is
//! serve's `setupMs`, `keyed` its `setupParts`, `line` the `--timings` `setup:` line). A server
//! request's entry runs are built by `run.rs` `entry_runs`. Entry points: `Laps::start`,
//! `Laps::finish`.

use std::time::{Duration, Instant};

/// One part of the setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The run seed, the server address, the output paths.
    Options,
    /// Reading the apps under test: their `app.json` and source files.
    Sources,
    /// Finding the test codeunits, the disabled tests, the test suites, the request's and its entries' selection.
    Selection,
    /// The target build, the company data app and the dependency apps (read, or kept by a server).
    Dependencies,
    /// Opening the test data (a backup or the cached artifact of the build).
    TestData,
    /// The license of the run (a file, the artifact's, or kept by a server).
    License,
    /// The .NET helpers: their configuration, whether the tests look free of data, starting the helpers ahead.
    DotNet,
    /// `--affected` and serve `affectedOnly` (ADR-050): reading the stored result and hashing the run's inputs.
    Affected,
    /// The runs of a server request's entries (`run.rs` `entry_runs`).
    Entries,
    /// The rest before the compile: the dependency notes, `--verbose`'s setup lines, the hooks and the run options.
    Prepare,
}

/// The parts in their order, with the name of each in the `setup:` line and in `setupParts`.
const PARTS: [(Part, &str, &str); 10] = [
    (Part::Options, "options", "optionsMs"),
    (Part::Sources, "sources", "sourcesMs"),
    (Part::Selection, "selection", "selectionMs"),
    (Part::Dependencies, "dependencies", "dependenciesMs"),
    (Part::TestData, "test data", "testDataMs"),
    (Part::License, "license", "licenseMs"),
    (Part::DotNet, "dotnet", "dotnetMs"),
    (Part::Affected, "affected", "affectedMs"),
    (Part::Entries, "entries", "entriesMs"),
    (Part::Prepare, "prepare", "prepareMs"),
];

/// The time of each part.
#[derive(Debug, Clone, Default)]
pub struct SetupParts {
    took: [Duration; PARTS.len()],
}

impl SetupParts {
    fn slot(part: Part) -> usize {
        PARTS
            .iter()
            .position(|(p, _, _)| *p == part)
            .expect("every part is listed")
    }

    #[cfg(test)]
    fn get(&self, part: Part) -> Duration {
        self.took[Self::slot(part)]
    }

    /// The whole setup: `setupMs`, and the `load` of `--verbose`'s `Time:` line.
    pub fn total(&self) -> Duration {
        self.took.iter().sum()
    }

    /// The `--timings` line.
    pub fn line(&self) -> String {
        let parts: Vec<String> = PARTS
            .iter()
            .zip(&self.took)
            .map(|((_, name, _), d)| format!("{name} {:.2} s", d.as_secs_f64()))
            .collect();
        format!("setup: {}", parts.join(", "))
    }

    /// `(key, time)` per part, for a serve summary's `setupParts`.
    pub fn keyed(&self) -> impl Iterator<Item = (&'static str, Duration)> + '_ {
        PARTS.iter().zip(&self.took).map(|((_, _, key), d)| (*key, *d))
    }
}

/// Measures the parts one after the other: each `lap` ends a part where the last one ended.
pub struct Laps {
    last: Instant,
    parts: SetupParts,
}

impl Laps {
    /// The setup starts at `started` (the start of the run).
    pub fn start(started: Instant) -> Self {
        Laps {
            last: started,
            parts: SetupParts::default(),
        }
    }

    /// Ends `part` now: the time since the last lap is added to it.
    pub fn lap(&mut self, part: Part) {
        let now = Instant::now();
        self.parts.took[SetupParts::slot(part)] += now - self.last;
        self.last = now;
    }

    /// Ends the last part, `Prepare`, and gives every part.
    pub fn finish(mut self) -> SetupParts {
        self.lap(Part::Prepare);
        self.parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parts_add_up_to_the_time_since_the_start_and_a_part_reached_twice_adds_up() {
        let started = Instant::now();
        let mut laps = Laps::start(started);
        std::thread::sleep(Duration::from_millis(2));
        laps.lap(Part::TestData);
        std::thread::sleep(Duration::from_millis(2));
        laps.lap(Part::Sources);
        std::thread::sleep(Duration::from_millis(2));
        laps.lap(Part::TestData);
        let parts = laps.finish();
        assert!(parts.total() >= Duration::from_millis(6) && parts.total() <= started.elapsed());
        assert!(parts.get(Part::TestData) >= Duration::from_millis(4));
        assert!(parts.get(Part::Sources) >= Duration::from_millis(2));
        assert_eq!(parts.get(Part::Options), Duration::ZERO);
    }

    #[test]
    fn the_line_and_the_keys_name_every_part_in_order() {
        let parts = SetupParts::default();
        assert_eq!(
            parts.line(),
            "setup: options 0.00 s, sources 0.00 s, selection 0.00 s, dependencies 0.00 s, test data 0.00 s, \
             license 0.00 s, dotnet 0.00 s, affected 0.00 s, entries 0.00 s, prepare 0.00 s"
        );
        let keys: Vec<&str> = parts.keyed().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            [
                "optionsMs",
                "sourcesMs",
                "selectionMs",
                "dependenciesMs",
                "testDataMs",
                "licenseMs",
                "dotnetMs",
                "affectedMs",
                "entriesMs",
                "prepareMs"
            ]
        );
    }
}
