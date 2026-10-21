//! A page without OnFindRecord opened without a record, whose OnOpenPage leaves Rec on a row that is not the first of the
//! view: whether the page opens on that row or on the first one is not pinned (the corpus pins it only for pages with
//! OnFindRecord, pages/lt-ai-list-order/AiListOrderTests.al:825 OpenOnRow_SetPositionInOnOpenPage_LastRow). Refused.
//! Tests-ERM 134710 EmptySetup reaches it: Payment Registration Setup inserts the user's row in OnOpenPage while another
//! user's row sorts first.

const SRC: &str = r#"
table 69970 "OpMv Setup" {
    fields { field(1; "User ID"; Code[50]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "User ID") { Clustered = true; } }
}
page 69971 "OpMv Card"
{
    PageType = Card; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } field(NameCtl; Rec.Name) { } } }
    trigger OnOpenPage()
    begin
        if not Rec.Get('ME') then begin
            Rec.Init();
            Rec."User ID" := 'ME';
            Rec.Insert();
        end;
    end;
}
page 69972 "OpMv Plain"
{
    PageType = Card; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
}
codeunit 69973 C
{
    procedure OtherRowFirst(): Text
    var
        R: Record "OpMv Setup";
        P: TestPage "OpMv Card";
    begin
        R."User ID" := 'DUMMY';
        R.Name := 'X';
        R.Insert();
        P.OpenEdit();
        exit(P.UserCtl.Value + '/' + P.NameCtl.Value);
    end;
    procedure OnlyRow(): Text
    var
        P: TestPage "OpMv Card";
    begin
        P.OpenEdit();
        exit(P.UserCtl.Value);
    end;
    procedure NoOnOpenPage(): Text
    var
        R: Record "OpMv Setup";
        P: TestPage "OpMv Plain";
    begin
        R."User ID" := 'B';
        R.Insert();
        R."User ID" := 'A';
        R.Insert();
        P.OpenEdit();
        exit(P.UserCtl.Value);
    end;
}
"#;

#[test]
fn onopenpage_that_leaves_rec_past_the_first_row_is_refused() {
    match al_testkit::run_in(SRC, "C", "OtherRowFirst") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("OnOpenPage left Rec"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn onopenpage_that_inserts_the_only_row_opens_on_it() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "OnlyRow")), "ME");
}

#[test]
fn a_page_without_onopenpage_opens_on_the_first_row() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "NoOnOpenPage")), "A");
}
