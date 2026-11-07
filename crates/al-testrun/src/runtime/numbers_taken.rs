//! What a test codeunit under runner 130450 leaves for the later codeunits besides what its OnRun trigger wrote: the numbers
//! its tests took from older number sequences. The codeunit's rollback does not give them back, while it drops the tests'
//! new sequences and their isolated storage writes (testlibraries test-framework/onrun-storage/Y35OnRunStorage.al:30-31,
//! C302_B2_TestWritesStorage :447-457).

use super::run::{CARRIED, Carried};

/// What the `OnRun` trigger of a codeunit under runner 130450 left, right after it ran (`run.rs`): `start` is the state
/// captured before it, `baseline` the one the codeunit's session started from, and `changes_from` the store's change
/// counter before OnRun. OnRun ran outside the codeunit isolation: the later codeunits start from what it left, committed
/// and open, whatever the tests do (codeunits/run-onrun/RqIsolationModes.al:357-369 #else).
pub(super) fn onrun_left(
    session: &mut al_vm::Session,
    start: &al_vm::Baseline,
    baseline: Option<&al_vm::Baseline>,
    skipped_installs: &[(std::collections::BTreeSet<u32>, String)],
    changes_from: u64,
) -> Carried {
    let everything = session.storage_or_sequences_differ(start);
    let as_started = baseline.is_some_and(|b| {
        b.workspace_installs.as_deref() == Some(skipped_installs) && session.holds_what_it_started_from(b)
    });
    let mut left = session.capture_baseline();
    left.workspace_installs = Some(skipped_installs.to_vec());
    // The rows OnRun wrote are in the session company only.
    left.install_rows = Some(session.install_rows());
    Carried {
        baseline: Some(left),
        tables: session.store_mut().tables_changed_since(changes_from),
        unfinished: None,
        // Isolated storage and number sequences carry as the rows do, committed and open
        // (testlibraries test-framework/onrun-storage/Y35OnRunStorage.al C301_B1_OnRunCommittedStorage,
        // C301_B1_OnRunSequences, C301_B1_OnRunOpenWrites): a later codeunit whose reach can touch them
        // starts from this baseline (`corpus.rs`, `Carried::everything`).
        storage_why: None,
        everything,
        as_started,
    }
}

/// Sets [`CARRIED`] to what the codeunit leaves ([`left_after_tests`]), when it leaves something known.
pub(super) fn carry_left(
    session: &al_vm::Session,
    obj: &al_ir::Object,
    onrun_left: Option<Carried>,
    start: Option<al_vm::Baseline>,
    skipped_installs: &[(std::collections::BTreeSet<u32>, String)],
) {
    if let Some(left) = left_after_tests(session, obj, onrun_left, start, skipped_installs) {
        CARRIED.with(|c| *c.borrow_mut() = Some(left));
    }
}

/// What the codeunit leaves: `onrun_left`, or for a codeunit without an OnRun trigger the state it started from (`start`),
/// with the numbers its tests took. `None`: it leaves nothing known (its OnRun did not run to its end).
fn left_after_tests(
    session: &al_vm::Session,
    obj: &al_ir::Object,
    onrun_left: Option<Carried>,
    start: Option<al_vm::Baseline>,
    skipped_installs: &[(std::collections::BTreeSet<u32>, String)],
) -> Option<Carried> {
    let has_onrun = obj
        .routines
        .iter()
        .any(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"));
    let onrun_left_was_none = onrun_left.is_none();
    let mut left = match onrun_left {
        Some(left) => left,
        None if !has_onrun => {
            // The install code already ran for this state, as for what an OnRun left (`run.rs`).
            let mut baseline = start?;
            baseline.workspace_installs = Some(skipped_installs.to_vec());
            baseline.install_rows = Some(session.install_rows());
            Carried {
                baseline: Some(baseline),
                tables: Default::default(),
                unfinished: None,
                storage_why: None,
                everything: false,
                as_started: false,
            }
        }
        None => return None,
    };
    let Some(b) = left.baseline.as_mut() else {
        return Some(left);
    };
    match session.carry_numbers_taken(b) {
        // No number taken: a codeunit without an OnRun trigger leaves the state it was given (`baseline: None`, which the
        // chain fills with that state, `corpus.rs`), not its session's. A codeunit the baseline rule check runs on an
        // empty store would otherwise hand that store to the later codeunits.
        None if onrun_left_was_none => left.baseline = None,
        // An OnRun that left exactly the state the codeunit started from (`Carried::as_started`), and no number taken:
        // the codeunit leaves the state it was given, as one without an OnRun trigger does. A session started from it
        // starts as one started from what OnRun left (`Session::holds_what_it_started_from`), so the later codeunits see
        // the same; the carry chain can then tell that it left nothing (`chain.rs`).
        None if left.as_started => left.baseline = None,
        None => {}
        Some(true) => left.everything = true,
        Some(false) => {
            left.everything = true;
            left.storage_why.get_or_insert_with(|| {
                format!(
                    "an earlier codeunit under runner 130450, {}, deleted a number sequence it had taken numbers from \
                     or restarted: what the later codeunits see of it is not pinned",
                    obj.name
                )
            });
        }
    }
    Some(left)
}
