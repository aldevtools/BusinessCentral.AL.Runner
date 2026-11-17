OptionFilterNonMemberError => {
    id: "option-filter-non-member-error",
    assumed: "Record.SetFilter called in AL code with the field and the filter text only (no placeholder argument), on a record that is not temporary, in filter group 0, on an Option field, the filter text as written one word of ASCII letters that is no member and no prefix of a member, raises `The filter \"<word>\" is not valid for the <field> field on the <table> table. '<word>' is not an option. The existing options are: <members joined by commas>` (code Filter:CSide), as the corpus pins for a quoted blank. Only under GlobalLanguage 1033, on an Option whose members are all non-blank and that has no OptionCaption or OptionCaptionML. Every other text (blanks, operators, alternatives, numbers), caller and shape, an Enum field and other languages keep main's answer.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterNonMemberOptionEnum.al:132-146 (Delta raises under 1033), in the call shape of :47-57; alternatives, ranges, <>, -1, 1030 (:149-163) and OptionCaption (:166-193) are measured too and the runtime does not answer them. Before that: Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8197, shape 1): Option field Kind (Alpha, Beta, Gamma), SetFilter(Kind, 'Delta') raises `The filter \"Delta\" is not valid for the Kind field on the RF Header table. 'Delta' is not an option. The existing options are: Alpha,Beta,Gamma`.",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterNonMemberOptionEnum.al. Not measured: FieldRef.SetFilter, SetView, placeholders, a temporary record, another filter group, a prefix of a member.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterNonMemberOptionEnum.al"],
        since: "2026-11-17",
    },
}
