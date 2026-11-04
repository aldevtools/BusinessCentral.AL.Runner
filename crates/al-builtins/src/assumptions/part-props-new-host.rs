PartPropsNewHost => {
    id: "part-props-new-host",
    assumed: "On a Document or Card host opened with OpenNew and nothing typed, the first Visible() or Editable() of a field-linked part inserts the host as a first field read does, with or without rows of the blank link key. On such a host opened with OpenEdit over an empty table, the part shows no row although rows of the blank key exist, and stamps its link value without validating it.",
    tests: "No Microsoft test: al-runner issue 6920 (a partner app's test reads Visible() of a part on a new card). Observed on the newest major by the corpus probe below.",
    probe: "PartLineBeforeHostKey.al R6920_* tests (codeunit 67582, pages 95160-95166) on corpus master 55fa9dff (corpus PR 1113).",
    awaiting: Awaiting {
        issues: &[6920],
        files: &[
            "tests/al-language/pages/parts/PartLineBeforeHostKey.al::R6920_CardIssue_BlankRows_OpenNew_VisibleFirst",
            "tests/al-language/pages/parts/PartLineBeforeHostKey.al::R6920_DocAfterGet_BlankRows_OpenEdit_VisibleFirst",
        ],
        since: "2026-11-04",
    },
}
