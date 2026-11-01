DataTransferInstallInitValueNoTriggers => {
    id: "datatransfer-install-initvalue-no-triggers",
    assumed: "Inside install code, DataTransfer.CopyRows gives an unmapped destination field its InitValue, and neither CopyRows nor CopyFields runs the destination table's triggers (or its table extensions' code). Measured on real BC by corpus writer Y47; the copies themselves are pinned by record/lt-as-records/AsDtInstaller.al and AsDtInstallRead.al.",
    tests: "No Microsoft test: al-runner issue 6586 (the reporter backfills fields with DataTransfer during install and upgrade).",
    probe: "CopyRows into a table with an InitValue field left unmapped, and CopyRows and CopyFields into a table with OnInsert, OnModify and an OnValidate on a mapped field: the stored values and a trigger log.",
    awaiting: Awaiting {
        issues: &[6586],
        files: &[
            "tests/al-language/record/lt-as-records/Y47DtInstall.al",
            "tests/al-language/record/lt-as-records/Y47DtInstallRead.al",
        ],
        since: "2026-11-01",
    },
}
