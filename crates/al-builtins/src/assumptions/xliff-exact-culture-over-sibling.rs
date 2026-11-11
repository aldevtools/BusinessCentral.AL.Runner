XliffExactCultureOverSibling => {
    id: "xliff-exact-culture-over-sibling",
    assumed: "When an app ships a translation file for exactly the session language's culture, that file decides whether or not it has the unit, even though a file of the same language for another culture has the unit: its target is read when it has the unit (nl-BE and nl-NL files under 2067: the nl-BE target), and the source text stays when it does not (de-DE without the unit, de-AT with it, under 1031: the source). The exact file alone is pinned by localization/app-xliff/TaXliff.al:170.",
    tests: "No Microsoft test. Kept from the runtime's earlier behavior for app folders; a first 29.0 run of the corpus probe showed the exact file's text (`nlbe_both`).",
    probe: "localization/app-xliff/TlTranslationCells.al TL_DependencyLabels_DutchCultures (`nlbe_both`, `nlnl_both`).",
    awaiting: Awaiting {
        issues: &[],
        files: &["tests/al-language/localization/app-xliff/TlTranslationCells.al"],
        since: "2026-11-11",
    },
}
