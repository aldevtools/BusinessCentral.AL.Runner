TestRunnerTriggerTransactions => {
    id: "test-runner-trigger-transactions",
    assumed: "The transaction scope of a test runner's triggers (ADR-066 probe P4): what OnBeforeTestRun and its subscribers write is committed when it returns, before the method runs (AL Runner v2, TestExecutor.cs: NavTestExecution.BeforeTestRunAsync commits after it); OnAfterTestRun runs after the test's own commit or rollback, in the same isolation as the tests (its Commit() is a commit of the test session, as a test's own is). Not measured.",
    tests: "Every Microsoft test: Microsoft's CI runs each one through runner codeunit 130450, whose triggers write the Test Method Line rows and commit (TestRunnerMgt.Codeunit.al:118-144).",
    probe: "ADR-066 probe P4: a top-level custom runner whose OnBeforeTestRun and OnAfterTestRun insert a row (with and without Commit()), under TestIsolation Codeunit and Function; a test that fails after reading the rows, and a later test that counts them.",
}
