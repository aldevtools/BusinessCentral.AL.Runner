RecordrefAsRunRecord => {
    id: "recordref-as-run-record",
    assumed: "A RecordRef in the place of the record of a static run call does not compile, beyond the shape must-not-compile 1633 and 1634 pin (a local RecordRef as the record of `Page.Run` and `Page.RunModal` with two arguments and `Report.Run` with four, at runtime 16.1, 17.1 and 18.0) (al-sema `check/recordref_record_arg.rs`). Judged in these forms, the object given by an Integer (`Page::\"X\"`) and the record a RecordRef parameter, `var` parameter or global in scope: AL0133 (`Argument <n>: cannot convert from 'RecordRef' to 'Table'`, or `'var Table'` for `Report.Run`) at the argument from runtime 17.0. For a local, parameter, `var` parameter or global RecordRef, from runtime 17.0: AL0133 for `Page.Run` with a field number that is an Integer (Argument 2, 'Table'), `Report.RunModal` and `Xmlport.Run` with four arguments (Argument 4, 'var Table') and `Codeunit.Run` with two (Argument 2, 'var Table'); `Xmlport.Run`, which has no overload by name, also from the runtime floor; below 17.0 the other three are not judged and compile as before. For the three pinned calls, AL0666 at the call (`'<method>' is not available in runtime version '<runtime>'. The supported runtime versions are: '17.0' or greater.`) at every runtime from 4.1 to below 17.0, and AR1007 below 4.1, where an app that depends on the Base Application does not compile at all. A page given by name, another argument count and any other record expression are not judged. A compile refusal ends before any test runs, so it carries no flagged-pass mark.",
    tests: "No Microsoft test. Corpus pull request 1645, rounds 1 and 2: must-not-compile 3862-3868 give these AL0133 diagnostics on AL compiler 16.2.28.57946 at runtime 16.0, 17.0.34.45391 at 17.0 and 18.0.43.1464 at 18.0, each with the platform symbols of its own release; at runtime 16.1, with the platform symbols of the oldest supported release, AL compilers 17.0.34.45391 and 18.0.43.1464 give AL0666 for 3862-3866 and 3868 (the overloads by name) and AL0133 for 3867. The 1633 calls give AL0666 with AL compilers 17.0.34.45391 and 18.0.43.1464 at every runtime from 4.0 to 16.1 and AL0133 at 17.0; with a dependency on the Base Application, app.json does not compile at 4.0 (`'dependencies.id' is not available in runtime version '4.0'`). AL compiler 16.2.28.57946 gives AL0133 there from 4.0 to 16.0; the runner follows the pinned 1633 (corpus pull request 1645 review).",
    probe: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1645 (open, not yet pinned): must-not-compile 3862-3868.",
    awaiting: Awaiting {
        issues: &[9280],
        files: &[
            "tests/must-not-compile/cases/3862-page-run-recordref-parameter.al",
            "tests/must-not-compile/cases/3863-page-run-recordref-var-parameter.al",
            "tests/must-not-compile/cases/3864-page-run-recordref-global.al",
            "tests/must-not-compile/cases/3865-page-run-recordref-field-number.al",
            "tests/must-not-compile/cases/3866-report-runmodal-recordref.al",
            "tests/must-not-compile/cases/3867-xmlport-run-recordref.al",
            "tests/must-not-compile/cases/3868-codeunit-run-recordref.al",
        ],
        since: "2026-11-28",
    },
}
