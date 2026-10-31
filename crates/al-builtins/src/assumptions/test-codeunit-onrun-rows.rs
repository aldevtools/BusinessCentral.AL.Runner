TestCodeunitOnRunRows => {
    id: "test-codeunit-onrun-rows",
    assumed: "How the runner reports the OnRun trigger of a test codeunit, on every major (ADR-055): as Microsoft's Test Runner app (page 130455, runner codeunit 130450, the way BcContainerHelper and AL-Go run tests) does. No row 'OnRun'; a codeunit whose OnRun raises is Skipped with no rows and the run stays green; the tests of a codeunit whose OnRun does not raise run after it. Measured on the newest major by the reporter of al-runner issue 5859 and on the oldest supported major by corpus writer RL (probe commit c69419f6, codeunit/lt-outside-onrun-error/RlOnRunError.al); the middle major through that runner is not measured. The tests of a codeunit with a declared OnRun are flagged.",
    tests: "No Microsoft test: al-runner issue 5859 (a user's run through BcContainerHelper on the newest major; owner decision 2026-10-28, ADR-055).",
    probe: "Through Microsoft's Test Runner app on each supported major: a test codeunit whose OnRun raises, one whose OnRun does not, and one whose OnRun calls a test method; the rows and outcomes the runner reports.",
    awaiting: Awaiting {
        issues: &[5859],
        files: &["tests/al-language/codeunit/lt-sw-onrun-runner/SwOnRunRunner.al"],
        since: "2026-10-28",
    },
}
