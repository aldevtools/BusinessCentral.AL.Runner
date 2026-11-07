AllobjwithcaptionTabledataColumns => {
    id: "allobjwithcaption-tabledata-columns",
    assumed: "The TableData row of a table in AllObjWithCaption has the Table row's App Package ID, App Runtime Package ID, App ID and AL Namespace, as AllObj's TableData row has them (metadata/session-system/AllObjNamespace.al:220-262). Its existence, name, caption and subtype are pinned (:156-218).",
    tests: "No Microsoft test: an outside report, an ISV app's install code that validates a setup field reading AllObjWithCaption for TableData 5050; metadata/CmInstallObjects.al counts the own objects of AllObjWithCaption by App Package ID.",
    probe: "Corpus writer Y64: metadata/Y64InstallLeads.al (the AllObj and AllObjWithCaption TableData rows, read during the per-company and per-database install), pinned by corpus pin 4b6013d0, but this runtime refuses its four tests: the install code calls NavApp.LoadPackageData. Retire once they run.",
}
