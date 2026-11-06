DotNetTextArgumentAsGuid => {
    id: "dotnet-text-argument-as-guid",
    assumed: "An AL Text argument of a DotNet constructor or method call binds to a System.Guid parameter when no overload takes the arguments as they are, the text is a Guid in form D (hyphens) or B (braces), and exactly one overload fits that way. Anything else stays refused. BC converts the text; its parse rules (other Guid forms, an invalid text: error or refusal) are not measured.",
    tests: "No corpus test. System Application, Image Impl..EncodeToImage (ImageImpl.Codeunit.al:347, `Encoder := Encoder.Encoder(QualityEncodeGuidTxt)`, the only one-argument constructor of System.Drawing.Imaging.Encoder takes a Guid), which passes on real BC (the reporter's run of Image.Resize, al-runner issue 5590).",
    probe: "An OnPrem test with DotNet variables: a Text that parses as a Guid into a constructor and a method whose only overload takes System.Guid; a Text that is no Guid (the error); other Guid forms (N, no hyphens, P); an overload set with both String and Guid (which wins); two Guid overloads of different other parameters; a System.Guid? parameter.",
    awaiting: Awaiting {
        issues: &[5590],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/R5590DotNetTextAsGuid.al"],
        since: "2026-11-06",
    },
}
