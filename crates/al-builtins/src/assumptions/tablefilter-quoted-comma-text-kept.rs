TablefilterQuotedCommaTextKept => {
    id: "tablefilter-quoted-comma-text-kept",
    assumed: "Evaluate into a TableFilter keeps a Text value in double quotes that holds a comma, `\"A,B\"`, as written, so Format gives `Text Field=\"A,B\"` back. Pinned: the stored text, fed to the Base Application page Table Filter and FieldRef.SetFilter, selects the row whose text is `A,B` (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:709 TableFilter_Evaluate_QuotedTextComma_SelectsLiteralCommaRow). The stored text itself and other quoted Text values are not pinned.",
    tests: "No Microsoft test: al-runner issue 7188 (the reporter's real-BC run of TableFilter_Evaluate_QuotedTextComma_SelectsLiteralCommaRow on the three supported majors, corpus pull request 1165).",
    probe: "format-evaluate/tablefilter-stored/TableFilterStoredQuotedText.al: a Text case with a quoted value that holds a comma, `Txt=\"A,B\"`: what Evaluate stores and Format gives back.",
    awaiting: Awaiting {
        issues: &[7188],
        files: &["tests/al-language/format-evaluate/tablefilter-stored/TableFilterStoredQuotedText.al"],
        since: "2026-11-07",
    },
}
