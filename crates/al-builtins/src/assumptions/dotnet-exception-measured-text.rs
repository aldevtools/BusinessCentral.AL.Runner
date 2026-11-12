DotnetExceptionMeasuredText => {
    id: "dotnet-exception-measured-text",
    assumed: "A .NET exception of class ArgumentNullException, FormatException, InvalidOperationException or ArgumentOutOfRangeException, thrown by a DotNet method called on an object or statically, by a property get on an object, or by a constructor, is an AL error with the text `A call to <Type>.<Member> failed with this message: <.NET message>` (a constructor: the type only) and the code `DotNetInvoke:<class without Exception>`, for any such member and type of these bindings when the type name is plain (no generic or nested type), the member is written in its .NET letter case and the exception has no inner exception. GetLastErrorObject is then a NavNCLDotNetInvokeException (Message = the text) whose InnerException has the .NET class and message. Other bindings (a setter, an indexer, a property reached through a call, a static property, a boxed value) keep the refusal; ToString, ErrorInfo and other members of the exception objects stay refused.",
    tests: "Corpus PR 1365 (open), tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al: four calls (Uri.IsBaseOf, Convert.ToInt32, Uri.AbsolutePath, new StringBuilder) on every BC major the corpus CI runs. al-runner issue 7924 (an outside report).",
    probe: "DotNetExceptionOutcome.al (the cells above). Not measured: other members and types of the same four classes, the text of a member written in another letter case, an exception with an inner exception.",
    awaiting: Awaiting {
        issues: &[7924],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al"],
        since: "2026-11-12",
    },
}
