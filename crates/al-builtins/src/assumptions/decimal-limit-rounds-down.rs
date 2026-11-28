DecimalLimitRoundsDown => {
    id: "decimal-limit-rounds-down",
    assumed: "A Decimal whose 18-digit round-up (half away from zero) would pass the largest .NET Decimal holds 79228162514264337500000000000 with its sign, the largest Decimal of 18 significant digits, with no error. Only these shapes (al-builtins `ops/decimal_overflow.rs`): a Decimal literal from 79228162514264337550000000000.0 up to the .NET maximum (the literal of the .NET maximum itself is pinned, json/JsonValueSetters.al:161-165, :193, and runs clean), and the sum or difference of two Decimals of 18 significant digits or fewer whose exact result lies from 79228162514264337550000000000 up to the .NET maximum, either sign. A product there and a sum with an operand of more than 18 digits keep main's value.",
    tests: "No Microsoft test. Corpus pull request 1630 (merged as 3eeda76e0ba1f28e7da2d23961175c675bdde053, green on every supported major), tests/al-language/types/decimal-integer/DecimalOverflowCells.al: codeunit 73592 Boundary_Cells (`plus_50000000000`, `plus_93543950335`, `literal_19_digits`, `negative_minus_50000000000`).",
    probe: "Pinned by the corpus bump that includes corpus pull request 1630. Still to measure: a product and a quotient whose round-up passes the .NET maximum, a sum with a BigInteger operand there.",
    awaiting: Awaiting {
        issues: &[9267],
        files: &["tests/al-language/types/decimal-integer/DecimalOverflowCells.al"],
        since: "2026-11-28",
    },
}
