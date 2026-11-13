SetfilterEmptyOrSide => {
    id: "setfilter-empty-or-side",
    assumed: "Record.SetFilter on a Text, Code or Integer field whose whole filter is one of the shapes `V|`, `|V`, `V||V` or `|`, where each V is one literal of letters and digits or one placeholder with a non-empty value, raises Filter:CSide: 'The filter \"<filter as written>\" is not valid for the <field> field on the <table> table. The right side of '|' operators cannot be empty.' (`V|`, `V||V`) or '... The left side ...' (`|V`, `|`) (al-builtins filter.rs `empty_or_side`). The Integer literals `|10`, `10|` and the Text `||` are pinned by record/crud/RcAFilters.al:376, :378, :648 and not flagged. Every other shape (blanks, more terms, `&`, parentheses, wildcards, an empty placeholder value next to a `|`, other field types, FieldRef.SetFilter) keeps its old outcome.",
    tests: "No Microsoft test. Outside reports: al-runner issue 7764 item 2 (`SetFilter(Name, 'a||b')` in Microsoft's Data Search tests).",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (record/filters/SetFilterEmptyTerm.al:123-222): the same error on BC 27.5, 28.5 and 29.0 for every shape above on Text, Code and Integer fields, and an empty substituted value ('%1|%2' with '', '%1' with '') is a valid term.",
    awaiting: Awaiting {
        issues: &[7764],
        files: &["tests/al-language/record/filters/SetFilterEmptyTerm.al"],
        since: "2026-11-13",
    },
}
