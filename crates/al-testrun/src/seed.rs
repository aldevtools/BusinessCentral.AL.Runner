//! The run seed of a test run (ADR-061): the seed itself, where it came from, and whether it also seeds
//! `Library - Random`. The derivations are `al_host::seed`'s; the CLI picks the seed (ADR-006).

/// Where the run seed came from (`seedSource` in `--output-json` and `serve`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeedSource {
    /// `--seed N` (or `seed` of a `serve` request, or the id of `--stability-replay`).
    Option,
    /// `ALRUNNER_SEED` or `AL_RUNNER_SEED`.
    Env,
    /// Picked for this run.
    Fresh,
    /// The fixed seed of `al-test-cli` and the corpus ratchet (ADR-061 section 5).
    #[default]
    Fixed,
}

impl SeedSource {
    pub fn text(self) -> &'static str {
        match self {
            SeedSource::Option => "option",
            SeedSource::Env => "env",
            SeedSource::Fresh => "fresh",
            SeedSource::Fixed => "fixed",
        }
    }
}

/// The run seed of a run (`RunOptions::run_seed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSeed {
    pub seed: i32,
    pub source: SeedSource,
    /// `--seed-library`, or a stability run: before each test method the runner calls `Library - Random`.SetSeed with
    /// `al_host::seed::derive_library` (after the reset step's SetSeed(1)), unless a variant gives its own seed.
    pub library: bool,
}

impl Default for RunSeed {
    fn default() -> RunSeed {
        RunSeed {
            seed: al_host::seed::FIXED_RUN_SEED,
            source: SeedSource::Fixed,
            library: false,
        }
    }
}

impl RunSeed {
    pub fn new(seed: i32, source: SeedSource) -> RunSeed {
        RunSeed {
            seed,
            source,
            library: false,
        }
    }
}
