//! The refusals of the query cache's Truncate gate (al-runner issue 9097; `al-vm` query_cache.rs `calc_fields_truncate_reader` and
//! the commit tuple in `note_commit_for_query_cache`). The AL is the neighbor app's
//! (tests/neighbors/1555-cache-row-commits-review-r2), one copy: each test runs one shape the Truncate cells of
//! BgTaskCacheEdges.al (:958, :1178, :1184, :1190) do not measure and checks that the runner refuses it. Delete this file when
//! the pin is in the corpus.

use crate::outside_fb_8637::outcomes;
use al_test_cli::report::Outcome;

const SRC: &str = include_str!("../../../../tests/neighbors/1555-cache-row-commits-review-r2/src/TruncateNeighbors.al");
const REASON: &str = "a CalcFields of a page background task after a commit of a Truncate of its row's table";

fn refused(test: &str) {
    let o = outcomes(SRC, "BCE Review R2 Tests");
    let (_, r, marks) = o.iter().find(|(n, _, _)| n == test).expect("the test");
    assert!(matches!(r, Outcome::NotRun(w) if w.contains(REASON)), "{test}: {r:?}");
    assert!(marks.is_empty(), "{test}: {marks:?}");
}

#[test]
fn calc_fields_of_a_record_loaded_by_get_is_refused() {
    refused("CalcFieldsRecordLoadedByGetRefused");
}

#[test]
fn calc_fields_of_a_temporary_record_loaded_by_get_is_refused() {
    refused("TemporaryCalcFieldsRecordLoadedByGetRefused");
}

#[test]
fn calc_fields_of_a_temporary_record_after_its_own_insert_is_refused() {
    refused("TemporaryCalcFieldsAfterOwnInsertRefused");
}

#[test]
fn marked_only_truncate_is_refused() {
    refused("MarkedOnlyTruncateRefused");
}

#[test]
fn truncate_true_is_refused() {
    refused("TruncateResetRefused");
}

#[test]
fn truncate_with_an_explicit_argument_is_refused() {
    refused("ExplicitFalseTruncateRefused");
}
