OptionFilterNegativeOneError => {
    id: "option-filter-negative-one-error",
    assumed: "Under GlobalLanguage 1033, `Record.SetFilter` called in AL code with the filter text `-1` and no placeholder argument, on an Option field whose members are all non-blank and that has no OptionCaption or OptionCaptionML, on a record that is not temporary, in filter group 0, raises `The filter \"-1\" is not valid for the <field> field on the <table> table. '-1' is not an option. The existing options are: <members>` with code Filter:CSide. Every other negative number, a blank around it, a negative number inside a longer filter, FieldRef.SetFilter, SetView, a temporary record, another filter group, an OptionCaption field and another language refuse. A placeholder argument keeps main's answer.",
    tests: "Corpus PR 1428 (open, commit 04e1fa0a), measured on a local BC of each of the three supported majors, the same on each: tests/al-language/record/filters/FilterNonMemberOptionEnum.al:132-146 (Option_NonMember_English1033: `[-1]` at :143 raises with the member list Alpha,Beta,Gamma, code Filter:CSide).",
    probe: "Corpus PR 1428 (open), tests/al-language/record/filters/FilterNonMemberOptionEnum.al. Measured, not answered: the Danish text of 1030 (:160). Not measured: another negative number, other callers and shapes.",
    awaiting: Awaiting {
        issues: &[8197],
        files: &["tests/al-language/record/filters/FilterNonMemberOptionEnum.al"],
        since: "2026-11-17",
    },
}
