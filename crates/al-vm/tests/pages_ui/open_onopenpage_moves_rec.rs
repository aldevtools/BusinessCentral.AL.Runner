//! A page without OnFindRecord opened without a record, whose OnOpenPage leaves Rec on a row that is not the first of the
//! view: a Card or List opens on that row, or on the nearest row of the view when Rec is outside the filter OnOpenPage set
//! (corpus pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:175 QR_Card_OpenView_Get, :292 QR_List_OpenView_Get, :253
//! QR_Card_OpenView_GetThenFilterExcludes, :409 QR_TempCard_OpenView_NoMove). Other page types, and a list whose key
//! OnOpenPage changed (:344 QR_List_OpenView_SetCurrentKeyGet), stay refused.
//! Tests-ERM 134710 EmptySetup reaches it: Payment Registration Setup inserts the user's row in OnOpenPage while another
//! user's row sorts first.

const SRC: &str = r#"
table 69970 "OpMv Setup" {
    fields { field(1; "User ID"; Code[50]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "User ID") { Clustered = true; } key(ByName; Name) { } }
}
page 69971 "OpMv Card"
{
    PageType = Card; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } field(NameCtl; Rec.Name) { } } }
    trigger OnOpenPage()
    begin
        case Mode.Get() of
            1: Rec.Get('C');
            7: begin Rec.Get('C'); Rec.SetRange("User ID", 'A', 'B'); end;
        end;
        Mode.Add('O:' + Rec."User ID");
    end;
    trigger OnAfterGetRecord() begin Mode.Add('R:' + Rec."User ID"); end;
    trigger OnAfterGetCurrRecord() begin Mode.Add('C:' + Rec."User ID"); end;
    var Mode: Codeunit OpMvMode;
}
page 69974 "OpMv List"
{
    PageType = List; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(UserCtl; Rec."User ID") { } } } }
    trigger OnOpenPage()
    begin
        case Mode.Get() of
            1: Rec.Get('C');
            7: begin Rec.Get('C'); Rec.SetRange("User ID", 'A', 'B'); end;
            2: Rec.FindLast();
            5: begin Rec.SetCurrentKey(Name); Rec.Get('B'); end;
        end;
        Mode.Add('O:' + Rec."User ID");
    end;
    trigger OnAfterGetRecord() begin Mode.Add('R:' + Rec."User ID"); end;
    trigger OnAfterGetCurrRecord() begin Mode.Add('C:' + Rec."User ID"); end;
    var Mode: Codeunit OpMvMode;
}
page 69975 "OpMv Temp Card"
{
    PageType = Card; SourceTable = "OpMv Setup"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
    trigger OnOpenPage()
    begin
        Rec."User ID" := 'A'; Rec.Insert();
        Rec."User ID" := 'B'; Rec.Insert();
    end;
}
page 69976 "OpMv Document"
{
    PageType = Document; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
    trigger OnOpenPage() begin Rec.Get('C'); end;
}
page 69972 "OpMv Plain"
{
    PageType = Card; SourceTable = "OpMv Setup"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(UserCtl; Rec."User ID") { } } }
}
codeunit 69977 OpMvMode
{
    SingleInstance = true;
    var M: Integer; L: Text;
    procedure Set(N: Integer) begin M := N; L := ''; end;
    procedure Get(): Integer begin exit(M); end;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
}
codeunit 69973 C
{
    var Mode: Codeunit OpMvMode;
    local procedure Seed(N: Integer)
    var
        R: Record "OpMv Setup";
    begin
        R.DeleteAll();
        R."User ID" := 'A'; R.Insert();
        R."User ID" := 'B'; R.Insert();
        R."User ID" := 'C'; R.Name := 'c'; R.Insert();
        R."User ID" := 'D'; R.Insert();
        Mode.Set(N);
    end;
    procedure CardGet(): Text
    var
        P: TestPage "OpMv Card";
    begin
        Seed(1);
        P.OpenView();
        exit(P.UserCtl.Value + '/' + P.NameCtl.Value + '|' + Mode.Take());
    end;
    procedure CardOutsideFilter(): Text
    var
        P: TestPage "OpMv Card";
    begin
        Seed(7);
        P.OpenView();
        exit(P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure ListGet(): Text
    var
        P: TestPage "OpMv List";
    begin
        Seed(1);
        P.OpenView();
        exit(P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure ListNewLineThenPrevious(): Text
    var
        P: TestPage "OpMv List";
        T: Text;
    begin
        Seed(2);
        P.OpenEdit();
        Mode.Take();
        T := Format(P.Next()) + '|' + P.UserCtl.Value + '|' + Mode.Take();
        exit(T + Format(P.Previous()) + '|' + P.UserCtl.Value + '|' + Mode.Take());
    end;
    procedure ListKeyChanged(): Text
    var
        P: TestPage "OpMv List";
    begin
        Seed(5);
        P.OpenView();
        exit(P.UserCtl.Value);
    end;
    procedure TempCard(): Text
    var
        P: TestPage "OpMv Temp Card";
    begin
        Seed(0);
        P.OpenView();
        exit(P.UserCtl.Value);
    end;
    procedure Document(): Text
    var
        P: TestPage "OpMv Document";
    begin
        Seed(0);
        P.OpenView();
        exit(P.UserCtl.Value);
    end;
    procedure OnlyRow(): Text
    var
        R: Record "OpMv Setup";
        P: TestPage "OpMv Card";
    begin
        R.DeleteAll();
        Mode.Set(0);
        R."User ID" := 'ME';
        R.Insert();
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
fn a_card_opens_on_the_row_onopenpage_got() {
    // QrMovedTests.al:175 QR_Card_OpenView_Get `C|bravo`, `O:C;R:C;C:C;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CardGet")),
        "C/c|O:C;R:C;C:C;"
    );
}

#[test]
fn a_card_whose_rec_is_outside_the_filter_opens_on_the_nearest_row_of_the_view() {
    // QrMovedTests.al:253 QR_Card_OpenView_GetThenFilterExcludes `B|charlie`, `O:C;R:B;C:B;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "CardOutsideFilter")),
        "B|O:C;R:B;C:B;"
    );
}

#[test]
fn a_list_opens_on_the_row_onopenpage_got() {
    // QrMovedTests.al:292 QR_List_OpenView_Get `C|bravo`, `O:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;C:C;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ListGet")),
        "C|O:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;C:C;"
    );
}

#[test]
fn a_list_on_the_moved_rec_goes_back_from_the_new_line_as_a_move() {
    // QrMovedTests.al:615 QR_List_RunModal_FindLast: `next` `Yes|`, `next log` `C:;`, `previous log` `R:D;R:D;C:D;`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ListNewLineThenPrevious")),
        "Yes||C:;Yes|D|R:D;R:D;C:D;"
    );
}

#[test]
fn a_list_whose_key_onopenpage_changed_is_refused() {
    match al_testkit::run_in(SRC, "C", "ListKeyChanged") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("changed its key"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_temporary_source_filled_in_onopenpage_opens_on_the_last_insert() {
    // QrMovedTests.al:409 QR_TempCard_OpenView_NoMove `D|alpha` (Rec left on the last row inserted).
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "TempCard")), "B");
}

#[test]
fn another_page_type_whose_onopenpage_moves_rec_is_refused() {
    match al_testkit::run_in(SRC, "C", "Document") {
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
