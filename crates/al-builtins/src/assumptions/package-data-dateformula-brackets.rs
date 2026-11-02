PackageDataDateformulaBrackets => {
    id: "package-data-dateformula-brackets",
    assumed: "A DateFormula that NavApp.LoadPackageData reads from a data file as a text with angle brackets (`<CM+1D>`) is held like the formula `<<CM+1D>>`, so Format with the standard format 9 prints the text as the file has it (install/lt-y85-package-data/Y85PackageData.al, Y85_Types_DurationGuidOptionEnumDateFormulaBlob, T1). CalcDate with it and Format with the other standard formats are not pinned.",
    tests: "No Microsoft test: an outside report about package data files, an ISV app's package data file.",
    probe: "A probe that loads a data file with a DateFormula text `<CM+1D>` and reads it with CalcDate and with Format(..., 0), (..., 1), (..., 2), (..., 9), and the same for the text `1D`.",
}
