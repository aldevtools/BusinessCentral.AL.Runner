DateFormulaFormat2Brackets => {
    id: "date-formula-format-two-brackets",
    assumed: "Format(DateFormula, 0, 2) of a formula that is not empty is the formula as format 0 writes it, in angle brackets (`<-1W>`, `<+CY+7973Y>`). Formats 0 and 9 without brackets are pinned (misc/lt-system-functions/RandomFormat.al:484-505); format 2 of the empty formula and format 1 stay refused.",
    tests: "System Application Test, Retention Period Test (codeunit 138700): Test1WeekRetentionPeriodDateFormulaTranslation and the seven other *DateFormulaTranslation tests (Retention Period Impl. writes Format(PeriodDateFormula, 0, 2)).",
    probe: "Format(F, 0, 2) and Format(F, 0, 1) of '<-1W>', '<CM+1D>', '<+CY-24M>' and the empty formula, in 1033 and 1030.",
}
