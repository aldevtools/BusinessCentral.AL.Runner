NametableAssignIsCall => {
    id: "nametable-assign-is-call",
    assumed: "`NsMgr.NameTable := Doc.NameTable` on an XmlNamespaceManager is the call `NsMgr.NameTable(Doc.NameTable())` (the call form is pinned; alc accepts the assignment form).",
    tests: "No Microsoft test: al-runner issue 6526, the reporter's repro (alc compiles it; the reporter's production code runs it on real BC). Corpus writer Y29 measured on the SaaS sandbox that both forms select the same nodes.",
    probe: "Corpus writer Y29: the assignment form of method-like members (XmlNamespaceManager.NameTable and the other members alc accepts).",
    awaiting: Awaiting {
        issues: &[6526],
        files: &["tests/al-language/xml/XmlXPathNamespaces.al::Y29_NameTable_AssignForm_SelectsLikeCallForm"],
        since: "2026-10-31",
    },
}
