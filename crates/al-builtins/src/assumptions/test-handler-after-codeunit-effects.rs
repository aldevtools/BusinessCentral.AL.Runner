TestHandlerAfterCodeunitEffects => {
    id: "test-handler-after-codeunit-effects",
    assumed: "OnAfterTestCodeunitRun of an ITestHandler runs after the last test of the codeunit, and what it sets in SingleInstance state is visible to the next test codeunit (SingleInstance state crosses test codeunits, corpus PR 1161). A handler error or Skip there is not pinned: the codeunit's passing tests are then reported as not run.",
    tests: "Every test of a test codeunit whose handler implements OnAfterTestCodeunitRun (BiTestHandler.al:47, BiTestHandlerBehavior.al:232, BiTestHandlerDefault.al:121).",
    probe: "A handler whose OnAfterTestCodeunitRun adds to a SingleInstance log, a second test codeunit (a later object id, same app) that reads the log: is the entry there? Then the same with an Error and with Skip in OnAfterTestCodeunitRun: do the tests of the first codeunit keep their result? (BiTestHandler.al:11-12 says a first attempt read it empty in run 39225437143, which is not pinned.)",
}
