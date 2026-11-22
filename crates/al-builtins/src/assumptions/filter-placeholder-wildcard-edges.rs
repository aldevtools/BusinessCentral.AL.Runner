FilterPlaceholderWildcardEdges => {
    id: "filter-placeholder-wildcard-edges",
    assumed: "`Record.SetFilter(Field, '<>%1', Value)` in AL code on a record that is not temporary, in filter group 0, under GlobalLanguage 1033, with `Value` a Text whose pattern is capital letters, digits and `-` followed by one `*` (al-vm `records/placeholder_wildcard_edge.rs`). With one blank before or after the pattern and the pattern no longer than the field: on a Code[n] field the filter is the pattern without the blank (`<>XYZ-*`); on a Text[n] field the filter is the pattern with the blank, read back in quotes (`<>'XYZ-* '`), and keeps every row without a blank at that end (a row with one is not measured and is read as the pattern). Without a blank and longer than a Code[n] field: the length error of the value. Not covered, and refused by the filter parser: these values through FieldRef.SetFilter, on a temporary record, in another filter group or language, with a Code argument, in a longer filter text, and every other wildcard text with a blank at one end or longer than a Code field. Not covered, main's answer: `%1` without `<>`, a blank inside the value (`XYZ- *`), a long value on a Text field.",
    tests: "No Microsoft test. An outside report (al-runner issue 8642) uses `SetFilter(Code, '<>%1', 'XYZ-*')`, which the pinned readers answer; the probe of corpus PR 1503 on three local BC versions measured its neighbors: a blank at one end and values longer than the field.",
    probe: "Corpus PR 1503 (open), record/filters/PlaceholderWildcardSuffix.al: `<>%1` with `XYZ-*` and its neighbors on a Code[20] and a Text[30] field, Record and FieldRef, GetFilter and the rows kept, three local BC versions.",
    awaiting: Awaiting {
        issues: &[8642],
        files: &["tests/al-language/record/filters/PlaceholderWildcardSuffix.al"],
        since: "2026-11-22",
    },
}
