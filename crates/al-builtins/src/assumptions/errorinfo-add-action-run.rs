ErrorInfoAddActionRun => {
    id: "errorinfo-add-action-run",
    assumed: "`ErrorInfo.AddAction(Caption, CodeunitId, Method)` and `AddAction(Caption, CodeunitId, Method, Description)` with Method exactly `Run` or `run`, on a compiled codeunit of the program that declares no Subtype (with or without TableNo), raise `The type Microsoft.Dynamics.Nav.Types.DataError is unknown.` with code `NotSupportedType` at the AddAction call, whatever the ErrorInfo holds (empty message, ErrorType Internal, collectible inside or outside a collecting scope, after another action); so no Error call with that ErrorInfo runs. On codeunit 0 and an id no codeunit has, AddAction adds the action and raises nothing. Every other spelling of Run, a negative id, a codeunit with a Subtype and a codeunit of a dependency app that was not compiled refuse.",
    tests: "No Microsoft test. An outside report measured on a W1 BC (al-runner issue 8401); the corpus CI of the probe pull request passes on BC with these cells.",
    probe: "Corpus PR 1601, tests/al-language/errors/errorinfo/EnTErrorInfoRunAction.al: Run on a codeunit without and with TableNo, through asserterror and Codeunit.Run, lower case, with a description, after a proper action, codeunit 0, a missing codeunit, empty message, Internal, collectible in and out of a collecting scope, OnRun, NoSuchMethod.",
    awaiting: Awaiting {
        issues: &[8401],
        files: &["tests/al-language/errors/errorinfo/EnTErrorInfoRunAction.al"],
        since: "2026-11-27",
    },
}
