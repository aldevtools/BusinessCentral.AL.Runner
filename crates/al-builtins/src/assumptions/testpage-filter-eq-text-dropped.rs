TestpageFilterEqTextDropped => {
    id: "testpage-filter-eq-text-dropped",
    assumed: "TestPage.Filter.SetFilter with a text that starts with '=' sets no filter, and leaves the field's earlier filter as it was, on a top-level List or Worksheet the test opened with OpenView, field with or without a control. The text after the '=': a member's name of an Enum or Option without captions (no Caption, CaptionML, OptionCaption or OptionCaptionML), written as one word of letters and digits; an Enum ordinal 0 or 1; the Option ordinal 1; a Boolean word. Measured cells, with no earlier filter on the field: temporary List: Enum and Option names, Option ordinal 1 (and the pinned Boolean words and Option first member); temporary Worksheet: Enum and Option names, Enum ordinals 0 and 1, Option ordinal 1, Boolean 1 and 0; database Worksheet: Enum names, Option names after the first member, Boolean 1; database List: only the pinned Boolean words. On the temporary Worksheet an Enum name after the test's own filter typed as an Enum name keeps that filter. Every other such text on a database source refuses; on a temporary source it keeps the filter, except the first member of an Option with captions, or written as more than one word, on a temporary List, which refuses (main drops it, not measured). The List with a Boolean word, or the temporary List with the Option's first member, and no earlier filter is pinned (testpage/filters/TempListOffPageBooleanFilter.al) and runs clean.",
    tests: "No Microsoft test: al-runner issue 6924 (an outside reporter's suite: a search Worksheet filtered with '=1' and '=<value>').",
    probe: "Corpus PR 1392, testpage/filters/WorksheetEnumEqFilter.al: :398, :425, :442, :463, :480, :513, :545 (temporary Worksheet), :575, :598 (temporary List), :620 (database Worksheet), measured alike on the two newest BC majors on a local BC; the probe PR (merged, not yet in the pinned corpus) passed its CI on all three BC majors the corpus runs.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/testpage/filters/WorksheetEnumEqFilter.al"],
        since: "2026-11-14",
    },
}
