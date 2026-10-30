# ADR-058: Results of BC 29 test framework features: skipped tests and data-driven test cases

- Status: Accepted (owner, 2026-10-30)
- Date: 2026-10-30

## Context

BC 29 (runtime 18.0) adds two test framework features that change the shape of a test result.

1. **Test handlers.** A test codeunit names handlers in its `TestHandlers` property (values of the system enum
   `TestHandler`, 2000000023, each implementing `ITestHandler`). `TestHandlerContext.Skip(Reason)` called in
   `OnBeforeTestProcedureRun` (or `OnBeforeTestCaseRun`) keeps the test body from running, and the test is
   reported as skipped (corpus `al-language-testlibraries/lt-bi-test-framework/BiTestHandler.al:7-9`, `:56`, `:125`).
   Our runner has three outcomes: Pass, Fail, NotRun. NotRun means "the runner refused or could not run it". JUnit
   writes NotRun as `<skipped>`, and the corpus ratchet and every count treat it as a gap of the runner.
   A test skipped by a handler is not a gap: real BC skips it too.
2. **Data-driven tests.** A procedure with `[TestDataSource(Codeunit, DataSet)]` and no `[Test]` runs once per
   case that the data source's `ITestDataSource.ListTestCases` returns (Microsoft's docs; corpus
   `BiTestDataSource.al:87`, not asserted: on the Linux corpus CI every such codeunit ended its session).
   Our result is one row per test procedure (`CaseId` = codeunit id and procedure name). One procedure now has
   many results, and the name of each result row (procedure name, case name, both, index) is not known.

Until this is decided, the runner refuses both: a codeunit with `TestHandlers`, or any extension of
`DefaultTestHandler`, has all its tests reported as not run, and a data-driven test is one not-run row under
its procedure name (`al-testrun` `runtime/run.rs`, `test_handlers_refusal`, `data_driven_tests`).

## Decision (proposed)

1. Add a fourth outcome, `Skipped(reason)`, for a test that BC itself skips (today only `TestHandlerContext.Skip`).
   It is written to JUnit as `<skipped message="...">` like NotRun, but it counts as matching BC: the ratchet and the
   pass rate count it apart from refusals, and `--top-refusals`, `--explain` and the not-run counts leave it out.
2. A data-driven test gives one result per test case. The `CaseId` gets an optional case part. The JUnit
   `name` and the console line use the shape Microsoft's test runner writes for a case. That shape is not
   known yet; it must come from a probe on a Windows container (see the probe list in the hand-off note)
   before this part is built. Until then, data-driven tests stay refused.
3. Both changes touch the result model shared by `al-test-cli` and `al-runner` (`al-testrun` `Outcome`,
   `CaseId`, JUnit and JSON writers, the `--affected` store, the ratchet). They are built behind this ADR only.

## Consequences

- Test codeunits that use `TestHandlers` or `[TestDataSource]` can be supported without being counted as runner
  gaps (skips) and without losing per-case results.
- Every consumer of `Outcome` must handle the new variant (exhaustive matches make the compiler list them).
- The JUnit row names of data-driven cases depend on a probe; picking a shape before it would be a guess.

## Conformance

- `crates/al-test-cli/tests/test_framework_refusals.rs`: the refusals that hold until this ADR is accepted.
- `crates/al-test-cli/tests/data_driven_tests.rs`: decision 2 as built (2026-10-30). The row shape is
  `Procedure[identifier]`, pinned by corpus `al-language-testlibraries/lt-bi-test-framework/BiDataDriven.al` (corpus
  PR 861: the BC 29 reference run in its header and the result names of the corpus CI on BC 29). No `CaseId` field was
  added: the case name is the test name of the row, so every consumer of the result model works unchanged.
