PageVarValidateSavesRowFirst => {
    id: "page-var-validate-saves-row-first",
    assumed: "TestField.SetValue on a control bound to a page variable saves the row the page holds an edit of first (OnModifyRecord, OnBeforeModifyEvent, OnModify, OnAfterModifyEvent; a new row is inserted), then runs the control's OnValidate; a control bound to a field of Rec saves nothing. Measured on BC 29 by corpus writer Y55; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6629, the reporter's Worksheet whose page variable OnValidate reads the row back with Get.",
    probe: "Y45SaveBeforeAction.al NextField_* (28 tests) and Y55NextFieldFixtures.al on corpus branch al-runtime-rs-staging-y55 (c79b80ea), measured on SaaS BC 29.",
    awaiting: Awaiting {
        issues: &[6629],
        files: &[
            "tests/al-language/pages/lt-y45-save-before-action/Y45SaveBeforeAction.al::NextField_Worksheet_NameThenPageVar",
            "tests/al-language/pages/lt-y45-save-before-action/Y55NextFieldFixtures.al",
        ],
        since: "2026-11-01",
    },
}
