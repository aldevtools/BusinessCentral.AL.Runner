//! xRec in a control's OnValidate on a NEW record of a DelayedInsert card opened with OpenNew (pages/draft.rs `start_draft`).
//! OnNewRecord sees the last row as xRec (handlers/TestPageCreateModeNewRecordArgs.al:243-257 `x=B`); from then on xRec is
//! the new record. Pinned by corpus handlers/TestPageControlOnValidateError_Tests.al:113-124
//! ControlOnValidate_AcceptsAValueItDoesNotRefuse: Base Application page 9170 "Profile Card" (DelayedInsert, OnNewRecord sets
//! Scope) confirms when `xRec."Profile ID" <> ''` in the key control's OnValidate, the table holds rows, and real BC passes the
//! test with no ConfirmHandler.

const SRC: &str = r#"
table 69990 "NX Row"
{
    fields { field(1; Scope; Option) { OptionMembers = System,Tenant; } field(2; "App ID"; Guid) { } field(3; "Profile ID"; Code[30]) { } field(4; Caption; Text[30]) { } }
    keys { key(PK; Scope, "App ID", "Profile ID") { Clustered = true; } }
}
page 69991 "NX Card"
{
    PageType = Card; SourceTable = "NX Row"; DelayedInsert = true; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(G)
            {
                field(ScopeField; Rec.Scope) { Visible = false; }
                field(ProfileIdField; Rec."Profile ID") { trigger OnValidate() begin Seen += 'v:x=' + xRec."Profile ID" + ',r=' + Rec."Profile ID" + ';'; end; }
                field(CaptionField; Rec.Caption) { }
                field(SeenField; Seen) { }
            }
        }
    }
    trigger OnNewRecord(BelowxRec: Boolean) begin Seen += 'new:x=' + xRec."Profile ID" + ';'; Rec.Scope := Rec.Scope::Tenant; end;
    var Seen: Text;
}
codeunit 69992 C
{
    procedure NewKeyTyped(): Text
    var Card: TestPage "NX Card"; Row: Record "NX Row";
    begin
        Row.Scope := Row.Scope::Tenant;
        Row."Profile ID" := 'LAST';
        Row.Insert();
        Card.OpenNew();
        Card.ProfileIdField.SetValue('P1');
        exit(Card.SeenField.Value());
    end;
}
"#;

#[test]
fn xrec_of_a_new_card_record_is_the_new_record_after_on_new_record() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "NewKeyTyped")),
        "new:x=LAST;v:x=,r=P1;"
    );
}
