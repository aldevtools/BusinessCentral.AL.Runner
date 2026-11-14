TableNoCodeunitGlobalRecOwnVariable => {
    id: "tableno-codeunit-global-rec-own-variable",
    assumed: "A codeunit with TableNo and no Subtype that declares a global variable `Rec`, a record of the TableNo table without `temporary`: in a procedure, `Rec` is that global variable (what a procedure assigned to it is read back by a procedure, also after Codeunit.Run); in OnRun, `Rec` is the record given to Codeunit.Run. Unchanged, main's result: a codeunit with a Subtype runs the declared variable as the implicit Rec; any other declared type of `Rec` refuses.",
    tests: "No Microsoft test. Probe measured on a local BC of each of the three supported majors, identical: corpus PR 1393, GlobalRecVariable_BareNames_Source passes.",
    probe: "Open corpus PR 1393, tests/al-language/codeunits/TableNoBareSystemFunction.al:459-482 (GlobalRecVariable_BareNames_Source, codeunit 83004): a procedure sets the global `Rec`, Codeunit.Run gives another record; OnRun's `Rec.UserId` reads the run record, a procedure's `Rec.UserId` reads the global variable. Not measured: a temporary record, a record of another table, and a codeunit with a Subtype.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/codeunits/TableNoBareSystemFunction.al"],
        since: "2026-11-14",
    },
}
