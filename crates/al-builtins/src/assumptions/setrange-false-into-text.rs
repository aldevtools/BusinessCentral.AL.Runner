SetRangeFalseIntoText => {
    id: "setrange-false-into-text",
    assumed: "On Business Central 27, 28 and 29, in an en-US session, FieldRef.SetRange(X), FieldRef.SetRange(X, X) and FieldRef.SetFilter('%1', X) with X a typed Boolean false (a Boolean variable, or the literal false, which lowering also writes as typed), FieldRef.SetRange(V) and FieldRef.SetFilter('%1', V) with V a Variant holding false, and Record.SetRange(Field, V) with V a Variant holding false, on a stored Code[20], Code[2] or Text[50] field that is no system field, of a table that is not temporary, in filter group 0, set the filter on the Format text of false: NO in a Code field, No in a Text field (the earlier conversion).",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1575 (open, not yet pinned), recordref-fieldref/fieldref/SrcFilterBoolIntoText.al:178-221 (BoolFalse_Cod, BoolFalse_Txt, BoolFalse_Cod2), asserted by the corpus CI on the Business Central majors 27, 28 and 29 in an en-US session, each cell on a fresh RecordRef or Record; and the reporter's repro on Business Central 29 in al-runner issue 9026 (https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/9026).",
    probe: "Pin corpus PR 1575 with the next corpus bump. Refused, not measured or not served: false into a Code or Text field of another length (Code[1] and Text[1] raise the string-length error in SrcFilterBoolIntoText.al:226-253, not served), Record.SetRange with a typed value or two values, Record.SetFilter, FieldRef.SetFilter with another filter text or more values, FieldRef.SetRange with a Variant and a typed value, a FieldRef of a KeyRef, a session in another language, a FlowField, FlowFilter or system field, a temporary table or another filter group.",
    awaiting: Awaiting {
        issues: &[9026],
        files: &["tests/al-language/recordref-fieldref/fieldref/SrcFilterBoolIntoText.al"],
        since: "2026-11-24",
    },
}
