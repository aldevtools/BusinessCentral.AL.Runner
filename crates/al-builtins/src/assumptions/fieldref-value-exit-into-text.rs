FieldRefValueExitIntoText => {
    id: "fieldref-value-exit-into-text",
    assumed: "On every supported BC major, in an en-US session, exit(FR.Value()) from a procedure that returns a Code or Text, for an Integer, BigInteger (the timestamp field 0 too), Decimal, Enum, Option, Boolean, Date or Duration field, gives the value's Format text as T := FR.Value() does: upper-cased into a Code, the Enum's and Option's caption, and StringLengthExceeded when the text is longer than the Code or Text.",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1629 (merged at 778b8fc1, not yet pinned), recordref-fieldref/fieldref-value-convert/FvcTimestampField.al:443 ExitValue_NumberFields, :466 ExitValue_EnumOptionFields, :485 ExitValue_OtherFields and :214 Field0_InsertedRow, measured on a local Business Central and on every supported major by the corpus CI; al-runner issue 9285, with the reporter's own measurement of the timestamp on a local Business Central.",
    probe: "Pin corpus PR 1629 with the next corpus bump. Not measured, so as before: another field type, another session language or format region, and a value in a Variant variable (which keeps its TypeConversion error).",
    awaiting: Awaiting {
        issues: &[9285],
        files: &["tests/al-language/recordref-fieldref/fieldref-value-convert/FvcTimestampField.al"],
        since: "2026-11-27",
    },
}
