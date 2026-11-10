//! What a chain codeunit that was not run as a whole leaves for the later codeunits (`chain_walk.rs`): a codeunit refused
//! before it started (`Link::Refused`), or one that stopped before its tests ran (refused at the company open or the
//! login, in or before its OnRun trigger, a failed OnRun). BC ran it, so what it left there is not known; a later
//! codeunit that can read what it can leave is not run, and every other one runs as if it had left nothing.
//!
//! What it can leave is what the run path would carry had it run (`run.rs`, `numbers_taken.rs`):
//!
//! - Its OnRun trigger's writes, under every isolation: runner 130450 runs OnRun outside the codeunit isolation
//!   (codeunits/run-onrun/RqIsolationModes.al:357-369, the `#else` branches the BC 27 legs run), and Disabled carries
//!   it with the rest. So do the isolated storage and number sequences its reach changes (testlibraries
//!   test-framework/onrun-storage/Y35OnRunStorage.al C301_B1_OnRunCommittedStorage, C301_B1_OnRunSequences).
//! - Under Disabled isolation its tests' writes too (QuCommitIsolation.al:695-718), and the session state they change.
//! - Under Codeunit isolation (runner 130450) not its tests' rows, which are rolled back with the codeunit
//!   (RqIsolationModes.al:396-472), but the numbers they took from an older number sequence (Y35OnRunStorage.al:30-31,
//!   C302_B2_TestWritesStorage :457), counted through the tests' own calls: a later codeunit that reaches number
//!   sequences or isolated storage is not run.
//! - Not what the session wrote when it opened the company (the Company Triggers subscribers of the apps under test and
//!   the login, `install::test_session_company_open`, `login.rs`): the run path commits it before it starts counting
//!   what the codeunit leaves, and every later codeunit's session opens the company on its own. So a codeunit refused
//!   inside Microsoft's login code (the BC 27.5 corpus run of LgnLoginChecklistTests.al, al-runner issue 7343) holds
//!   back nothing for its login. (Counting what the login subscribers write through their own calls, Guided
//!   Experience, Translation and more, held back about 2,100 BC 27.5 corpus cases that pass on BC.)
//!
//! The writes are those of the routine's static reach. Where that reach is not fully known or writes a table not known
//! statically, every table the routine and its calls name counts as well; its session state counts only through what it
//! is known to change, as for the chain's members (`chain.rs` `onrun_writes`). A codeunit that can leave nothing of
//! these leaves the state it was given.

use super::blocked::tables_touched;
use super::run::Carried;
use super::written::{ReachIndex, call_reach_facts, reach_facts, state};
use al_sema::lower::Lowered;
use std::collections::BTreeSet;
use std::sync::Arc;

/// What chain codeunit `oi`, which was not run (`why`), leaves for the later codeunits. `onrun_only`: it runs under
/// Codeunit or Function isolation, so its tests' rows do not carry.
pub(super) fn not_run_link(
    engine: &Arc<al_vm::Engine>,
    lowered: &Lowered,
    oi: usize,
    onrun_only: bool,
    why: String,
) -> Carried {
    let o = &lowered.program.objects[oi];
    let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
    let on_run = o
        .routines
        .iter()
        .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"));
    let tests: Vec<usize> = (0..o.routines.len())
        .filter(|&r| {
            o.routines[r]
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("Test"))
        })
        .collect();
    let roots: Vec<usize> = if onrun_only {
        on_run.into_iter().collect()
    } else {
        on_run.into_iter().chain(tests.iter().copied()).collect()
    };
    let mut tables = BTreeSet::new();
    let mut storage = false;
    for r in roots {
        let f = reach_facts(&engine.events, lowered, &index, oi, r);
        tables.extend(f.writes.iter().copied());
        if f.unknown.is_some() || f.writes_any {
            tables.extend(tables_touched(lowered, oi, r));
        }
        storage |= f.state_writes & (state::ISOLATED_STORAGE | state::NUMBER_SEQUENCE) != 0;
    }
    // The numbers its tests took: through the tests' own calls ([`call_reach_facts`]). The event fan-out of a walk
    // reaches a number sequence from almost any record of Microsoft's apps, and so does almost every later codeunit's
    // reach (`corpus.rs` `reaches_session_storage`): counting it would hold back nearly the whole run for a codeunit
    // whose tests take no number (the BC 27.5 corpus run, LgnLoginChecklistTests.al), as `login.rs` does for the login.
    if onrun_only {
        storage |= tests.iter().any(|&r| {
            call_reach_facts(&engine.events, lowered, &index, oi, r).state_writes & state::NUMBER_SEQUENCE != 0
        });
    }
    if tables.is_empty() && !storage {
        return Carried::nothing();
    }
    let who = if onrun_only {
        format!(
            "an earlier test codeunit, {}, whose OnRun trigger or tests can leave state this test reads",
            o.name
        )
    } else {
        format!(
            "an earlier codeunit with RequiredTestIsolation = Disabled, {}, can write a table this test reads",
            o.name
        )
    };
    let reason = format!(
        "{who}, and it was not run ({}): what BC's run of it left is not known",
        why.chars().take(160).collect::<String>()
    );
    Carried {
        baseline: None,
        tables,
        unfinished: Some(reason.clone()),
        everything: storage,
        storage_why: storage.then_some(reason),
        as_started: false,
    }
}
