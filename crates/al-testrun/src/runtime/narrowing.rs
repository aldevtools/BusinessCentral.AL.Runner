//! Whether the outcomes of a run on a compile made for a selection are the full run's (al-runner issue 6924).
//!
//! A one-shot selection (`--explain`, `--codeunit`, `--test`) compiles less than the full run: the test
//! codeunits it does not keep and the procedures no selected test names (`al_sema::check::UNSELECTED`), and with a lean
//! compile the workspace objects no selected test names (`al_sema::check::LEFT_OUT`). That is only safe while nothing that
//! runs needs what was left out, and static rules cannot see every way in: install code may hand an id to a dependency
//! app that runs the test codeunit of that id, as Microsoft's Test Runner app does with the ids in its tables. So the run
//! decides. When a test, or install code of an app under test (`note_install`, whatever the tests touch), met a routine
//! the compile left out, the selection runs again on a compile that leaves less out, up to the full run's compile, where
//! every test codeunit is a root and only the selected tests run (`settle`). A dependency routine the demand did not
//! reach (`al_sema::check::NOT_REACHED`) does not count: the full run gives the same refusal.
//!
//! This module is the one owner of that decision; `CorpusRuntime::try_run_suites_inner` follows it. `Warm::run` compiles
//! without a selection (al-runner issue 7978), so its compile is `Narrowing::Full` and nothing is redone.

use super::{Compiled, CorpusRuntime, RunOptions};
use crate::Outcome;
use al_vm::Engine;
use std::sync::atomic::{AtomicBool, Ordering};

/// How much a compile for a selection leaves out, from most to nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Narrowing {
    /// Lean: also the workspace objects no selected test names (`Roots::lean`).
    Lean,
    /// What the selection does not keep (`Roots::only_tests`, `Roots::reach_only`).
    Selection,
    /// What the full run compiles: every test codeunit is a root. The selected tests still run alone.
    Full,
}

/// What a run met that its compile left out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Met {
    /// A routine of an object a lean compile left out (`al_sema::check::LEFT_OUT`).
    pub(crate) left_out: bool,
    /// A routine the compile for the selection left out (`al_sema::check::UNSELECTED`).
    pub(crate) unselected: bool,
}

impl Met {
    /// What a refusal reason says was left out.
    pub(crate) fn of_reason(why: &str) -> Met {
        Met {
            left_out: why.contains(al_sema::check::LEFT_OUT),
            unselected: why.contains(al_sema::check::UNSELECTED),
        }
    }

    /// What run `rt` met: a test refused or failed for such a routine, before it started (its static calls) or while it
    /// ran (also inside the test runner's codeunits), and what its install code met (`CorpusRuntime::met`).
    pub(crate) fn of(rt: &CorpusRuntime) -> Met {
        rt.results
            .iter()
            .filter_map(|r| match &r.outcome {
                Outcome::NotRun(why) | Outcome::Fail(why) => Some(Met::of_reason(why)),
                Outcome::Pass => None,
            })
            .fold(rt.met, Met::or)
    }

    pub(crate) fn or(self, other: Met) -> Met {
        Met {
            left_out: self.left_out || other.left_out,
            unselected: self.unselected || other.unselected,
        }
    }
}

/// What the install code of the apps under test met on an engine (`note_install`), until the run takes it (`take_install`).
#[derive(Default)]
struct InstallMet {
    left_out: AtomicBool,
    unselected: AtomicBool,
}

/// Install code of an app under test could not run (`why`). When it met a routine the compile left out, the engine notes
/// it, so the run is redone whatever the selected tests touch: what the install code would have left (rows, number
/// sequences, isolated storage) is not known.
pub(crate) fn note_install(engine: &Engine, why: &str) {
    let met = Met::of_reason(why);
    if met != Met::default() {
        let noted = engine.derived(InstallMet::default);
        noted.left_out.fetch_or(met.left_out, Ordering::Relaxed);
        noted.unselected.fetch_or(met.unselected, Ordering::Relaxed);
    }
}

/// What the install code met on `engines` since the last call, and clears it (a warm server keeps the engines).
pub(crate) fn take_install<'a>(engines: impl IntoIterator<Item = &'a Engine>) -> Met {
    engines.into_iter().fold(Met::default(), |met, e| {
        let noted = e.derived(InstallMet::default);
        met.or(Met {
            left_out: noted.left_out.swap(false, Ordering::Relaxed),
            unselected: noted.unselected.swap(false, Ordering::Relaxed),
        })
    })
}

impl Narrowing {
    /// The narrowing of `compiled`: what `Lowered::narrowed` and the lean compile say.
    pub(crate) fn of(compiled: &Compiled) -> Narrowing {
        match (compiled.lowered.narrowed(), compiled.lean.is_some()) {
            (false, _) => Narrowing::Full,
            (true, true) => Narrowing::Lean,
            (true, false) => Narrowing::Selection,
        }
    }

