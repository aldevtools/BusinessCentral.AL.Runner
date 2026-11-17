OptionFilterNonMemberError => {
    id: "option-filter-non-member-error",
    assumed: "Record.SetFilter in AL code on an Option field, with no placeholder argument and the filter text as written one word of ASCII letters that is no member, raises `The filter \"<word>\" is not valid for the <field> field on the <table> table. '<word>' is not an option. The existing options are: <members joined by commas>` (code Filter:CSide), as the corpus pins for a quoted blank. Only under GlobalLanguage 1033, on an Option whose members are all non-blank and that has no OptionCaption or OptionCaptionML. Every other text (blanks, operators, alternatives, placeholders), FieldRef.SetFilter, SetView, an Enum field and other languages keep main's refusal.",
    tests: "Corpus PR 1428 (open), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterNonMemberOptionEnum.al:132, :149, :166, :180 (the error under 1033; alternatives, ranges, <> and OptionCaption are measured too but the runtime refuses them). Before that: Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8197, shape 1): Option field Kind (Alpha, Beta, Gamma), SetFilter(Kind, 'Delta') raises `The filter \"Delta\" is not valid for the Kind field on the RF Header table. 'Delta' is not an option. The existing options are: Alpha,Beta,Gamma`.",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterNonMemberOptionEnum.al. Not measured: FieldRef.SetFilter, Option values that contain digits.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterNonMemberOptionEnum.al"],
        since: "2026-11-17",
    },
}
