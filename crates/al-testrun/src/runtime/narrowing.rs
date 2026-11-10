//! Whether the outcomes of a run on a compile made for a selection are the full run's (al-runner issue 6924).
//!
//! A selection (`--explain`, `--codeunit`, `--test`) compiles less than the full run: the test codeunits it does not
//! keep (`al_sema::check::Roots::only_tests`), and with a lean compile the workspace objects no selected test names
//! (`Roots::lean`). That is only safe while nothing that runs needs what was left out. Static rules cannot see every way
//! in: install code may hand an id to a dependency app that runs the test codeunit of that id, as Microsoft's Test Runner
//! app does with the ids in its tables. So the run decides: when a test, or install code that runs before the tests
//! (`install::touched_or_all`), met a routine the compile did not compile, the selection runs again on a compile that
//! leaves less out. This module is the one owner of that ladder; `CorpusRuntime::try_run_suites_inner` follows it.

use super::{CorpusRuntime, RunOptions};
use crate::Outcome;

/// How much a compile for a selection leaves out, from most to nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Narrowing {
    /// Lean: the unselected test codeunits and the workspace objects no selected test names (`Roots::lean`).
    Lean,
    /// The unselected test codeunits (`Roots::only_tests`).
    Selection,
    /// What the full run compiles: every test codeunit is a root. The selected tests still run alone.
    Full,
}

/// What the tests of a run met that the compile did not compile.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Met {
    /// A routine of an object a lean compile left out (`al_sema::check::LEFT_OUT`).
    pub(crate) left_out: bool,
    /// A routine nothing reached (`al_sema::check::NOT_REACHED`): in a compile for a selection, perhaps a test codeunit
    /// the selection left out.
    pub(crate) not_reached: bool,
}

impl Met {
    /// What the outcomes of `rt` met: a test refused or failed for such a routine, before it started (its static calls),
    /// while it ran (also inside the test runner's codeunits), or because install code before it stopped at one.
    pub(crate) fn of(rt: &CorpusRuntime) -> Met {
        let mut met = Met::default();
        for r in &rt.results {
            if let Outcome::NotRun(why) | Outcome::Fail(why) = &r.outcome {
                met.left_out |= why.contains(al_sema::check::LEFT_OUT);
                met.not_reached |= why.contains(al_sema::check::NOT_REACHED);
            }
        }
        met
    }

    pub(crate) fn or(self, other: Met) -> Met {
        Met {
            left_out: self.left_out || other.left_out,
            not_reached: self.not_reached || other.not_reached,
        }
    }
}

impl Narrowing {
    /// The narrowing of a compile: `narrowed` is `Lowered::narrowed` (it left workspace objects out), `lean` whether it
    /// is lean.
    pub(crate) fn of(narrowed: bool, lean: bool) -> Narrowing {
        match (narrowed, lean) {
            (false, _) => Narrowing::Full,
            (true, true) => Narrowing::Lean,
            (true, false) => Narrowing::Selection,
        }
    }

    /// The compile to run the selection on again after a run on this one met `met`; `None` when its outcomes stand. A
    /// lean run that met only what the lean compile left out goes to the compile for the selection; anything that met a
    /// routine nothing reached goes to the full run's compile, which compiles every test codeunit.
    pub(crate) fn next(self, met: Met) -> Option<Narrowing> {
        match self {
            Narrowing::Full => None,
            Narrowing::Lean if met.not_reached => Some(Narrowing::Full),
            Narrowing::Lean if met.left_out => Some(Narrowing::Selection),
            Narrowing::Selection if met.not_reached || met.left_out => Some(Narrowing::Full),
            Narrowing::Lean | Narrowing::Selection => None,
        }
    }

    /// The options a compile at this narrowing is made with: the full run's compile has no selection (the run over it
    /// keeps `opts.only`).
    pub(crate) fn compile_options(self, opts: &RunOptions) -> std::borrow::Cow<'_, RunOptions> {
        match self {
            Narrowing::Full if opts.only.is_some() => std::borrow::Cow::Owned(RunOptions {
                only: None,
                ..opts.clone()
            }),
            _ => std::borrow::Cow::Borrowed(opts),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn met(left_out: bool, not_reached: bool) -> Met {
        Met { left_out, not_reached }
    }

    #[test]
    fn the_ladder_goes_to_less_left_out_and_ends_at_the_full_compile() {
        assert_eq!(Narrowing::Lean.next(met(false, false)), None);
        assert_eq!(Narrowing::Lean.next(met(true, false)), Some(Narrowing::Selection));
        assert_eq!(Narrowing::Lean.next(met(true, true)), Some(Narrowing::Full));
        assert_eq!(Narrowing::Selection.next(met(false, false)), None);
        assert_eq!(Narrowing::Selection.next(met(false, true)), Some(Narrowing::Full));
        assert_eq!(Narrowing::Selection.next(met(true, false)), Some(Narrowing::Full));
        assert_eq!(Narrowing::Full.next(met(true, true)), None);
        assert_eq!(Narrowing::of(false, true), Narrowing::Full);
    }
}