    /// Whether a run on `compiled` may be redone: it must keep its engines to free them (`process_ends_after` would leak
    /// them while the next compile is made).
    pub(crate) fn may_redo(compiled: &Compiled) -> bool {
        Narrowing::of(compiled) != Narrowing::Full
    }

    /// Whether a run must not be redone: it has effects outside the run that a second run would repeat (a debugger's
    /// breakpoints and its ready call, `al-runner debug`; files in the `--files-dir` directory, which the second run would
    /// find). Such a run compiles as the full run does from the start ([`Narrowing::compile_options`] of `Full`).
    pub(crate) fn live_effects(opts: &RunOptions) -> bool {
        opts.hooks.debugger.is_some() || opts.debug_ready.is_some() || crate::host::uses_real_files()
    }

    /// The compile to run the selection on again after a run on this one met `met`; `None` when its outcomes stand. A
    /// lean run that met only what the lean compile left out goes to the compile for the selection; one that met what the
    /// selection left out goes to the full run's compile.
    pub(crate) fn next(self, met: Met) -> Option<Narrowing> {
        match self {
            Narrowing::Full => None,
            Narrowing::Lean if met.unselected => Some(Narrowing::Full),
            Narrowing::Lean if met.left_out => Some(Narrowing::Selection),
            Narrowing::Selection if met.unselected || met.left_out => Some(Narrowing::Full),
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

/// What [`settle`] ended with.
pub(crate) enum Settled {
    /// The compile the outcomes stand on and the run over it; `redone` when it is not the first compile.
    Ran {
        compiled: Box<Compiled>,
        rt: CorpusRuntime,
        redone: bool,
    },
    /// A hard memory limit stopped a compile (`compile_halt.rs`).
    Stopped(CorpusRuntime),
}

/// Runs the selection again on compiles that leave less out until a run's outcomes stand ([`Narrowing::next`]). `rt` is
/// the run on `compiled`. `compile` makes the compile of a narrowing with its options from the compile before it (whose
/// apps it takes), `run` runs the selection (`opts.only`) on a compile.
pub(crate) fn settle(
    mut compiled: Compiled,
    mut rt: CorpusRuntime,
    opts: &RunOptions,
    mut compile: impl FnMut(Compiled, &RunOptions) -> Result<Result<Compiled, CorpusRuntime>, String>,
    mut run: impl FnMut(&Compiled) -> CorpusRuntime,
) -> Result<Settled, String> {
    let mut narrowing = Narrowing::of(&compiled);
    let started_lean = narrowing == Narrowing::Lean;
    let (mut earlier, mut compiles, mut redone) = (std::time::Duration::ZERO, 0, false);
    while let Some(next) = narrowing.next(Met::of(&rt)) {
        earlier += rt.phases.compile + rt.phases.tests;
        compiles += rt.phases.compiles;
        compiled = match compile(compiled, &next.compile_options(opts))? {
            Ok(c) => c,
            Err(stopped) => return Ok(Settled::Stopped(stopped)),
        };
        rt = run(&compiled);
        narrowing = match next {
            Narrowing::Full => Narrowing::Full,
            _ => Narrowing::of(&compiled),
        };
        rt.phases.full_redone = next == Narrowing::Full;
        redone = true;
    }
    if redone {
        rt.phases.lean_redone = started_lean;
        rt.phases.compile += earlier;
        rt.phases.compiles += compiles;
        rt.phases.work.compiles = rt.phases.compiles;
    }
    Ok(Settled::Ran {
        compiled: Box::new(compiled),
        rt,
        redone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn met(left_out: bool, unselected: bool) -> Met {
        Met { left_out, unselected }
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
    }

    /// A dependency routine the demand did not reach is refused in the full run too: it starts no second compile.
    #[test]
    fn only_what_the_selection_left_out_counts() {
        assert_eq!(Met::of_reason(al_sema::check::NOT_REACHED), Met::default());
        assert_eq!(
            Met::of_reason(&format!(
                "install routine X could not run: {}",
                al_sema::check::UNSELECTED
            )),
            met(false, true)
        );
        assert_eq!(Met::of_reason(al_sema::check::LEFT_OUT), met(true, false));
    }

    #[test]
    fn a_run_with_effects_outside_it_is_not_redone() {
        assert!(!Narrowing::live_effects(&RunOptions::default()));
        let debugged = RunOptions {
            debug_ready: Some(crate::runtime::DebugReadyHook(std::sync::Arc::new(
                |_: &crate::runtime::DebugReady| {},
            ))),
            ..Default::default()
        };
        assert!(Narrowing::live_effects(&debugged));
    }
}
