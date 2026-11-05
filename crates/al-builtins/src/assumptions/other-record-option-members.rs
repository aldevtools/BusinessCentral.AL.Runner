OtherRecordOptionMembers => {
    id: "other-record-option-members",
    assumed: "A page control bound to an Option field of another record variable shows the member name before the OptionCaption gate where that is confirmed on Windows; on a List page that shows no row every control bound to a field of another record variable reads blank.",
    tests: "No Microsoft test: al-runner issue 7005 (a partner app's test; the reporter's Windows run shows the member name, and a blank value without rows).",
    probe: "pages/controls/OtherRecordOptionMembers.al on corpus branch al-runtime-rs-staging-r6945, measured on the corpus CI legs of all three supported majors.",
    awaiting: Awaiting {
        issues: &[7005],
        files: &["tests/al-language/pages/controls/OtherRecordOptionMembers.al"],
        since: "2026-11-04",
    },
}
