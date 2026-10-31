CopyCompanyInSaas => {
    id: "copy-company-in-saas",
    assumed: "Database.CopyCompany of a company makes a new Company row (the destination's name, Evaluation Company as the source's, Display Name and Business Profile Id empty, a new Id and SystemId) and copies every per-company row of the source as it is, system fields included; tables that are not per company stay as they are; no trigger, table event or database trigger of a copied row runs; the Company's OnAfterInsertEvent runs once. Measured on SaaS BC 29 by corpus writer Y22 (session/lt-y22-copy-company/Y22CopyCompany.al), not yet in the pinned corpus. The two errors are pinned (lt-o-small-builtins/OSmallBuiltins.al:92-112).",
    tests: "No Microsoft test: al-runner issue 6507, a partner suite whose install code makes its second company with Base Application's report Copy Company.",
    probe: "Written: session/lt-y22-copy-company/Y22CopyCompany.al (the copied company and rows, the events, the return value, the names, a copy inside a write transaction, report Copy Company).",
    awaiting: Awaiting {
        issues: &[6507],
        files: &["tests/al-language/session/lt-y22-copy-company/Y22CopyCompany.al"],
        since: "2026-10-31",
    },
}
