SecurityFilteringFlow => {
    id: "security-filtering-flow",
    assumed: "A record keeps its own SecurityFiltering value through a TransferFields into it and a RecordRef.SetTable into it. A by-value parameter (also a temporary one) reads the argument's declared value, not its current value, and Clear in the callee goes back to that declared value; the caller keeps its current value. Clear, Reset, Init, assignment, Copy and var parameters are pinned (al-language/reports/runtime/ACAttributes.al:355-414).",
    tests: "Base Application tests that read SecurityFiltering() of a record after a TransferFields into it: codeunit 137409 (CalcAnalysisReportValueUnitPrice, DrillDownInventoryAnalysisReport*), codeunit 137400 SalesUnitPriceFromCustomerPriceLineGroup, codeunit 134331 PurchaseLineFactboxUnitCost*V16.",
    probe: "al-language/reports/runtime/SFSecurityFilteringFlow.al (corpus PR 1310, measured the same on every supported major): SF_TransferFields, SF_RecordRef, SF_ParameterClear and SF_CodeunitRunTemporaryVariant.",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/reports/runtime/SFSecurityFilteringFlow.al"],
        since: "2026-11-11",
    },
}
