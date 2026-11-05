TableExtensionFieldCaptionTranslated => {
    id: "table-extension-field-caption-translated",
    assumed: "The caption of a field that a table extension adds to a table of another app is the target of the unit `TableExtension <extension> - Field <field> - Property Caption` of the extension app's translation file for the session's language (FieldCaption, FieldRef.Caption); without a unit it keeps the source. An extension of a table of its own app, CaptionML, CaptionClass and a table another translating app extends stay refused.",
    tests: "No Microsoft test: an outside report (al-runner issue 7144: report captions read with FieldRef.Caption of a field an app adds to Sales Invoice Header, under each Windows Language).",
    probe: "Pinned by R7144ExtFieldCaption.al:68-145 (corpus PR 1156, merged to master), measured on the 27.5, 28.5 and 29.0 CI containers: retire when the pin includes it.",
    awaiting: Awaiting {
        issues: &[7144],
        files: &["tests/al-language/localization/app-xliff/lt-r7144-ext-caption/R7144ExtFieldCaption.al"],
        since: "2026-11-05",
    },
}
