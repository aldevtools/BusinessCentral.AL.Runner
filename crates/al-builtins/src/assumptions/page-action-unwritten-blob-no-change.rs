PageActionUnwrittenBlobNoChange => {
    id: "page-action-unwritten-blob-no-change",
    assumed: "On an editable page (Editable and ModifyAllowed left true), a Blob of Rec that no stream wrote since the action started is no change of the row, so an action that changes nothing else in Rec leaves no edit for the page to save: what the action's code stored through its own record variable stays. Pinned for Modify itself (record/PnASystemFields.al:110) and for a List with Editable = false and ModifyAllowed = false (testpage/RmReadOnlyBlob.al:316-470). An action whose code committed is refused.",
    tests: "No Microsoft test: an outside report (al-runner issue 7114), 3 tests of a partner app that pass on a BC 28.5 container (an editable List over a table with a Blob, an action that imports the row through a codeunit).",
    probe: "RmReadOnlyBlob.al's list and a Card made editable, with an action that modifies the row through its own record variable, without and with a Commit after the Modify: the stored row after the action and after Close, the log of OnModifyRecord and the table's OnModify; contrast an action that writes the Blob of Rec through a stream.",
    awaiting: Awaiting {
        issues: &[7114],
        files: &["tests/al-language/pages/actions/R7114EditableBlobAction.al"],
        since: "2026-11-05",
    },
}
