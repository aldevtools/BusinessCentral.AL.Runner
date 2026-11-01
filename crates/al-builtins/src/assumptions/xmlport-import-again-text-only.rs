XmlPortImportAgainTextOnly => {
    id: "xmlport-import-again-text-only",
    assumed: "A second unguarded Import on the same XML XmlPort instance whose schema has no table element reads the source again and runs its triggers, with the globals the first Import left (real BC values that corpus writer Y66 measured, not on corpus master yet). With a table element the second Import reads nothing (QrXpContainerOccurs.al TT_ImportTwice_*).",
    tests: "No Microsoft test: al-runner issue 6657, Y66's probe of the members of an XmlPort variable.",
    probe: "Y66XmlPortVarMembers.al:235-277 (`got = a;a;`); which part of the schema decides whether the second Import reads again.",
    awaiting: Awaiting {
        issues: &[6657],
        files: &["tests/al-language/xmlport/lt-y66-xmlport-var-members/Y66XmlPortVarMembers.al"],
        since: "2026-11-01",
    },
}
