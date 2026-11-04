TablefilterEnumMemberForms => {
    id: "tablefilter-enum-member-forms",
    assumed: "Evaluate into a TableFilter field stores an Enum condition by the member's caption (name in any casing, caption or ordinal), and lists, `<>` and ranges of Option or Enum members by their captions; a bad member, also in a list, gives the 'is not an option' error.",
    tests: "No Microsoft test; al-runner issue 6951. Measured on the newest major (Linux CI) by the probe below.",
    probe: "format-evaluate/tablefilter-neutral/TableFilterEnumForms.al on corpus branch al-runtime-rs-staging-r6951 (TableFilterEvaluate_EnumMemberForms, _EnumExpressions, _OptionExpressions, _EnumRoundTrip).",
    awaiting: Awaiting {
        issues: &[6951],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/TableFilterEnumForms.al"],
        since: "2026-11-04",
    },
}
