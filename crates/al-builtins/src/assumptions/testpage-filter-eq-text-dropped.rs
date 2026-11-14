TestpageFilterEqTextDropped => {
    id: "testpage-filter-eq-text-dropped",
    assumed: "TestPage.Filter.SetFilter with a text that starts with '=' sets no filter, and leaves the field's earlier filter as it was, on a top-level List or Worksheet the test opened with OpenView, field with or without a control: an Enum with a member's name or the ordinal 0 or 1 after the '=', an Option with a member's name or the ordinal 1, a Boolean with 1, 0, Yes, No, true or false. Only on a field the test has not filtered yet, except on a temporary Worksheet with an Enum field (an earlier filter there stays). On a temporary source every such shape; on a database Worksheet only names and Boolean words (an ordinal refuses); on a database List only the pinned Boolean words (every other shape refuses). The List with a Boolean word, or the temporary List with the Option's first member, and no earlier filter is pinned (testpage/filters/TempListOffPageBooleanFilter.al) and runs clean.",
    tests: "No Microsoft test: al-runner issue 6924 (an outside reporter's suite: a search Worksheet filtered with '=1' and '=<value>').",
    probe: "Corpus PR 1392, testpage/filters/WorksheetEnumEqFilter.al: :398, :425, :442, :463, :480, :513, :545 (temporary Worksheet), :575, :598 (temporary List), :620 (database Worksheet), measured alike on BC 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/testpage/filters/WorksheetEnumEqFilter.al"],
        since: "2026-11-14",
    },
}
