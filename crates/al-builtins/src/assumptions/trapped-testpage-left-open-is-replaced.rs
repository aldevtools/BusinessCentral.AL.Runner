TrappedPageLeftOpenIsReplaced => {
    id: "trapped-testpage-left-open-is-replaced",
    assumed: "A TestPage variable that trapped a page and was left open (never closed) can trap the next RunObject action of its page: the new page replaces the open one, whose close triggers do not run. (Page.Run closes the open page first, with its close triggers: pinned by pages/lt-f3-pages/F3TrappedAndModal.al:36.)",
    tests: "Library - Error Message keeps one TestPage \"Error Messages\" in a codeunit variable, traps it in every VerifyDimErrors and never closes it; Tests-Misc Check Dimensions On Posting (35 tests) trap it again in the next test of the codeunit.",
    probe: "A codeunit with a TestPage variable that Trap()s and runs a page in test one and leaves it open; test two Trap()s the same variable and runs the page again: does it open, and do OnQueryClosePage and OnClosePage of the first page run? Asked in corpus #641.",
}
