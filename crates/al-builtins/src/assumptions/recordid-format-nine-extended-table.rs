RecordIdFormatNineExtendedTable => {
    id: "recordid-format-nine-extended-table",
    assumed: "Format(RecordId, 0, 9) in a language other than 1033 names a table as it does under 1033 (`[<namespace>.<name>]`, or the plain name outside a namespace) when the caption gate refuses the table's caption only because an app that ships a translation file of that language extends the table. Covered only for a table that declares a Caption (no CaptionML, no table extension's Caption over it) and whose own app ships no translation file of that language. Every other refused caption keeps format 9 refused, and format 1, which shows the caption, stays refused there.",
    tests: "No Microsoft test. An outside report: al-runner issue 8123, the reporter's repro (Unit of Measure, extended by an app with a da-DK file, under 1030: 2/2 tests pass on Business Central 29.0) and a partner app that formats the RecordId of a line of a Base Application table it extends in a Danish session.",
    probe: "Open corpus PR 1413 (tests/al-language/localization/app-xliff/lt-r7144-ext-caption/R7184BaseAppCaption.al:235-256, Item and Sales Invoice Header, which the app extends, without and with a cross-app unit for the table caption, under 1043; :260-282 two tables it does not extend; :286-301 its own table whose caption the file translates; :305-336 format 1 of the same tables gives the source caption). Measured with the same shapes on a local Business Central 29.0 OnPrem w1 container without a Microsoft language app.",
    awaiting: Awaiting {
        issues: &[8123],
        files: &[
            "tests/al-language/localization/app-xliff/lt-r7144-ext-caption/R7184BaseAppCaption.al::R8123_RecordIdFormat9_ExtendedBaseAppTables_Dutch_EnglishText",
        ],
        since: "2026-11-15",
    },
}
