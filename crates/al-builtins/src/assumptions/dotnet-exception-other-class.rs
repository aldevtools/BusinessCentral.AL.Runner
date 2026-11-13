DotnetExceptionOtherClass => {
    id: "dotnet-exception-other-class",
    assumed: "A .NET exception whose text the runtime does not pin is an AL error like the measured ones (Codeunit.Run answers false, asserterror catches it, a TryFunction answers false) only for a class an oracle case or the probe shows as an AL error: ArgumentNullException, FormatException, InvalidOperationException or ArgumentOutOfRangeException (with a member written in another letter case, an inner exception or a generic or nested type) thrown by a DotNet method called on an object or statically, by a property get on an object or by a constructor, and ArgumentException thrown by a constructor. The call must send no InStream or OutStream, must write into no OutStream an earlier call was handed, and the member must have no ref or out parameter. Its text and code are not known, so GetLastErrorText, GetLastErrorCode and GetLastErrorObject refuse after it, and a test that ends on it is not run. Every other class and binding keeps the refusal.",
    tests: "Corpus PR 1365 (open), tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al measures the error for the four classes. The oracle case KeyVault_Certificates (tests/al-language-onprem-dotnet/dotnet/key-vault/Y58AzureKeyVault.al:317-325) shows ArgumentException of the X509Certificate2 constructor inside a TryFunction that answers false. al-runner issue 7924 (an outside report).",
    probe: "A .NET exception of another class (for example OverflowException, KeyNotFoundException, NullReferenceException, CryptographicException) through the four bindings, ArgumentException through a method or property, and the four classes with a folded name, an inner exception or a generic type, in Codeunit.Run, asserterror and a TryFunction.",
    awaiting: Awaiting {
        issues: &[7924],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/DotNetExceptionOutcome.al"],
        since: "2026-11-12",
    },
}
