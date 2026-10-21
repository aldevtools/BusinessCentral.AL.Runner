//! Running test codeunits and reading the result: the part of the corpus harness that a user
//! command line shares with it (ADR-016 section 2). Loads apps, resolves dependency apps,
//! compiles, runs each test codeunit on a session of its own, and keeps one outcome per test.
//! The oracle, the pass-rate report and the ratchet stay in `al-test-cli`.

pub mod affected;
pub mod baseline;
pub mod baseline_cache;
pub mod cache_gc;
pub mod capture;
pub mod compile_cache;
pub mod coverage;
mod cpuclock;
pub mod crash;
pub mod deps;
pub mod disabled;
pub mod discover;
pub mod dotnet;
pub mod events;
pub mod explain;
pub mod flagged;
pub mod host;
pub mod impact_static;
pub mod impact_trace;
pub mod install;
pub mod isolation;
pub mod junit;
pub mod observe;
pub mod reach;
pub mod refusals;
pub mod runtime;
pub mod target;

/// What one test did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    /// The runtime ran the case and it failed (assertion or runtime error).
    Fail(String),
    /// The runtime could not run the case: compile-time unsupported, or no runtime yet.
    NotRun(String),
}

/// A test: its codeunit id and procedure name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaseId {
    pub codeunit: u32,
    pub name: String,
}
pub use al_builtins::assumptions;
pub use al_builtins::native::UNTRACKED_CALL_STACK;
