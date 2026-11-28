ErrorInfoInternalText => {
    id: "errorinfo-internal-text",
    assumed: "An `Error(ErrorInfo)` with ErrorType Internal, whose Title and ControlName are empty, Verbosity is Error, DataClassification is CustomerContent, TableId, FieldNo and PageNo are 0, RecordId and SystemId were never set, CustomDimensions was never set or read, and no action was added (any Message, also none, and any DetailedMessage), raises the generic text, a backslash, `Correlation ID: ` and a new id on every raise: 36 characters, lower case hex with dashes, no braces. That is the text of GetLastErrorText() and GetLastErrorText(false), with code Dialog, after asserterror, after a [TryFunction] that returns false and after Codeunit.Run of an instance that returns false; GetLastErrorText(true) is the personal data removed text. A collectible one inside a collecting scope is collected, and its collected Message has the same id after CR LF; when the collecting procedure ends with it alone, the raised text has the backslash again. A collectible one raised outside a collecting scope, and every other shape of an Internal ErrorInfo, keep the braced id.",
    tests: "No Microsoft test. Outside report al-runner issue 9409 (the reporter's BC on Windows: no braces). Corpus PR 1666 (merged, not pinned yet), tests/al-language/errors/errorinfo/ErrorInfoInternalText.al, codeunit 77400, 11 tests: green on the three supported majors in its corpus CI run.",
    probe: "Corpus PR 1666, tests/al-language/errors/errorinfo/ErrorInfoInternalText.al: Message by method, by Create, none, with a DetailedMessage; GetLastErrorText(true), (false) and (); TryFunction; Codeunit.Run; collected Message; end of a collecting procedure; two raises. Not measured: Title, actions, CustomDimensions and the other properties, a collectible one outside a collecting scope, TestField and FieldError with an ErrorInfo, GetLastErrorObject.",
    awaiting: Awaiting {
        issues: &[9409],
        files: &["tests/al-language/errors/errorinfo/ErrorInfoInternalText.al"],
        since: "2026-11-28",
    },
}
