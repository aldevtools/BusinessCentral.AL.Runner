SecurityFilteringFlow => {
    id: "security-filtering-flow",
    assumed: "A record keeps its own SecurityFiltering value through a TransferFields into it and a RecordRef.SetTable into it. A copy of a record that is a variable of its own reads the original's declared value, not its current value, and Clear on it goes back to that declared value; the original keeps its current value. The probe measures this for a by-value and a temporary parameter; the same rule is applied, unmeasured, to an event argument, a record handed to StartSession and a record assigned to a Variant. A RecordRef that took a record (GetTable, also through a Variant, or a conversion) reads Validated whatever the record's value. Clear, Reset, Init, assignment, Copy and var parameters are pinned (al-language/reports/runtime/ACAttributes.al:355-414).",
    tests: "Base Application tests that read SecurityFiltering() of a record after a TransferFields into it: codeunit 137409 (CalcAnalysisReportValueUnitPrice, DrillDownInventoryAnalysisReport*), codeunit 137400 SalesUnitPriceFromCustomerPriceLineGroup, codeunit 134331 PurchaseLineFactboxUnitCost*V16. Base Application posting: Sales-Post, Purch.-Post and Service-Post LockTables call GLEntry.GetLastEntryNo on a global declared [SecurityFiltering(SecurityFilter::Ignored)] (legacy posting with Automatic Cost Posting, set by many SCM and ERM tests), and Find Record Management.FindLastEntryIgnoringSecurityFilter reads RecRef.SecurityFiltering of the RecordRef it took through a Variant.",
    probe: "al-language/reports/runtime/SFSecurityFilteringFlow.al (corpus PR 1310, measured the same on every supported major): SF_TransferFields, SF_RecordRef, SF_ParameterClear and SF_CodeunitRunTemporaryVariant. Still to ask: an event argument and a record handed to StartSession whose current value differs from its declared one.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/reports/runtime/SFSecurityFilteringFlow.al"],
        since: "2026-11-11",
    },
}
