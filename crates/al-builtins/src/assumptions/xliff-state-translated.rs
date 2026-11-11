XliffStateTranslated => {
    id: "xliff-state-translated",
    assumed: "A unit of a translation file whose `<target>` has `state=\"translated\"` gives its target, as a unit without a state does (pinned without a state by localization/app-xliff/TaXliff.al:170). Every unit of Microsoft's translation files has this state.",
    tests: "No Microsoft test. Outside reports: Base Application Labels under GlobalLanguage 2067 and 1031 read from the localized Base Application's translation file (al-runner issues 6924 and 7361); their tests pass on real BC.",
    probe: "localization/app-xliff/TlTranslationCells.al TL_UnitForms_Dutch `nlnl_state_translated` (and the other states of the same test).",
    awaiting: Awaiting {
        issues: &[6924, 7361],
        files: &["tests/al-language/localization/app-xliff/TlTranslationCells.al"],
        since: "2026-11-11",
    },
}
