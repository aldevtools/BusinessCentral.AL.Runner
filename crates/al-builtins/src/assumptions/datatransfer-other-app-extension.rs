DatatransferOtherAppExtension => {
    id: "datatransfer-other-app-extension",
    assumed: "DataTransfer.CopyFields into a table that has a table extension (of the table's own app or of another app) copies, filters and joins the extension's fields like the table's and runs no trigger. Measured on SaaS (newest major) by corpus writers Y57 (own app) and Y65 (a Base Application table with another app's extension); not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6668 (install code copying Item Attribute.Name into a field of the app's own table extension).",
    probe: "Y47DtInstallRead.al Y65_DtItemAttribute_Outcomes (installer S8/S9 in Y47DtInstall.al, Y65DtItemAttributeExt.al; corpus PR 1012) and Y57_DtExt_* (corpus PR 1006).",
    awaiting: Awaiting {
        issues: &[6668],
        files: &["tests/al-language/record/lt-as-records/Y65DtItemAttributeExt.al"],
        since: "2026-11-01",
    },
}
