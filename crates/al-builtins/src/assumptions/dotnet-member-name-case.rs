DotNetMemberNameCase => {
    id: "dotnet-member-name-case",
    assumed: "A DotNet member written in another letter case than its .NET name binds to that member (AL does not tell identifiers apart by case). A name some public method, property or field of the type carries exactly is kept; otherwise the one public name equal to it ignoring case is taken, for get, set and invoke, static and instance. Two or more names that differ from it only in case are refused (which one BC binds is not pinned). Flagged only when the case-insensitive match was taken.",
    tests: "Microsoft Tests-Data Exchange, codeunit 132547 \"Test Data Exch.Import - XML\", ReadElementFromXMLFileUTF8NodeNameUsingXmlReader (TestDataExchImportXML.Codeunit.al:120, runs codeunit 1203 directly; ReadRepeatedLineFromXMLFile and the other tests of that codeunit reach it through Import Bank Statement). Codeunit 1203 \"Import XML File to Data Exch.\" ParseParentChildLine (ImportXMLFiletoDataExch.Codeunit.al:106) writes `XmlElement.OuterXML` for the .NET `OuterXml`; it compiles and runs on BC (al-runner issue 6897).",
    probe: "An OnPrem test (tests/al-language-onprem-dotnet/dotnet/dotnet/R6897DotNetMemberCase.al): a method (LoadXml as loadxml and LOADXML), a property get (OuterXml as OuterXML), a property set (InnerText as innertext), a field and a static member, each written in another case. The ambiguous case (public members `value` and `Value`) needs an add-in assembly.",
    awaiting: Awaiting {
        issues: &[6897],
        files: &["tests/al-language-onprem-dotnet/dotnet/dotnet/R6897DotNetMemberCase.al"],
        since: "2026-11-03",
    },
}
