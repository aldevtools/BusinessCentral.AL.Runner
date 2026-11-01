QuotedEnumInitValue => {
    id: "quoted-enum-init-value",
    assumed: "An Enum field whose InitValue names a member in single quotes (`InitValue = 'G/L Account';`) starts at that member when the text is a member's name in any case and with spaces around it, as the double-quoted and the plain forms do; a number and an unknown name stay refused. Measured by corpus writer Y47 on real BC (Y47EnumInitQuoted.al on corpus branch al-runtime-rs-staging-y47) (types/TestBlankEnumInitTests.Codeunit.al): on a fresh variable, after Init and after Insert.",
    tests: "No Microsoft test: al-runner issue 6585, the reporter's app passes on real BC of the newest major with this field shape.",
    probe: "An Enum field with InitValue = 'Name With Space' and one with InitValue = 'Plain': the value of a fresh variable, after Init, and after Insert and Get, on all three majors.",
    awaiting: Awaiting {
        issues: &[6585],
        files: &["tests/al-language/record/lt-y47-initvalue-quoted/Y47EnumInitQuoted.al"],
        since: "2026-11-01",
    },
}
