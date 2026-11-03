//! Page.Run and Page.RunModal with a fully qualified page name (corpus master pages/lt-y74-fqn-run/Y74FqnRunForms.al,
//! BC 28 and later): Run_Name_Only, Run_Name_Record, Run_Name_Record_FieldValue, Run_Name_Spellings, RunModal_Name_Only,
//! Err_NoNamespace, Err_PageIdAsText. A name that resolves to no page is not pinned and refused.

const SRC: &str = r#"
namespace My.Ns;
table 51700 "FQ Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[20]) { } } keys { key(PK; Id) { } } }
page 51701 "FQ Card" { PageType = Card; SourceTable = "FQ Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } } }
page 51702 "FQ List" { PageType = List; SourceTable = "FQ Row";
    layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
codeunit 51703 C
{
    procedure NoNamespace(): Text
    begin
        asserterror Page.Run('FQ Card');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure IdAsText(): Text
    begin
        asserterror Page.Run('51701');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure Missing()
    begin
        Page.Run('My.Ns.FQ Missing');
    end;
}
"#;

#[test]
fn a_name_without_a_namespace_raises_the_not_fully_qualified_error() {
    for routine in ["NoNamespace", "IdAsText"] {
        assert_eq!(
            al_testkit::text(al_testkit::run_in(SRC, "C", routine)),
            "The provided object name is not fully qualified.|InvalidOperation",
            "{routine}"
        );
    }
}

#[test]
fn a_name_that_resolves_to_no_page_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "Missing");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(w)) if w.contains("fully qualified name")),
        "{r:?}"
    );
}
