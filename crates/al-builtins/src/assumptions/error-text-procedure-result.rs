ErrorTextProcedureResult => {
    id: "error-text-procedure-result",
    assumed: "Error(Text) where the text is the Text or Code result of an AL procedure: GetLastErrorText(true) answers the text of a value ('Use ERROR with a text constant to improve telemetry details'), whatever the procedure returns. Pinned for a procedure that returns a global Label as text (errors/last-error/LastErrorTextExcludeLabel.al ExcludeFlag_LabelHandedOnAsText, `proc_true`); assumed alike for a procedure that returns a literal, a variable or a built text.",
    tests: "No Microsoft test and no user report: the pinned shape is the only one the corpus holds.",
    probe: "LastErrorTextExcludeLabel.al with procedures that return a literal, a local Text variable, a concatenation and a StrSubstNo result, each raised through Error(Proc()), read with GetLastErrorText(true) and (false).",
}
