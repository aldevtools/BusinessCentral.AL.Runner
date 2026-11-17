XrecTableProcThroughCodeunit => {
    id: "xrec-table-proc-through-codeunit",
    assumed: "A procedure of a table called on a record that a codeunit procedure got as a `var` parameter runs with the xRec of the field OnValidate trigger of that record: the codeunit procedure (plain or reached through an interface) was called by a procedure of the same table that is no trigger, called from the table's own field OnValidate, in a `Record.Validate` AL code called on a record whose buffer came from its own Insert or a Get. Every other call through other code on a record whose trigger or procedure runs with an xRec that is not blank refuses a read of xRec (al-vm records/nested_validate.rs `XREC_OF_RUNNING_CALL`).",
    tests: "al-runner issue 8189: the reporter's app (10 tests; ways 3 and 4 after an Insert and after a Get) passes on real BC; the issue has the code.",
    probe: "The reporter's ways 3 and 4 (a plain codeunit and a codeunit behind an interface, after an Insert and after a Get), plus neighbors: the codeunit called from the trigger directly, a fresh record, a Validate from a TestPage, FieldRef.Validate, OnInsert and OnModify as the running trigger, a table extension's OnValidate, the procedure assigning xRec and the trigger reading it after the call.",
    awaiting: Awaiting {
        issues: &[8189],
        files: &["tests/al-language/record/validate/XRecTableProcThroughCodeunit.al"],
        since: "2026-11-17",
    },
}
