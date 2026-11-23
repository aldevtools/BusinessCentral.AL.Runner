ClearSharedCodeunit => {
    id: "clear-shared-codeunit",
    assumed: "Clear and ClearAll of a codeunit variable whose instance something else may share. (1) `Clear(X)` of a codeunit variable nothing has used yet, of a codeunit that is not SingleInstance, gives X a fresh instance, as it does for a used one: an interface, a codeunit variable or a parameter that shares the old instance keeps it (al-builtins `native/values.rs`, `clear`). Measured for X a local variable, a global and a var parameter, shared through an interface, a second codeunit variable and a value parameter. Assumed beyond the cells: X a value parameter itself, and an unused X bound by BindSubscription (the binding ends, as it does for a used X, events/binding-senders/BindSubscriptionClearAll.al Clear_HolderWithBoundGlobal_Binding). An element of an array of codeunits keeps main's answer (Clear of an unused element changes nothing). (2) `ClearAll()` in a codeunit that is not SingleInstance, not a test codeunit and has no TableNo resets a global codeunit variable's instance in place instead of giving the variable a fresh one, when that codeunit is not SingleInstance, has no TableNo, holds one global and that global is an Integer, the instance is not bound, and no routine of that codeunit runs (al-vm `clear_all_shared.rs`): a holder that shared the instance sees the reset and later writes through either side. Every other ClearAll keeps main's answer (the variable gets a fresh instance): a SingleInstance or test codeunit or a codeunit with a TableNo that calls it, a report, a page or a table, a global codeunit of another shape, an array of codeunits, a running instance.",
    tests: "No Microsoft test: al-runner issue 8912 (Clear of an unused codeunit variable an alias shares; the reporter is the runner's own check of a partner login).",
    probe: "Open corpus pull request 1560, tests/al-language/codeunits/clear-instance/ClearSharedCodeunit.al (75582 \"CLX Clear Shared Tests\", every cell with X unused and with X used once) and ClearSharedCodeunitSI.al (75585, SingleInstance), 96 tests green on a local Business Central 29.0 container.",
    awaiting: Awaiting {
        issues: &[8912],
        files: &[
            "tests/al-language/codeunits/clear-instance/ClearSharedCodeunit.al",
            "tests/al-language/codeunits/clear-instance/ClearSharedCodeunitSI.al",
        ],
        since: "2026-11-23",
    },
}
