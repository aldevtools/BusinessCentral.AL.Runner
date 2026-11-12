DotnetTypeOpenGeneric => {
    id: "dotnet-type-open-generic",
    assumed: "GetDotNetType of a DotNet variable declared as System.Collections.Generic.Dictionary`2, List`1 or KeyValuePair`2 that holds no object, or declared as Dictionary`2 and holding the dictionary its own constructor made, is the open generic type definition, as System.Type.GetType of the declared name gives it; what the code reads from it then comes from .NET (FullName, Name, Format, IsGenericTypeDefinition, MakeGenericType). Other open generics, and a Dictionary`2 variable holding an object .NET made, keep main's answer or refusal.",
    tests: "Corpus PR 1365 (open), tests/al-language-onprem-dotnet/dotnet/dotnet/N4GetDotNetTypeGeneric.al: the three unassigned types, the Microsoft MakeGenericType pattern and an assigned Dictionary`2, on every BC major the corpus CI runs. Microsoft's Late Payment Prediction Tests, codeunit LP Prediction Test, TestIsEnoughDataAvailable (through Base App MLPredictionManagement.CreateLabelDictionary). al-runner issue 7833 (an outside report).",
    probe: "N4GetDotNetTypeGeneric.al (the cells above). Not measured: other generic types, a Dictionary`2 variable holding a closed dictionary .NET made.",
    awaiting: Awaiting {
        issues: &[7833],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/N4GetDotNetTypeGeneric.al"],
        since: "2026-11-12",
    },
}
