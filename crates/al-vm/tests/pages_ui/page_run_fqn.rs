//! Page.Run and Page.RunModal with a fully qualified page name (corpus master pages/run-modal/Y74FqnRunForms.al,
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
    procedure EmptyWithRecord(Modal: Boolean): Text
    var
        Row: Record "FQ Row";
    begin
        if Modal then
            asserterror Page.RunModal('', Row)
        else
            asserterror Page.Run('', Row);
        exit(GetLastErrorCode());
    end;
    procedure EmptyAlone()
    begin
        Page.Run('');
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

// corpus pages/missing-object-fqn/Y89FqnMissing.al:125 Y89_Fqn_Empty_RunRec and :153 Y89_Fqn_Empty_RunModalRec (BC 28.5): an empty
// name with a record raises AL:MetadataNotFound (the text is not asserted there). The name alone ends in an uncatchable
// NullReferenceException on BC (:113 Y89_Fqn_Empty_Run is no [Test]) and stays refused.
#[test]
fn an_empty_name_with_a_record_raises_metadata_not_found() {
    for modal in ["true", "false"] {
        let src = SRC
            .replace(
                "procedure EmptyWithRecord(Modal: Boolean)",
                "procedure EmptyWithRecord()",
            )
            .replace("if Modal then", &format!("if {modal} then"));
        assert_eq!(
            al_testkit::text(al_testkit::run_in(&src, "C", "EmptyWithRecord")),
            "AL:MetadataNotFound",
            "modal {modal}"
        );
    }
}

#[test]
fn an_empty_name_alone_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "EmptyAlone");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
