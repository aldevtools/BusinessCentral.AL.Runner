//! What a test codeunit under runner 130450 leaves for the later codeunits besides what its OnRun trigger wrote: the numbers
//! its tests took from older number sequences. The codeunit's rollback does not give them back, while it drops the tests'
//! new sequences and their isolated storage writes (testlibraries lt-y35-onrun-storage/Y35OnRunStorage.al:30-31,
//! C302_B2_TestWritesStorage :447-457).

use super::run::Carried;

/// What the codeunit leaves: `onrun_left`, or for a codeunit without an OnRun trigger the state it started from (`start`),
/// with the numbers its tests took. `None`: it leaves nothing known (its OnRun did not run to its end).
pub(super) fn left_after_tests(
    session: &al_vm::Session,
    obj: &al_ir::Object,
    onrun_left: Option<Carried>,
    start: Option<al_vm::Baseline>,
) -> Option<Carried> {
    let has_onrun = obj
        .routines
        .iter()
        .any(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"));
    let mut left = match onrun_left {
        Some(left) => left,
        None if !has_onrun => Carried {
            baseline: Some(start?),
            tables: Default::default(),
            unfinished: None,
            storage_why: None,
            everything: false,
        },
        None => return None,
    };
    let Some(b) = left.baseline.as_mut() else {
        return Some(left);
    };
    match session.carry_numbers_taken(b) {
        None => {}
        Some(true) => left.everything = true,
        Some(false) => {
            left.everything = true;
            left.storage_why.get_or_insert_with(|| {
                format!(
                    "an earlier codeunit under runner 130450, {}, changed a number sequence other than by taking numbers \
                     (Restart, Delete): what the later codeunits see of it is not pinned",
                    obj.name
                )
            });
        }
    }
    Some(left)
}
