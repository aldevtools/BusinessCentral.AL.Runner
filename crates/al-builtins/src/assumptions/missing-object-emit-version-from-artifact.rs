MissingObjectEmitVersionFromArtifact => {
    id: "missing-object-emit-version-from-artifact",
    assumed: "The number at the end of the missing-object error (\"... does not exist in the current application compiled with emit version N.\", errors/error-texts/AzErrorTexts.al:125, where the corpus masks N) is the constant EmitVersion.CodeEmitVersion of Microsoft.Dynamics.Nav.CodeAnalysis.dll in the platform artifact of the run's BC build (al-deps `emit_version`, read on the first such error; al-vm transactions.rs `missing_object_from`). Every missing-object text that carries the number passes through this entry: a missing codeunit, table, page or report, from any caller. When the run has no build to read it from, or the artifact gives no number, the text is not known: a reader of the text refuses and GetLastErrorCode still answers AL:MetadataNotFound.",
    tests: "No Microsoft test. Read from the artifacts: 28014 in 28.5.54151.54365, 28015 in 28.5.54976.58543 and 28.5.54151.58194, 27025 in builds 54899.58548 and 46862.58057 of the major 27 minor 5 (al-deps `emit_version` lists every build read), 29022 in 29.0.54011.55816 and 29.0.54011.56262. The corpus comment at errors/error-texts/AzErrorTexts.al:125 records the numbers its CI saw: 27025, 28015 and 29022 on its three legs, oldest first (the CI pins 29.0.54011.55816, .github/workflows/ci.yml:365). On BC, NavEnvironment.EmitVersion formats the error and defaults to that constant (EmitVersionOverride -1). AL Runner v2 read the same constant and printed 27024 for build 46862.48827 of the same minor.",
    probe: "Corpus PR 1483 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/errors/last-error/EmitVersion.al: the number cut out of the text, its digits and its value div 1000 on each of the three CI legs, the value 29022 on 29.0.54011.55816, and the same number for a missing codeunit, table, page and report. Measured on a local Business Central 29.0 so far: the text shape and the code; the number cells are still being observed.",
    awaiting: Awaiting {
        issues: &[7617],
        files: &["tests/al-language/errors/last-error/EmitVersion.al"],
        since: "2026-11-20",
    },
}
