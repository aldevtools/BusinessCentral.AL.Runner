TestpageFilterOptionNeNumber => {
    id: "testpage-filter-option-ne-number",
    assumed: "TestPage.Filter.GetFilter of an Option field the test filtered with '<>' and the first member's name (filter group 0) reads '<>0', on a top-level List or Worksheet the test opened with OpenView, over a temporary source. Another member after '<>' keeps the member's name.",
    tests: "No Microsoft test: al-runner issue 6924 (an outside reporter's suite filters '<>' after '=' on a search Worksheet).",
    probe: "Corpus PR 1392, testpage/filters/WorksheetEnumEqFilter.al: :450 and :456 (temporary Worksheet), :606 and :611 (temporary List), measured alike on BC 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/testpage/filters/WorksheetEnumEqFilter.al"],
        since: "2026-11-14",
    },
}
