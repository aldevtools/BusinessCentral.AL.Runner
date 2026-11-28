FieldRefValueExitIntoText => {
    id: "fieldref-value-exit-into-text",
    assumed: "On every supported major, in an en-US session, exit(FR.Value()) from a procedure that returns a Code or Text, for an Integer, BigInteger (the timestamp field 0 too), Decimal, Enum, Option, Boolean, Date or Duration field, gives the value's Format text, as FieldRef.Validate converts it: upper-cased into a Code, an Option's OptionCaption, an Enum member's Caption or else its name, a Decimal with its thousands separator, and StringLengthExceeded when the text is longer than the Code or a bounded Text. A parenthesized exit((FR.Value())) is the same.",
    tests: "Corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1629 (merged at 778b8fc1, not yet pinned), recordref-fieldref/fieldref-value-convert/FvcTimestampField.al:443 ExitValue_NumberFields, :466 ExitValue_EnumOptionFields, :485 ExitValue_OtherFields and :214 Field0_InsertedRow, measured on a local Business Central and on every supported major by the corpus CI; corpus PR https://github.com/aldevtools/BusinessCentral.AL.Language.Tests/pull/1641 (open), recordref-fieldref/fieldref-value-convert/FvcExitValueMore.al:142-282 ExitValue_* (Decimals of 1000 or more, an OptionCaption, an Enum member without a Caption, Text[3] and Text[30] returns), measured on a local Business Central and on every supported major by its CI; al-runner issue 9285, with the reporter's own measurement of the timestamp on a local Business Central.",
    probe: "Pin corpus PRs 1629 and 1641 with the next corpus bumps. Not measured: another field type (converts as before), and another session language or format region, or a major the corpus does not cover, where a number or an Enum refuses. A value in a Variant variable keeps its TypeConversion error, as FvcTimestampField.al variant_to_code measures.",
    awaiting: Awaiting {
        issues: &[9285, 9305],
        files: &[
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcTimestampField.al",
            "tests/al-language/recordref-fieldref/fieldref-value-convert/FvcExitValueMore.al",
        ],
        since: "2026-11-27",
    },
}
