BindingAfterFailedTest => {
    id: "binding-after-failed-test",
    assumed: "Under TestIsolation = Codeunit, with runner codeunit 130450, the probe's runner codeunit 61301 (matched by object id and name) or al-runner's own runner, which replays the steps of runner 130450 around each test (ADR-066 decision 6), not on the TestRunnerHub and not in a scratch run, a test with TransactionModel AutoCommit whose body raised an error, whose failure is trusted and that bound a manual subscriber held in a global variable of the test codeunit (not a SingleInstance codeunit) leaves it bound for the later tests of the codeunit, and a later failure is trusted (al-testrun `runtime/run.rs` `left_bindings`). Every later test of the codeunit that starts with that binding passes through this entry. Not covered, main's result (a later failure is not trusted): TestIsolation = Function or Disabled, other runners, a failed test under TransactionModel AutoRollback or None, a failure not raised by the body (a handler that did not run), a binding of a SingleInstance subscriber or one held anywhere else than a global of the test codeunit, a failure judged not trusted.",
    tests: "No Microsoft test. An outside report measured on a local BC of the middle supported major (al-runner issue 8337).",
    probe: "Corpus PR 1465 (merged 2026-11-19, not yet pinned), tests/al-language-isolation-probe/src/SqBindFailed.al cells P5a (a global binding survives the failed test; a second BindSubscription raises \"already been bound\"), P5b (a local binding ends with it) and P5c (UnbindSubscription in the next test), asserted by SqReader.al, measured on a local BC of each of the three supported majors, the same on each, under runners 61301 and 130450.",
    awaiting: Awaiting {
        issues: &[8337],
        files: &["tests/al-language-isolation-probe/src/SqBindFailed.al", "tests/al-language-isolation-probe/src/SqReader.al"],
        since: "2026-11-19",
    },
}
