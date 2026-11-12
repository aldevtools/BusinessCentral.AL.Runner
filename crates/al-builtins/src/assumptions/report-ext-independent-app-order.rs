ReportExtIndependentAppOrder => {
    id: "report-ext-independent-app-order",
    assumed: "The report extensions of two apps with no dependency path between them apply in the order of the app ids, where the app names sort the same way (dataset, triggers, Report.WordXmlPart). Pinned for one pair only, app B before app D (reports/extensions/IndependentAppsReportExtensions.al:93-102, :118-127), which the app id, the app name, the publisher (descending), the publish order (descending) and the SQL order of the app id (descending) all predict.",
    tests: "No Microsoft test. An outside report: two report extensions from apps that do not depend on each other, reached through Report.WordXmlPart, pass on real BC (al-runner issue 7358).",
    probe: "Queued as corpus PR 1338, reports/extensions/IndependentAppsOrderRule.al: one report of app A extended by apps B, D, E and F, whose app id, app name, publisher, publish order and SQL order of the app id each predict another order; the column order of the dataset tells the rule.",
    awaiting: Awaiting {
        issues: &[7358],
        files: &["tests/al-language/reports/extensions/IndependentAppsOrderRule.al"],
        since: "2026-11-12",
    },
}
