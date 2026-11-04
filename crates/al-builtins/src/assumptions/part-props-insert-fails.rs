PartPropsInsertFails => {
    id: "part-props-insert-fails",
    assumed: "On a Card or Document host opened with OpenNew, when the insert that the first Visible() or Editable() of a field-linked part tries fails with a TestField error in the page's OnInsertRecord or the table's OnInsert, and the triggers wrote nothing to the database and changed no field: no error reaches the test, Visible() and Editable() answer their normal values, the host is not stored and stays unsaved, and a second Visible() or Editable() and Close() run no insert trigger. Any other call on the host or its parts afterwards is refused, and so is GetLastErrorText.",
    tests: "No Microsoft test: al-runner issue 7031 (a partner app's test reads Visible() of a part on a new card whose table's insert trigger requires a field). Measured on all three supported majors by the corpus probe below.",
    probe: "PartLineBeforeHostKey.al R7031_* tests (codeunit 67582, pages 95604-95606) on corpus master 037baafe (corpus PR 1134), not yet the pin.",
    awaiting: Awaiting {
        issues: &[7031],
        files: &[
            "tests/al-language/pages/parts/PartLineBeforeHostKey.al::R7031_ConstTrue_TableError_VisibleFirst",
            "tests/al-language/pages/parts/PartLineBeforeHostKey.al::R7031_VarFalse_PageError_EditableFirst",
        ],
        since: "2026-11-04",
    },
}
