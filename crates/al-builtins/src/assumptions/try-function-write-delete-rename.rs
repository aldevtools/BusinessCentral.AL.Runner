TryFunctionWriteDeleteRename => {
    id: "try-function-write-delete-rename",
    assumed: "Delete, DeleteAll (also on an empty table), ModifyAll and Rename of a database table inside a TryFunction whose result is used raise TryFunctionWrite ('Call to the function 'DELETE' is not allowed inside the call to '<scope>' when it is used as a TryFunction.', names DELETE, DELETEALL, MODIFYALL, RENAME) before any row changes, as Insert and Modify do. A table with triggers or subscribers for the write stays refused.",
    tests: "No Microsoft test: an outside report (al-runner issue 7143: a TryFunction that wraps Planning Line Management.Calculate, which deletes and inserts planning components).",
    probe: "Pinned by R7143TryWriteDeny.al:94-176 (corpus PR 1156, branch al-runtime-rs-staging-r7142), measured on the 27.5, 28.5 and 29.0 CI containers: retire when the pin includes it.",
    awaiting: Awaiting {
        issues: &[7143],
        files: &["tests/al-language/errors/lt-r7143-try-write/R7143TryWriteDeny.al"],
        since: "2026-11-05",
    },
}
