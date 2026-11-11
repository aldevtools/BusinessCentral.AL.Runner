ModalCloseSaveErrorCode => {
    id: "modal-close-save-error-code",
    assumed: "When a ModalPageHandler edits an existing row of a Card or List without parts and returns, and the save at close raises an error whose code is not Dialog (a TestField error of a codeunit the page's OnModifyRecord runs), RunModal fails with the error's own text, as it does for a Dialog error (pages/close-errors/CemModalHandlerTests.al:900, :1743, code TestWrapped). The code it carries is not measured: GetLastErrorCode refuses.",
    tests: "No Microsoft test: al-runner issue 7400, the reporter's repro (a List without OnQueryClosePage whose OnModifyRecord runs a codeunit that calls TestField; Assert.ExpectedError of the TestField text passes on real BC).",
    probe: "pages/close-errors/ModalCloseSaveErrorShapes.al: the reporter's shape on a List and a Card without OnQueryClosePage (TestField through Codeunit.Run, and Error), the error text and code; OnInsertRecord raising at close of a new row; a Worksheet.",
    awaiting: Awaiting {
        issues: &[7400],
        files: &["tests/al-language/pages/close-errors/ModalCloseSaveErrorShapes.al"],
        since: "2026-11-11",
    },
}
