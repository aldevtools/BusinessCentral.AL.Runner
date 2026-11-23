//! Behavior that differs between BC versions in an open corpus probe, before a corpus bump pins it. Not a [`crate::Gate`]: a gate
//! needs its `#if BCnnPLUS` guard in the pinned corpus (`tests/gate_completeness.rs`). When the bump pins the probe's file, that
//! test fails until the function here becomes a gate. A major that the probe did not measure gets `None` (or `false` where the
//! function answers whether a major is measured); no target means the newest behavior, as [`crate::has`] does, or `false` where the
//! newest major is not measured.
//!
//! Behavior that differs between BC versions in an open corpus probe, before a bump pins its guard
//! (not a `Gate`; `tests/gate_completeness.rs` fails at the bump until it becomes one). Entry points:
//! `journal_temp_part_reads_after_refresh`.

use crate::Target;

/// Whether a temporary part that a Visible expression hid, on the Worksheet or List host of the journal shape, reads its rows
/// once more after the host's refresh that follows an action, and runs OnAfterGetCurrRecord on its blank row when a host move
/// emptied it and left it hidden (testpage/parts/HiddenPartJournalShape.al, the `#if BC29PLUS` guards of :540, :574, :608,
/// :642 and :677): `Some(false)` on 27 and 28, `Some(true)` on 29.
pub fn journal_temp_part_reads_after_refresh(target: Option<&Target>) -> Option<bool> {
    match target.map(|t| t.bc.major) {
        Some(27 | 28) => Some(false),
        Some(29) | None => Some(true),
        Some(_) => None,
    }
}

/// Whether the test session under `al-runner test --no-client` (a web-service session: Microsoft's test runner called over
/// OData) is measured on the target's major: the reporter's cells of al-runner issues 8154, 8158 and 8028 ran on BC 28.4, and
/// the probe that measures them on each major (open corpus PR 1420, tests/al-language-webservice-probe/src/WsSessionDialogs.al)
/// cannot run in corpus CI. `true` on 28 only; on every other major, and with no target, the answers of that session are not
/// measured (assumption `no-client-session`, al-vm `clientless.rs`).
pub fn web_service_session_measured(target: Option<&Target>) -> bool {
    matches!(target.map(|t| t.bc.major), Some(28))
}

/// What a TestPage does with a Card's field control bound to no field of the source table whose own Visible names a field of
/// the record (`ir::Control::visible_row_field`; open corpus PR 1504, tests/al-language/pages/controls/VisibleExpressionSource.al,
/// al-runner issue 8660): `Some(true)` on 29, where its Visible reads Yes whatever the row holds; `Some(false)` on 27 and 28,
/// where a TestPage's OpenEdit, OpenView or OpenNew of the page raises "The identifier '<field>' could not be found."
/// (the probe's `#if not BC29PLUS` tests); `None` on a major the probe did not measure.
pub fn visible_names_row_field_reads_yes(target: Option<&Target>) -> Option<bool> {
    match target.map(|t| t.bc.major) {
        Some(27 | 28) => Some(false),
        Some(29) | None => Some(true),
        Some(_) => None,
    }
}

/// A bulk write of a table that a commit carried (al-runner issue 8467, [`bulk_commit_ends_cached_row`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BulkWrite {
    ModifyAll,
    DeleteAll,
    Truncate,
}

/// Whether a commit of a `kind` write of another row of a table ends the query cache entry of a row a page background task read,
/// when the test then changes the row without reading it (al-runner issue 8467, open corpus PR 1495, tests/al-language/pages/
/// background-tasks/BgTaskQueryCache.al:7035 ModifyAll, :7042 DeleteAll; the cells tell the minors apart at run time, which the
/// preprocessor cannot): `Some(true)` on 27.5, 28.5 and 29.0 (the corpus CI), `Some(false)` on 29.1 (a local container). A
/// Truncate has no cell (after it, a ModifyAll of the row made the worker's read wait on a lock on the local container): `None`.
/// On every other minor, and with no target, `None`.
pub fn bulk_commit_ends_cached_row(target: Option<&Target>, kind: BulkWrite) -> Option<bool> {
    match (target.map(|t| (t.bc.major, t.bc.minor)), kind) {
        (_, BulkWrite::Truncate) => None,
        (Some((27, 5) | (28, 5) | (29, 0)), _) => Some(true),
        (Some((29, 1)), _) => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(v: &str) -> Target {
        Target::for_bc(crate::BcVersion::parse(v).unwrap())
    }

    #[test]
    fn the_journal_temp_part_read_follows_the_measured_majors() {
        assert_eq!(
            journal_temp_part_reads_after_refresh(Some(&target("27.5"))),
            Some(false)
        );
        assert_eq!(
            journal_temp_part_reads_after_refresh(Some(&target("28.5"))),
            Some(false)
        );
        assert_eq!(journal_temp_part_reads_after_refresh(Some(&target("29.0"))), Some(true));
        assert_eq!(journal_temp_part_reads_after_refresh(None), Some(true));
        assert_eq!(journal_temp_part_reads_after_refresh(Some(&target("30.0"))), None);
    }

    #[test]
    fn the_bulk_commit_of_the_query_cache_follows_the_measured_minors() {
        use BulkWrite::*;
        for kind in [ModifyAll, DeleteAll] {
            for v in ["27.5", "28.5", "29.0"] {
                assert_eq!(
                    bulk_commit_ends_cached_row(Some(&target(v)), kind),
                    Some(true),
                    "{v} {kind:?}"
                );
            }
            assert_eq!(
                bulk_commit_ends_cached_row(Some(&target("29.1")), kind),
                Some(false),
                "{kind:?}"
            );
            for v in ["27.0", "28.4", "29.2", "30.0"] {
                assert_eq!(
                    bulk_commit_ends_cached_row(Some(&target(v)), kind),
                    None,
                    "{v} {kind:?}"
                );
            }
            assert_eq!(bulk_commit_ends_cached_row(None, kind), None);
        }
        for v in ["27.5", "28.5", "29.0", "29.1"] {
            assert_eq!(bulk_commit_ends_cached_row(Some(&target(v)), Truncate), None, "{v}");
        }
    }

    #[test]
    fn the_web_service_session_is_measured_on_28_only() {
        assert!(web_service_session_measured(Some(&target("28.4"))));
        assert!(web_service_session_measured(Some(&target("28.5"))));
        for v in ["27.5", "29.0", "30.0"] {
            assert!(!web_service_session_measured(Some(&target(v))), "{v}");
        }
        assert!(!web_service_session_measured(None));
    }
}
