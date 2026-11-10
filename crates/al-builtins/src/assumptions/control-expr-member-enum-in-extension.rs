ControlExprMemberEnumInExtension => {
    id: "control-expr-member-enum-in-extension",
    assumed: "A Boolean control expression (Visible, Editable, Enabled, HideValue) of a control a page extension declares or changes, with an Enum value named through an Enum field of the extension's own global Record variable (`ExtDir = ExtE.Mode::Beta`), compiles and runs as the same expression on a page's own control does: the value is a constant and the record is not read (pinned for a page's own controls by testpage/CcPageLeads.al:1051, :1118, :1185, :1386).",
    tests: "No Microsoft test. An outside report: 24 tests that open a page whose control has `Visible = EDocumentDirection = EDocument.Direction::Incoming` pass on real BC (al-runner issue 6924); the control may sit in a page extension.",
    probe: "testpage/CcVqMemberEnumProbe.al V9: a page extension that adds a field and an action with `ExtDir = ExtE.Mode::Beta` on Visible, Editable and Enabled, read after open, after GoToKey and after an action that sets ExtDir.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/testpage/CcVqMemberEnumProbe.al"],
        since: "2026-11-10",
    },
}
