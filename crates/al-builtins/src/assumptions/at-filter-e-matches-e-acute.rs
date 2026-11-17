AtFilterEMatchesEAcute => {
    id: "at-filter-e-matches-e-acute",
    assumed: "A case-insensitive `@` filter on a Text field matches the letter `É` (U+00C9) of the value with the letter `e` or `E` of the filter (`@emile` finds `Émile`) (al-builtins `filter::like`). Measured on a normal record only. Not covered, and refused by a read of the store (al-data `memory/held_cols.rs`) or a filter check outside it (`FilterExpr::matches`) when the answer with the accents ignored differs from main's: a Code field, `é` in the value, `é` or `É` in the filter against `e` in the value, every other accented letter. Refused: `É` in a Text value of a temporary record, and through `FilterExpr::matches`, which does not know the record kind. Not covered, main's answer: a filter without `@`, the rows of a virtual table, rows served by a restored database.",
    tests: "No Microsoft test. An outside report measured on a W1 database of the reporter (en-US): `SetFilter(Name, '@emile')` finds the row `Émile`; `@alpha*` finds `Alpha` and `alpha two` (al-runner issue 8196).",
    probe: "Corpus PR 1429 (draft, observe form), record/filters/FilterCollationNordic.al: `@` filters against values with `É`, `é`, `È`, `Ä`, `ä`, `Æ`, `ø` and the filter letter with and without its accent, with and without wildcards, on a Text and a Code field, on every supported major.",
    awaiting: Awaiting {
        issues: &[8196],
        files: &["tests/al-language/record/filters/FilterCollationNordic.al"],
        since: "2026-11-17",
    },
}
