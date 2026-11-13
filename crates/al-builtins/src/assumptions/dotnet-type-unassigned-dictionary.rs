DotnetTypeUnassignedDictionary => {
    id: "dotnet-type-unassigned-dictionary",
    assumed: "GetDotNetType of an unassigned DotNet variable declared as System.Collections.Generic.Dictionary`2 is the open generic type definition Dictionary`2, as System.Type.GetType gives it; what the code reads from it then comes from .NET. Only that declared type: other open generics (List`1, KeyValuePair`2) stay refused.",
    tests: "Microsoft's Late Payment Prediction Tests, codeunit LP Prediction Test, TestIsEnoughDataAvailable: it passes on real BC through Base App MLPredictionManagement.CreateLabelDictionary, which calls MakeGenericType on this result (only a generic type definition allows it). al-runner issue 7833 (an outside report).",
    probe: "dotnet/dotnet/N4GetDotNetTypeGeneric.al (observe form): FullName, Name, Format and IsGenericTypeDefinition of the unassigned Dictionary`2, List`1 and KeyValuePair`2, the Microsoft MakeGenericType pattern, and an assigned Dictionary`2. Corpus branch al-runtime-rs-staging-natives4.",
    awaiting: Awaiting {
        issues: &[7833],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/N4GetDotNetTypeGeneric.al"],
        since: "2026-11-12",
    },
}
