RecordIdFormatOneSourceCaption => {
    id: "recordid-format-one-source-caption",
    assumed: "Format(RecordId, 0, 1) in a language other than 1033 names a table by its caption as its source declares it, untranslated, where the caption gate refuses that caption because an app that ships a translation file of that language could change it. Covered only for a table that declares a Caption (no CaptionML, no table extension's Caption over it) in two shapes: a table whose own app ships no file of that language and which an app that ships one extends, and a table of an app that ships one, which no extension extends and whose caption the file translates (TableCaption gives the translated text there). Every other refused caption keeps format 1 refused.",
    tests: "No Microsoft test. A repro of the format 1 issue: a table with Caption 'Repro Gadget' whose app's da-DK file translates it to 'Repro Dims'; under 1030 format 1 gives 'Repro Gadget: G1' and TableCaption 'Repro Dims' on Business Central 29.0 (2/2 pass).",
    probe: "Open corpus PR 1413 (tests/al-language/localization/app-xliff/lt-r7144-ext-caption/R7184BaseAppCaption.al:305-336 R8123_RecordIdFormat1_Dutch_SourceCaptions under 1043: Item and Sales Invoice Header, which the corpus app extends, give 'Item: R8123' and 'Sales Invoice Header: R8123'; its own table LT TA Item, whose nl-NL unit gives TableCaption 'LT TA Artikel', gives 'LT TA Item: R8123'). Green on the corpus CI of every supported major.",
    awaiting: Awaiting {
        issues: &[],
        files: &[
            "tests/al-language/localization/app-xliff/lt-r7144-ext-caption/R7184BaseAppCaption.al::R8123_RecordIdFormat1_Dutch_SourceCaptions",
        ],
        since: "2026-11-15",
    },
}
