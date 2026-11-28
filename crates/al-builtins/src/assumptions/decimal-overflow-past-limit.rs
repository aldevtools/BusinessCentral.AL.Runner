DecimalOverflowPastLimit => {
    id: "decimal-overflow-past-limit",
    assumed: "A Decimal `+`, `-`, `*` or `/` (also `+=`, `-=`, `*=`, `/=`, into a variable, a record field or an argument) whose exact result is past the largest .NET Decimal raises 'Value was either too large or too small for a Decimal.' (AL:Overflow) before the target is assigned, so the target keeps its value. Only these shapes (al-builtins `ops/decimal_overflow.rs` `measured_overflow`): `+` and `-` of two Decimals, `*` of a Decimal and an Integer on either side, of two Decimals and of a Decimal and a BigInteger on the right, `/` of two Decimals; every operand of 18 significant digits or fewer; the exact result's size at least twice 79228162514264337500000000000 for `+`, `-` and `/`, and at least 79228162514264337600000000000 for `*`, of either sign. Main's refusal: a result past the .NET maximum below those sizes (79228162514264337500000000000 + 93543950336, a quotient just past the limit), an operand of more than 18 digits, a BigInteger on the left of a product, and every other type pair.",
    tests: "No Microsoft test. Corpus pull request 1630 (merged as 3eeda76e0ba1f28e7da2d23961175c675bdde053, green on every supported major), tests/al-language/types/decimal-integer/DecimalOverflowCells.al: codeunit 73591 Add_Cells1-3, Multiply_Cells1-3, Convert_Cells1 (`bigmax_times_1e10`), Convert_Cells3 (`literal_29digits_times_10`), TargetAfterError_Cells; codeunit 73592 Division_Cells. Pinned, and run clean: the largest Decimal literal (it holds 79228162514264337500000000000, json/JsonValueSetters.al:193) times the Integer 10 (errors/PnBErrorsCollections.al:564-566).",
    probe: "Pinned by the corpus bump that includes corpus pull request 1630. Still to measure: a result past the .NET maximum below the sizes above, a negative quotient, a BigInteger on the left of a product, an Integer, BigInteger or Duration beside a Decimal in a quotient.",
    awaiting: Awaiting {
        issues: &[9267],
        files: &["tests/al-language/types/decimal-integer/DecimalOverflowCells.al"],
        since: "2026-11-28",
    },
}
