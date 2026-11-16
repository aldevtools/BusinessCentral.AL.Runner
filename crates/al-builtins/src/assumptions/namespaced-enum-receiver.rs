NamespacedEnumReceiver => {
    id: "namespaced-enum-receiver",
    assumed: "`Ns.Sub.\"Enum\".FromInteger(n)` without `Enum::`, where `Ns` is no variable, parameter, field or procedure in scope and no object of any kind in an app the file sees is named `Ns`, names the enum through its namespace and runs as `Enum::Ns.Sub.\"Enum\".FromInteger(n)` does (pinned by types/ProbeNamespacedEnumQualified.al:14). Only FromInteger is taken; Names, Ordinals and other members on such a receiver keep AL0118. Not measured but taken: an unquoted enum name, quoted namespace parts, other letter case, a caller file without a namespace.",
    tests: "No Microsoft test that the runner reaches. An outside report: alc 18.1 compiles it and Base Application's ServIntegrationMgt.Codeunit.al:442 uses it (al-runner issue 7920).",
    probe: "A types test that calls `Ns.Sub.\"Enum\".FromInteger(1)` from another namespace as a case selector and into a variable, with and without a using (corpus PR 1363, tests/al-language/types/enum-option/NqReceiverNoUsing.al and NqReceiverUsing.al).",
    awaiting: Awaiting {
        issues: &[7920],
        files: &["tests/al-language/types/enum-option/NqReceiverNoUsing.al", "tests/al-language/types/enum-option/NqReceiverUsing.al"],
        since: "2026-11-12",
    },
}
