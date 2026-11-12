DotnetExceptionOtherClass => {
    id: "dotnet-exception-other-class",
    assumed: "A .NET exception of any class whose text the runtime does not pin, thrown by a DotNet method called on an object or statically, by a property get on an object, or by a constructor, is an AL error like the measured ones: Codeunit.Run answers false, asserterror catches it, a TryFunction answers false. Its text and code are not known, so GetLastErrorText, GetLastErrorCode and GetLastErrorObject refuse after it, and a test that ends on it is not run. Other bindings keep the refusal.",
    tests: "Corpus PR 1365 (open), tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al measures the error for four classes (every one becomes an AL error); older pins add UriFormat, Xml, Argument, ObjectDisposed, IO and NotSupported (WdTryFunctionDotNetException.al, AeXmlDocumentLoadErrors.al, Y44ZipCreateMode.al). al-runner issue 7924 (an outside report).",
    probe: "A .NET exception of another class (for example OverflowException, KeyNotFoundException, NullReferenceException) through the same four bindings, in Codeunit.Run, asserterror and a TryFunction.",
    awaiting: Awaiting {
        issues: &[7924],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al"],
        since: "2026-11-12",
    },
}
