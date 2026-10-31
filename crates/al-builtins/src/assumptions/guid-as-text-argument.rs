GuidAsTextArgument => {
    id: "guid-as-text-argument",
    assumed: "A Guid passed where a function wants a Text is its text form {XXXXXXXX-...}, as in an assignment. Pinned for CopyStr, StrLen, UpperCase, LowerCase, StrPos, Notification.SetData, StrSubstNo's format, DelStr, PadStr, ConvertStr, IncStr, SelectStr, InsStr, DelChr, Contains, IndexOf, StartsWith, Replace and a procedure's Text and Code parameter (types/lt-guid-as-text-argument/GuidAsTextArgument.al); the other text parameters that read through the shared text argument are assumed alike.",
    tests: "Match General Jnl Lines UT (CopyStr(CreateGuid(), 1, 50)); 61 Tests-General Journal tests. Error Message Handler.ShowNotification (Notification.SetData with a Guid); 27 Tests-ERM tests.",
    probe: "A Guid passed to the Text parameters not in GuidAsTextArgument.al (for example Text.EndsWith, Text.Split, Text.LastIndexOf, TextBuilder.Append).",
}
