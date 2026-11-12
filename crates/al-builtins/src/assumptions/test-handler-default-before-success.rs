TestHandlerDefaultBeforeSuccess => {
    id: "test-handler-default-before-success",
    assumed: "TestHandlerContext.Success() reads false in every Before callback (OnBeforeTestCodeunitRun, OnBeforeTestProcedureRun, OnBeforeTestCaseRun) of a default handler (an enum extension of DefaultTestHandler), as it does in a TestHandlers handler (pinned, BiTestHandlerSuccess.al:116).",
    tests: "No corpus test for a default handler. al-runner issue 7181: the outside reporter measured Success = false in every Before callback of a DefaultTestHandler handler on BC 29.0 W1 (altool runtests and runner 130450); their default handler reads it in every callback.",
    probe: "tests/al-language-xhandler-lib/XhLibHandler.al: a default handler in the XHandler Lib app logs Success in every callback of test codeunit 81599; BiTestHandlerSuccess.al BiTs_Zz_DefaultHandlerSuccess reads that log.",
    awaiting: Awaiting {
        issues: &[7181],
        files: &["tests/al-language-xhandler-lib/XhLibHandler.al"],
        since: "2026-11-12",
    },
}
