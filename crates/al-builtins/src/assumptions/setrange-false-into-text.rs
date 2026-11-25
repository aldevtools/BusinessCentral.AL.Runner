SetRangeFalseIntoText => {
    id: "setrange-false-into-text",
    assumed: "On Business Central 29, in an en-US session, FieldRef.SetRange(X) and FieldRef.SetFilter('%1', X) with X a typed Boolean false (a Boolean variable, or the literal false, which lowering also writes as typed), on a stored Code[20] or Text[50] field that is no system field, of a table that is not temporary, in filter group 0, set the filter on the Format text of false: NO in a Code field, No in a Text field (the earlier conversion).",
    tests: "The reporter's repro on Business Central 29.0 in al-runner issue 9026 (https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/9026): FieldRef.SetRange(false) and FieldRef.SetFilter('%1', false) on a Code[20] and a Text[50] field give the filters NO and No.",
    probe: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1575 (open, observe form), recordref-fieldref/fieldref/SrcFilterBoolIntoText.al: false into Code[20], Text[50], Code[2], Code[1] and Text[1] fields and true into Code[2], Code[1] and Text[1] fields, by FieldRef.SetRange(X), SetRange(V), SetRange(X, X), SetFilter('%1', X), SetFilter('%1', V) and Record.SetRange(Field, V), on Business Central 27, 28 and 29. Refused until it is measured: false into a Code or Text field on Business Central 27 or 28, into a field of another length, by FieldRef.SetRange(V) or SetFilter('%1', V) with V a Variant, FieldRef.SetRange(X, X), Record.SetRange or Record.SetFilter, FieldRef.SetFilter with another filter text or more values, a FieldRef of a KeyRef, in a session in another language, on a FlowField, FlowFilter or system field, on a temporary table or in another filter group.",
    awaiting: Awaiting {
        issues: &[9026],
        files: &["tests/al-language/recordref-fieldref/fieldref/SrcFilterBoolIntoText.al"],
        since: "2026-11-24",
    },
}
