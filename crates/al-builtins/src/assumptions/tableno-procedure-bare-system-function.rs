TableNoProcedureBareSystemFunction => {
    id: "tableno-procedure-bare-system-function",
    assumed: "In a procedure (global, local, internal, a parameter named `Rec` of the TableNo table, an event subscriber) of a codeunit with TableNo and no Subtype whose table has a field named like a system function, the bare name (`UserId`, `Today`, `WorkDate`, `CompanyName`, `Message`, with or without parentheses) and the quoted name (`\"UserId\"`, `\"Today\"`, `\"WorkDate\"`, `\"CompanyName\"`) call the system function, not the field; the same when the codeunit also declares a global `Rec`. Refused (not run in the probe): a trigger other than OnRun (Install, TestRunner triggers) and every routine of a codeunit with a Subtype (a test procedure of a test codeunit, a procedure of an Install codeunit). OnRun keeps the field.",
    tests: "Outside report: al-runner issue 6924, the reporter's Microsoft-style test SalesInvoicePostTestApplyCustEntriesByApplicationNo passes on real BC (comment of 2026-11-13). Probe measured on a local BC of each of the three supported majors, identical: corpus PR 1393, 9 of 9 pass.",
    probe: "Open corpus PR 1393, tests/al-language/codeunits/TableNoBareSystemFunction.al (codeunit 83005): a global, local and internal procedure and a procedure with a parameter `Rec` call the system function (:337-401); the quoted names in a procedure (:405-417); bare `Message` in every procedure (:421-431); an event subscriber (:435-456); a codeunit with a global `Rec`: the procedure calls the system function and `Rec.UserId` reads the global variable, OnRun reads the run record (:459-482). Not run there: a test codeunit, an Install codeunit and a TestRunner trigger with TableNo (:20-23).",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/codeunits/TableNoBareSystemFunction.al"],
        since: "2026-11-14",
    },
}
