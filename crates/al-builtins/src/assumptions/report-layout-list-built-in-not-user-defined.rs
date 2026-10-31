ReportLayoutListBuiltInNotUserDefined => {
    id: "report-layout-list-built-in-not-user-defined",
    assumed: "\"User Defined\" (column 14) of a Report Layout List row of an RDLC or Word layout declared in AL is No; a report without layouts has no rows. Excel and Custom layouts stay refused.",
    tests: "No Microsoft test: an outside report (ForNAV Rep Lay Sel Core.GetBuiltInLayout, reached from an app's install code; it runs on the reporter's real-BC run).",
    probe: "Pinned on corpus master by platform/lt-ug-report-layout-list/UgReportLayoutList.al (corpus PR 805), but the runtime does not run ReportLayoutList_AllColumns_RdlcLayout and _WordLayout yet (CaptionML of Report Layout List is not pinned): run them, then retire. Still wanted: an Excel and a Custom layout.",
}
