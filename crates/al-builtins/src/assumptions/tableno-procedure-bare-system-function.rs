TableNoProcedureBareSystemFunction => {
    id: "tableno-procedure-bare-system-function",
    assumed: "In a procedure of a codeunit with TableNo (global, local, internal, event subscriber, test procedure; a parameter named `Rec` included) whose table has a field named like a system function, a bare unquoted call of that system function (`UserId`, `Today`, `WorkDate`, `CompanyName`, `Message`, with or without parentheses) runs the system function, not the field: the procedure has no implicit Rec (corpus must-not-compile 3140, corpus PR 1386, merged, not pinned yet). Refused, as before: a trigger other than OnRun (Install, Upgrade, TestRunner triggers), a codeunit that declares a global `Rec`, and the quoted name (`\"UserId\"`). OnRun keeps the field.",
    tests: "Outside report: al-runner issue 6924, the reporter's Microsoft-style test SalesInvoicePostTestApplyCustEntriesByApplicationNo passes on real BC (comment of 2026-11-13).",
    probe: "corpus PR TBD (bare system function in a TableNo codeunit procedure): OnRun writes the fields, then a procedure, a local procedure, an event subscriber, a test procedure and a procedure with a parameter `Rec` read bare UserId, Today, WorkDate and CompanyName and call Message; also the three refused shapes.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/codeunits/TableNoBareSystemFunction.al"],
        since: "2026-11-14",
    },
}
