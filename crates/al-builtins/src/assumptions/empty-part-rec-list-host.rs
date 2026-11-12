EmptyPartRecListHost => {
    id: "empty-part-rec-list-host",
    assumed: "A ListPart that shows no row, in the FactBoxes area of a top-level List page opened for edit and moved by GoToRecord, linked by one const(..) (or a filter(..) of one value, which reads the same) and one field(..) on key fields of the part (no SubPageView, Provider, SourceTableView, PopulateAllFields or OnNewRecord), holds the link's values on Rec for its action, as on a Card or Document host (pinned there by pages/parts/EmptyPartRecLink.al).",
    tests: "No Microsoft test. An outside report (al-runner issue 6924, comment 6822249976): AttachedDocumentsDrillDownOnBodyTranslations opens a List page with Base Application page 1178 \"Doc. Attachment List Factbox\" in its FactBoxes (SubPageLink = \"Table ID\" = const(..), \"No.\" = field(..)) with OpenEdit, GoToRecord, then invokes OpenInDetail, which reads Rec.\"Table ID\"; it passes on BC 29.",
    probe: "pages/parts/EmptyPartRecLinkList.al: List hosts, FactBox and content area, OpenEdit and OpenView, as opened, after GoToRecord and after Next, const, field and filter links (corpus pull request 1339).",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/pages/parts/EmptyPartRecLinkList.al"],
        since: "2026-11-12",
    },
}
