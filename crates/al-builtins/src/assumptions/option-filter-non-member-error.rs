OptionFilterNonMemberError => {
    id: "option-filter-non-member-error",
    assumed: "SetFilter on an Option field with a word of letters that is no member (and no number) raises `The filter \"<word>\" is not valid for the <field> field on the <table> table. '<word>' is not an option. The existing options are: <members joined by commas>` (code Filter:CSide), as the corpus pins for a quoted blank. Only an Option whose members are all non-empty, under GlobalLanguage 1033; an Enum field, other texts and other languages keep main's refusal.",
    tests: "Outside reporter's measurement on a W1 BC of the second supported major, en-US region (al-runner issue 8197, shape 1): Option field Kind (Alpha, Beta, Gamma), SetFilter(Kind, 'Delta') raises `The filter \"Delta\" is not valid for the Kind field on the RF Header table. 'Delta' is not an option. The existing options are: Alpha,Beta,Gamma`.",
    probe: "Corpus PR 1428 (open, observe form), tests/al-language/record/filters/FilterNonMemberOptionEnum.al: SetFilter of an Option field and an Enum field with a non-member word, a word with digits, a member name with OptionCaption different from the member, under 1033 and 1030; also FieldRef.SetFilter.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterNonMemberOptionEnum.al"],
        since: "2026-11-17",
    },
}
