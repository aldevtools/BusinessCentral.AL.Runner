EmptyPartRecListHost => {
    id: "empty-part-rec-list-host",
    assumed: "A ListPart that shows no row, on a top-level List page standing on a stored row and opened for edit or for view, holds the single values of its SubPageLink on key fields on Rec (as on a Card or Document host, pinned by pages/parts/EmptyPartRecLink.al) when it lands as the host opens, after a GoToRecord that left the row or after Next, a part the List page declares itself, in the FactBoxes or the content area, linked by const(..), filter(..) and field(..) entries only (no field(filter(..)), SubPageView or Provider), with no SourceTableView, PopulateAllFields or OnNewRecord. Observed locally on one major only (corpus pull request 1339).",
    tests: "No Microsoft test. An outside report (al-runner issue 6924, comment 6822249976): AttachedDocumentsDrillDownOnBodyTranslations opens a List page with Base Application page 1178 \"Doc. Attachment List Factbox\" in its FactBoxes (SubPageLink = \"Table ID\" = const(..), \"No.\" = field(..)) with OpenEdit, GoToRecord, then invokes OpenInDetail, which reads Rec.\"Table ID\"; it passes on real BC (the reporter ran the newest major).",
    probe: "pages/parts/EmptyPartRecLinkList.al (corpus pull request 1339): 42 List-host cells, FactBox and content area, OpenEdit and OpenView, as opened, after GoToRecord and after Next, const, field, filter and range links, a host field outside the key; the other majors by the pull request run.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/pages/parts/EmptyPartRecLinkList.al"],
        since: "2026-11-12",
    },
}
