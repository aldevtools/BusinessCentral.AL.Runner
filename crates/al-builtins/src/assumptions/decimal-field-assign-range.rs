DecimalFieldAssignRange => {
    id: "decimal-field-assign-range",
    assumed: "The assignment `Rec.Field := X` of a Decimal outside -999,999,999,999,999.99 .. 999,999,999,999,999.99 to a stored Decimal field of a record that is not temporary raises 'The value of Decimal parameter  is outside of the permitted range. The current value is: <X>. The permitted range is: from -999,999,999,999,999.99 to 999,999,999,999,999.99.' (OutsidePermittedRange), the text Format gives (al-builtins `ops/decimal_overflow.rs` `field_assignment_range`). Only the `:=` statement into a field (also the first half of `Rec.Field += X`, which reads the field the assignment set). Main's store: a value whose range error text no pin shows (below 1,000,000,000,000,000 once kept to 18 digits), a temporary record, a FlowField, and every other writer (Validate, FieldRef.Value, Init, TransferFields, a builtin's write-back).",
    tests: "No Microsoft test. Corpus pull request 1630 (merged as 3eeda76e0ba1f28e7da2d23961175c675bdde053, green on every supported major), tests/al-language/types/decimal-integer/DecimalOverflowCells.al: codeunit 73592 FieldRange_Cells (`field_assign_max`, `field_assign_1e15` raise, `field_assign_below_1e15` is taken), codeunit 73591 Add_Cells3 (`field_plusassign_max_max`, `field_minusassign_negmax_max`).",
    probe: "Pinned by the corpus bump that includes corpus pull request 1630. Still to measure: a temporary record, Validate, FieldRef.Value and the other writers, and a value just past the range (999,999,999,999,999.995).",
    awaiting: Awaiting {
        issues: &[9267],
        files: &["tests/al-language/types/decimal-integer/DecimalOverflowCells.al"],
        since: "2026-11-28",
    },
}
