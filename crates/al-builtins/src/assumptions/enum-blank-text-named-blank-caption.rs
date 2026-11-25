EnumBlankTextNamedBlankCaption => {
    id: "enum-blank-text-named-blank-caption",
    assumed: "`Evaluate` of a text of blanks only (also the empty text) into an Enum without a translation file, in which no member has a blank name and exactly one member has a name and a blank Caption, names that member when it is the first member, its Caption is `' '` or `''` and GlobalLanguage is 1033, 1034 or 2067 (with the result used, and as a statement for `' '` only), or when it is the last member, its Caption is `''`, GlobalLanguage is 1033 and the result is used (al-builtins `evaluate/enum_text.rs`). A TAB, NUL or other white space, two such members, a middle one, a member with a blank name beside it, a translation file and every other language and form refuse.",
    tests: "No Microsoft test. The first member: corpus PR 1306 (merged on the corpus master, not yet in the pinned corpus), format-evaluate/EvalNamedBlankCaption.al:208-211, :224, :383-385 (Enum variables under GlobalLanguage 1034, 2067 and 1033). The last member: the outside reporter's repro of al-runner issue 8356 on BC 28.4 and 29.0 (`value(10; Blank) { Caption = ''; }`, `''` gives true and member 10).",
    probe: "Corpus PR 1577 (open), format-evaluate/EvalOptionBlankCaptions.al OBC_Enum_Last, OBC_Enum_SpaceMid, OBC_Enum_TwoBlank, OBC_Enum_EmptyMid and OBC_Enum_Field; corpus PR 1306 (merged), EvalNamedBlankCaption.al.",
    awaiting: Awaiting {
        issues: &[8354, 8356],
        files: &["tests/al-language/format-evaluate/EvalNamedBlankCaption.al", "tests/al-language/format-evaluate/EvalOptionBlankCaptions.al"],
        since: "2026-11-25",
    },
}
