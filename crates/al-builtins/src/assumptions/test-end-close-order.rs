TestEndCloseOrder => {
    id: "test-end-close-order",
    assumed: "When two or more local TestPages with a close trigger are still open as the test method returns, each closes as Close() closes it (testpageclose/Y47CloseAtTestEnd.al pins that for one page), in the order of their declaration.",
    tests: "No Microsoft test: corpus testpageclose/TestPageClosingActions.al Card_OK_ThenEveryOpenModeOpensTheVariableAgain and List_OK_ThenEveryOpenModeOpensTheVariableAgain leave two pages open and pass on real BC.",
    probe: "A test that leaves two local TestPages of pages with OnQueryClosePage and OnClosePage open, declared in one order and opened in the other: the order of the triggers in a SingleInstance log read by the next test.",
}
