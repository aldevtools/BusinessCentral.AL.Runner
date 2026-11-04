//! A list whose only row an action deleted, with or without CurrPage.Update. Opened for editing it shows its blank line:
//! OnNewRecord and OnAfterGetCurrRecord three times (corpus pages/currpage/TestPageUpdateGoneRow.al:List_DeletedByAction_OnlyRow_ShowsNoStoredRow,
//! testpage/PnGRefresh.al:120, :142). Opened for viewing it raises nothing (pages/W1Update.al:534, :556).

use al_testkit::text;

const SRC: &str = r#"
table 69980 "GR Row" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
codeunit 69981 "GR Log"
{
    SingleInstance = true;
    var N: Integer;
    procedure Bump() begin N += 1; end;
    procedure Take(): Integer var R: Integer; begin R := N; N := 0; exit(R); end;
}
page 69982 "GR List"
{
    PageType = List; SourceTable = "GR Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } } } }
    actions { area(Processing) { action(Gone) { ApplicationArea = All; trigger OnAction() begin Rec.Delete(); end; } } }
    trigger OnAfterGetCurrRecord() var L: Codeunit "GR Log"; begin L.Bump(); end;
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "GR Log"; begin L.Bump(); end;
}
codeunit 69983 C
{
    procedure F(): Text
    var R: Record "GR Row"; P: TestPage "GR List"; L: Codeunit "GR Log"; Before: Integer;
    begin
        R.Code := 'A'; R.Insert();
        P.OpenEdit();
        L.Take();
        P.Gone.Invoke();
        Before := L.Take();
        P.Close();
        exit(Format(Before));
    end;
    procedure V(): Text
    var R: Record "GR Row"; P: TestPage "GR List"; L: Codeunit "GR Log"; Before: Integer;
    begin
        R.Code := 'A'; R.Insert();
        P.OpenView();
        L.Take();
        P.Gone.Invoke();
        Before := L.Take();
        P.Close();
        exit(Format(Before));
    end;
}
"#;

#[test]
fn a_list_whose_only_row_an_action_deleted_shows_the_blank_line() {
    // Three times OnNewRecord and OnAfterGetCurrRecord.
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "6");
}

#[test]
fn a_view_list_whose_only_row_an_action_deleted_raises_nothing() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "V")), "0");
}
