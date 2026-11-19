//! Behavior that differs between BC versions in an open corpus probe, before a corpus bump pins it. Not a [`crate::Gate`]: a gate
//! needs its `#if BCnnPLUS` guard in the pinned corpus (`tests/gate_completeness.rs`). When the bump pins the probe's file, that
//! test fails until the function here becomes a gate. A major that the probe did not measure gets `None` (or `false` where the
//! function answers whether a major is measured); no target means the newest behavior, as [`crate::has`] does, or `false` where the
//! newest major is not measured.

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

/// Whether a commit of a ModifyAll of another row of a table ends the query cache entry of a row a page background task read
/// (al-runner issue 8467, open corpus PR 1457, tests/al-language/pages/background-tasks/BgTaskQueryCache.al: the cell was
/// dropped from the file, as the preprocessor cannot tell the minors apart): `Some(true)` on 27.5, 28.5 and 29.0 (the PR's CI),
/// `Some(false)` on 29.1 (a local container); on every other minor, and with no target, not measured.
pub fn bulk_commit_ends_cached_row(target: Option<&Target>) -> Option<bool> {
    match target.map(|t| (t.bc.major, t.bc.minor)) {
        Some((27, 5) | (28, 5) | (29, 0)) => Some(true),
        Some((29, 1)) => Some(false),
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
        for v in ["27.5", "28.5", "29.0"] {
            assert_eq!(bulk_commit_ends_cached_row(Some(&target(v))), Some(true), "{v}");
        }
        assert_eq!(bulk_commit_ends_cached_row(Some(&target("29.1"))), Some(false));
        for v in ["27.0", "28.4", "30.0"] {
            assert_eq!(bulk_commit_ends_cached_row(Some(&target(v))), None, "{v}");
        }
        assert_eq!(bulk_commit_ends_cached_row(None), None);
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
