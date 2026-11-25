OptionCaptionSameAsNamesOrdinal => {
    id: "option-caption-same-as-names-ordinal",
    assumed: "`Evaluate` without a format number, under GlobalLanguage 1033, into the Option field of an app's table, named directly, with two members whose names are letters only and a plain OptionCaption equal to the member names entry by entry (as `Alpha,Beta` with `'Alpha,Beta'`): with the result used, the texts `2`, `99`, `+2` and `2147483647` store that ordinal and give true, and the texts `-1` and `2147483648` give false and keep the value; as a statement, the text `99` stores 99 and the text `-1` raises `'-1' is not an option. The existing options are: ` and the member names joined by `,` (InvalidOptionString; `Alpha,Beta` for those members) (al-builtins `evaluate.rs` `option_field`, `evaluate/option_text.rs` `same_as_names_measured`). Every other text, member count, member name with a character other than a letter, platform table, form or language keeps the reading of a field without these captions.",
    tests: "No Microsoft test. The outside reporter's repro of al-runner issue 8355 on two BC majors: `2` gives true and 2, `99` true and 99, `-1` false and the value kept. The CI run of corpus PR 1577 on all three BC majors of corpus CI (run 45559748084), format-evaluate/EvalOptionBlankCaptions.al OBC_Option_PairOrdinals.",
    probe: "Corpus PR 1577 (open), format-evaluate/EvalOptionBlankCaptions.al OBC_Option_PairOrdinals: numbers past the members of an Option field whose OptionCaption repeats its member names.",
    awaiting: Awaiting {
        issues: &[8355],
        files: &["tests/al-language/format-evaluate/EvalOptionBlankCaptions.al"],
        since: "2026-11-25",
    },
}
