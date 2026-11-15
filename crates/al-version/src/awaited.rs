//! Behavior that differs between BC versions in an open corpus probe, before a corpus bump pins it. Not a [`crate::Gate`]: a gate
//! needs its `#if BCnnPLUS` guard in the pinned corpus (`tests/gate_completeness.rs`). When the bump pins the probe's file, that
//! test fails until the function here becomes a gate. A major that the probe did not measure gets `None`; no target means the
//! newest behavior, as [`crate::has`] does.

use crate::Target;

/// Whether a temporary part that a Visible expression hid, on the Worksheet or List host of the journal shape, reads its rows
/// once more after the host's refresh that follows an action, and runs OnAfterGetCurrRecord on its blank row when a host move
/// emptied it and left it hidden (open corpus PR 1367, testpage/parts/HiddenPartJournalShape.al, the `#if BC29PLUS` guards of
/// :540, :574, :608, :642 and :677 at commit 32b29320; green on 27.5, 28.5 and 29.0 in its CI run 43307199491): `Some(false)` on 27 and 28,
/// `Some(true)` on 29.
pub fn journal_temp_part_reads_after_refresh(target: Option<&Target>) -> Option<bool> {
    match target.map(|t| t.bc.major) {
        Some(27 | 28) => Some(false),
        Some(29) | None => Some(true),
        Some(_) => None,
    }
}

/// Whether the `Permissions` property of a table is the object grant of the code of its table extensions (the OnInsert trigger,
/// a procedure of the extension) (open corpus PR 1403, permissions/testpermissions/Y19PermExtAndCompose.al, the `#if BC29PLUS`
/// guards of PC_B_ExtensionCode for b13, b14, b17, b18, b23, b24, b27 and b28, commit 100c39fa; observed on 27.5, 28.5 and 29.0 in
/// its CI run 43637000726): `Some(true)` on 27 and 28, `Some(false)` on 29.
pub fn base_table_permissions_cover_extension_code(target: Option<&Target>) -> Option<bool> {
    match target.map(|t| t.bc.major) {
        Some(27 | 28) => Some(true),
        Some(29) | None => Some(false),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_table_grant_of_extension_code_follows_the_measured_majors() {
        let at = |v: &str| base_table_permissions_cover_extension_code(Some(&target(v)));
        assert_eq!(at("27.5"), Some(true));
        assert_eq!(at("28.5"), Some(true));
        assert_eq!(at("29.0"), Some(false));
        assert_eq!(base_table_permissions_cover_extension_code(None), Some(false));
        assert_eq!(at("30.0"), None);
    }

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
}
