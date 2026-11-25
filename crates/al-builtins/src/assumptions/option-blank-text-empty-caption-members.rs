OptionBlankTextEmptyCaptionMembers => {
    id: "option-blank-text-empty-caption-members",
    assumed: "`Evaluate` of a text of blanks only (also the empty text) into an Option record field named directly, whose plain OptionCaption has as many entries as the field has members, where every member with a blank name or a blank caption is named `\"\"` with the caption `''` and none of them is the first member, with the result used, answers false and keeps the value (al-builtins `evaluate/option_blank.rs`). A TAB or other white space, the statement form, a FieldRef, OptionCaptionML, such a member in the first place, and every other mix of blank names and captions refuse.",
    tests: "No Microsoft test. The outside reporter's repro of al-runner issue 8354 on BC 28.4 and 29.0: `OptionMembers = First,Second,Third,,,Skip`, `OptionCaption = 'First one,Second one,Third one,,,Skip it'`, `''` and `' '` give false and the value stays First.",
    probe: "Corpus PR 1577 (open), format-evaluate/EvalOptionBlankCaptions.al OBC_Option_Reporter and OBC_Option_FirstEmpty: blank texts into Option fields with members `\"\"` of caption `''`.",
    awaiting: Awaiting {
        issues: &[8354, 8356],
        files: &["tests/al-language/format-evaluate/EvalOptionBlankCaptions.al"],
        since: "2026-11-25",
    },
}
