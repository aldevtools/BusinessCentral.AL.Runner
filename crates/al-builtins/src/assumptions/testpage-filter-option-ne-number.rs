TestpageFilterOptionNeNumber => {
    id: "testpage-filter-option-ne-number",
    assumed: "TestPage.Filter.GetFilter of an Option field without OptionCaption or OptionCaptionML reads '<>0' when the test typed '<>' and the first member's name and that filter (filter group 0) is still the field's, on a top-level List or Worksheet the test opened with OpenView, over a temporary source. A filter page code wrote, '<>0' typed as a number, and another member after '<>' keep the text main gives.",
    tests: "No Microsoft test: al-runner issue 6924 (an outside reporter's suite filters '<>' after '=' on a search Worksheet).",
    probe: "Corpus PR 1392, testpage/filters/WorksheetEnumEqFilter.al: :450 and :456 (temporary Worksheet), :606 and :611 (temporary List), measured alike on the two newest BC majors on a local BC; the probe PR (merged, not yet in the pinned corpus) passed its CI on all three BC majors the corpus runs.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/testpage/filters/WorksheetEnumEqFilter.al"],
        since: "2026-11-14",
    },
}
