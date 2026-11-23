//! The seam between the harness and the runtime.
//!
//! Entry points: `Runner`, `NoRuntime`.

use crate::oracle::CaseId;
use crate::report::Outcome;

pub trait Runner {
    fn outcome(&self, case: &CaseId) -> Outcome;
}

/// Milestone 0: no runtime exists, so every case is reported as not run.
pub struct NoRuntime;

impl Runner for NoRuntime {
    fn outcome(&self, _case: &CaseId) -> Outcome {
        Outcome::NotRun("runtime not implemented".into())
    }
}

/// The outcomes of the first run (the main corpus app, or the only run).
impl Runner for crate::runtime::CorpusRuntime {
    fn outcome(&self, case: &CaseId) -> Outcome {
        self.first_outcome(case)
    }
}
