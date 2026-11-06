TablefilterCaptionClassDeclared => {
    id: "tablefilter-caption-class-declared",
    assumed: "Evaluate into a TableFilter field finds a field with a literal CaptionClass by its name and stores it by its declared caption (the Caption property, else the English CaptionML, else its name), the text the virtual Field table gives as \"Field Caption\"; any other text that names no field (also the caption the CaptionClass resolves to) is an unknown field. Pinned: `Nominal Price=>=40` selects the priced row through the Base Application page Table Filter, and the resolved caption `Displayed Price` gives the unknown-field error (format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:539 and :551). The stored text, a numeric caption area and a text other than the resolved caption are not pinned.",
    tests: "No Microsoft test: al-runner issue 7164 (the reporter's real-BC run of TableFilter_Evaluate_CaptionClassFieldName_SelectsPricedRow and TableFilter_Evaluate_ResolvedCaptionClass_RejectsUnknownField on the three supported majors, corpus pull request 1157).",
    probe: "format-evaluate/tablefilter-neutral/R7164CaptionClassStored.al: what Format gives back after Evaluate of a CaptionClass field by its name (resolved and unresolved, a numeric and a non-numeric area), whether its declared Caption is accepted as the field, and what the virtual Field table holds as its Field Caption.",
    awaiting: Awaiting {
        issues: &[7164],
        files: &["tests/al-language/format-evaluate/tablefilter-neutral/R7164CaptionClassStored.al"],
        since: "2026-11-06",
    },
}
