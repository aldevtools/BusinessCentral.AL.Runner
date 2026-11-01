AllobjwithcaptionTabledataColumns => {
    id: "allobjwithcaption-tabledata-columns",
    assumed: "The TableData row of a table in AllObjWithCaption has the Table row's App Package ID, App Runtime Package ID, App ID and AL Namespace, as AllObj's TableData row has them (metadata/lt-all-obj-namespace/AllObjNamespace.al:220-262). Its existence, name, caption and subtype are pinned (:156-218).",
    tests: "No Microsoft test: al-runner issue 6643, an ISV app's install code that validates a setup field reading AllObjWithCaption for TableData 5050; install/lt-cm-install-leads/CmInstallObjects.al counts the own objects of AllObjWithCaption by App Package ID.",
    probe: "Corpus writer Y64: install/lt-cm-install-leads/Y64InstallLeads.al (the AllObj and AllObjWithCaption TableData rows, read during the per-company and per-database install).",
    awaiting: Awaiting {
        issues: &[6643],
        files: &["tests/al-language/install/lt-cm-install-leads/Y64InstallLeads.al"],
        since: "2026-11-01",
    },
}
