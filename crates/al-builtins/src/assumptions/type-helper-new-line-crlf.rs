TypeHelperNewLineCrlf => {
    id: "type-helper-new-line-crlf",
    assumed: "Microsoft.Dynamics.Nav.SystemEnvironment.SystemEnvironment.NewLine (System Application Type Helper.NewLine) is CR LF (13, 10), as on a Windows service tier. The corpus pins only that the last character is LF (Y58ServerHelpers.al SystemEnvironment_New_NewLine, CnSessionLeads.al TypeHelper_NewLine_CharacterCodes); the Linux CI containers measured LF only, and the Windows value is not measured. (Constructor, ToString, Format, Equals and the error on older majors for a never-constructed variable are pinned and not marked.)",
    tests: "System Application Test Library / Tests-Misc TypeHelperUnitTests.Codeunit.al:508-515 TestNewLine (Assert.AreEqual(TypeHelper.CRLFSeparator(), TypeHelper.NewLine())), which passes on real BC; and the 152 tests of al-runner issue 6924 that reach Type Helper.NewLine.",
    probe: "tests/al-language/session-system/TypeHelperNewLineValue.al TypeHelper_NewLine_FullValue (corpus pull request 1281): StrLen and the code of each character of Type Helper.NewLine on every supported major; expect 2 and 13,10 once the bc-linux CR LF patch (bc-linux issues 227 and 228) is in the corpus CI.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/session-system/TypeHelperNewLineValue.al"],
        since: "2026-11-10",
    },
}
