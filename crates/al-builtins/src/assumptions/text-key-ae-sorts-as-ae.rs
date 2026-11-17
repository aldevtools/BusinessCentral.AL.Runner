TextKeyAeSortsAsAe => {
    id: "text-key-ae-sorts-as-ae",
    assumed: "In the key order of a Text field and in a range or comparison filter on a Text field against a Text value, the capital letter `Æ` (U+00C6) sorts as the two letters `AE` (`Ærø` between `a*b?` and `Alpha`, inside `'A'..'B'`); two texts that tie once `Æ` is written `AE` keep main's order (al-builtins `collation::compare_text_key_expanded`, by `collation::text_value_key_order`). Measured on a normal record only. Not covered, and refused by a read of the store whose answer rests on it (an order of two rows or more, a range or comparison filter; al-data `memory/held_cols.rs`): the small letter `æ`, a Code field, a Text value against a Code value, a temporary record. A filter check outside the store (`FilterExpr::matches`: FlowField filters, queries, relations, a record's own filters) does not know the record kind and refuses `Æ` and `æ` in a range or comparison. Not covered, main's answer: the rows of a virtual table, rows served by a restored database (not noted by the store), the AL operators `<` and `>` (they still refuse `Æ`), and every other letter without a weight (main's order by code point).",
    tests: "No Microsoft test. An outside report measured on a W1 database of the reporter (en-US, the default database collation), a normal table with a Text[50] field: `SetRange(Name, 'A', 'B')` finds `Ærø`, and `SetCurrentKey(Name)` puts it after `a*b?` and before `Alpha` (al-runner issue 8195).",
    probe: "Corpus PR 1429 (draft, observe form), record/filters/FilterCollationNordic.al: The order of `Æ` and `æ` against `AE`, `Ae`, `A` and `B` in the key of a Text and of a Code field, on a normal and on a temporary table, in SetRange, SetFilter ranges and `<`/`>` filters, on every supported major.",
    awaiting: Awaiting {
        issues: &[8195],
        files: &["tests/al-language/record/filters/FilterCollationNordic.al"],
        since: "2026-11-17",
    },
}
