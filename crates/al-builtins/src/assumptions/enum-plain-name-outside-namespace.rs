EnumPlainNameOutsideNamespace => {
    id: "enum-plain-name-outside-namespace",
    assumed: "A procedure's local variable `Enum \"E\"` (one-segment name) in a file with a namespace and no `using` of the enum's namespace is AL0185 `Enum 'E' is missing` when exactly one enum `E` is visible and it is declared in the file's own app in a sibling namespace of the file's (the same parent, another last part, as `Acme.Logic` and `Acme.Kinds`) (al-sema `alcodes/enum_variables.rs`). A compile refusal ends before any test runs, so it carries no flagged-pass mark. Every other shape keeps compiling: a global variable, a parameter, a trigger's variable, a file without a namespace, a parent or child namespace, a namespace with another parent, two enums of that name, an enum of a dependency app.",
    tests: "No Microsoft test. An outside report: the reporter's own alc 18.1 output gives `AL0185: Enum 'Widget Kind' is missing` for this shape and compiles it with the `using` or the qualified name (al-runner issue 7921).",
    probe: "Corpus PR 1363 measures the cross-app forms (must-not-compile 3134: a Base Application enum; 3135: an enum of AL Language XApp A). The same-app form cannot be a must-not-compile case: a case is one file, and one file declares one namespace; it rests on the issue 7921 repro until the corpus can hold a case of two files of one app.",
    awaiting: Awaiting {
        issues: &[7921],
        files: &[
            "tests/must-not-compile/cases/3134-enum-plain-name-from-other-namespace-without-using.al",
            "tests/must-not-compile/cases/3135-enum-plain-name-of-dependency-app-other-namespace.al",
        ],
        since: "2026-11-12",
    },
}
