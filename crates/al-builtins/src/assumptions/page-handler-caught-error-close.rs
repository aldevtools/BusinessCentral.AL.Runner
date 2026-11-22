PageHandlerCaughtErrorClose => {
    id: "page-handler-caught-error-close",
    assumed: "A PageHandler or ModalPageHandler that caught an error before it handled the page closes the page as a handler that caught none (al-vm pages/page_run_close.rs `page_run_cell`, pages/document_handler_error.rs `close_document_after_handler_error`; the session's count of caught errors, caught_errors.rs, tells that the handler caught one). Every cell of the pinned close matrix of Page.Run (the card by object id or from a Page variable, the list, the wizard) and every case of the pinned document's Page.RunModal after a handler error answers as with nothing caught; a case refused with nothing caught is refused with a caught error too. The way the handler caught the error does not matter: asserterror, a TryFunction, two errors, ClearLastError after the catch, Codeunit.Run or JsonObject.ReadFrom with its result used, a procedure of the handler that catches. Flagged: each such close after a handler that caught an error.",
    tests: "No Microsoft test. The repro of al-runner issue 8729 (1 of 1 green on a Business Central 29.1 W1 container in each of 3 runs); corpus PR 1524 CemHandlerCaughtErrors.al (merged), 23 of 23 green there in each of 3 runs; corpus PR 1531 CemPageHandlerCaughtTests.al and CemDocumentCaughtTests.al, where each of the 98 tests gave the answer of its pinned test in each of 3 runs on that container (64 pass; the other 34 differ from their pinned answer on that container with nothing caught too, an unhandled Message or Confirm in a close trigger, and its CI on every supported major settles them).",
    probe: "Corpus PR 1531, tests/al-language/pages/close-errors/CemPageHandlerCaughtTests.al and CemDocumentCaughtTests.al: every test of CemPageHandlerTests.al and CemDocumentTests.al with a handler that first catches an error with asserterror, the same expectations. Corpus PR 1524 (merged), CemHandlerCaughtErrors.al: the card by object id after a handler error with asserterror (:150), a TryFunction (:157), two errors (:164), asserterror and ClearLastError (:199), Codeunit.Run (:206), JsonObject.ReadFrom (:213), a procedure that catches (:220), none (:171); a return (:178), OK (:185), a return while OnQueryClosePage raises (:192); the list (:227, :234); Page.RunModal of the card (:370, :377, :384) and the list (:391); the document after the six ways of catching (:398 to :433).",
    awaiting: Awaiting {
        issues: &[8729],
        files: &[
            "tests/al-language/pages/close-errors/CemHandlerCaughtErrors.al",
            "tests/al-language/pages/close-errors/CemPageHandlerCaughtTests.al",
            "tests/al-language/pages/close-errors/CemDocumentCaughtTests.al",
        ],
        since: "2026-11-22",
    },
}
