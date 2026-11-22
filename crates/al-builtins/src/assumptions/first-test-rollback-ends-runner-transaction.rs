FirstTestRollbackEndsRunnerTransaction => {
    id: "first-test-rollback-ends-runner-transaction",
    assumed: "In a test codeunit's first test method, the rollback of an asserterror in the code of the test (with or without a write in it) ends the test runner's transaction: a Database.CurrentTransactionType set after it takes effect, as after a Commit, and the next test method starts with the type it set. Marked where AL code reads the type such a set gave. Only the asserterror is measured: after the rollback of a guarded Codeunit.Run, of a report's world or of an XmlPort import the runner's transaction is kept, and a set at the start is refused as before.",
    tests: "A first test method that traps an error and then sets the type.",
    probe: "session-system/TxTypeFirstTestMethod.al (the asserterror cells) of corpus pull request 1520, green on a local Business Central container and in the corpus pull request's CI on every supported major.",
    awaiting: Awaiting {
        issues: &[8324],
        files: &["tests/al-language/session-system/TxTypeFirstTestMethod.al"],
        since: "2026-11-22",
    },
}
