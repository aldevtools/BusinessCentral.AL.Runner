TableExtSiblingProtectedVar => {
    id: "tableext-sibling-protected-var",
    assumed: "A table extension may use a `protected var` global (a simple type or a Record) that another extension of the same table in the same app declares: one variable per record variable, shared by both extensions; record assignment and Copy do not copy it, Init keeps it, Clear resets it.",
    tests: "No Microsoft test: al-runner issue 6485, an ISV app with two extensions of one table (125 tests refused). Measured on BC by corpus writer Y20 (Y20TableExtSiblingGlobals.al, SaaS sandbox, newest major).",
    probe: "Y20TableExtSiblingGlobals.al on branch al-runtime-rs-staging-y20: a trigger of one extension writes the other's Record and Integer globals; per record variable, assignment, Copy, Init, Clear; Insert(false).",
    awaiting: Awaiting {
        issues: &[6485],
        files: &["tests/al-language/tableextension/lt-y20-sibling-globals/Y20TableExtSiblingGlobals.al"],
        since: "2026-10-31",
    },
}
