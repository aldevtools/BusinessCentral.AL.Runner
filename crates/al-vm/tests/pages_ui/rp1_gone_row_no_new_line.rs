//! A list without the new-row line (InsertAllowed = false) whose only row an action deleted: the page reads the gone row once
//! more, then runs OnNewRecord and OnAfterGetCurrRecord once on a blank Rec and shows no row; a second action runs the two
//! again. A Worksheet opened for edit, for view or run modally, a List opened for edit (corpus pages/lt-outside-worksheet/
//! RmWorksheetGoneRow.al:291-:450, al-runner issue 5752). A List without the line opened for view is not pinned.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69960 "GN Line" { fields { field(1; "Line No."; Integer) { } field(2; Description; Text[50]) { } } keys { key(PK; "Line No.") { } } }
codeunit 69961 "GN Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(S: Text) begin E += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := E; E := ''; exit(R); end;
}
page 69962 "GN Ws"
{
    PageType = Worksheet; SourceTable = "GN Line"; InsertAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } field(Descr; Rec.Description) { } } } }
    actions { area(Processing) {
        action(Gone) { trigger OnAction() var L: Record "GN Line"; begin L.DeleteAll(); CurrPage.Update(false); end; }
        action(GoneNoUpdate) { trigger OnAction() var L: Record "GN Line"; begin L.DeleteAll(); end; }
    } }
    trigger OnAfterGetRecord() var L: Codeunit "GN Log"; begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "GN Log"; begin L.Add('C:' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "GN Log"; begin L.Add('N:' + Format(Rec."Line No.")); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean var L: Codeunit "GN Log"; begin L.Add('Q'); exit(true); end;
    trigger OnClosePage() var L: Codeunit "GN Log"; begin L.Add('X'); end;
}
page 69963 "GN List"
{
    PageType = List; SourceTable = "GN Line"; InsertAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(LineNo; Rec."Line No.") { } } } }
    actions { area(Processing) { action(Gone) { trigger OnAction() var L: Record "GN Line"; begin L.DeleteAll(); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() var L: Codeunit "GN Log"; begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "GN Log"; begin L.Add('C:' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "GN Log"; begin L.Add('N:' + Format(Rec."Line No.")); end;
}
codeunit 69964 C
{
    local procedure Seed()
    var R: Record "GN Line"; L: Codeunit "GN Log";
    begin
        R."Line No." := 10000; R.Description := 'Only'; R.Insert();
        L.Take();
    end;
    procedure WsEdit(): Text
    var P: TestPage "GN Ws"; L: Codeunit "GN Log"; S: Text;
    begin
        Seed();
        P.OpenEdit();
        L.Take();
        P.Gone.Invoke();
        S := L.Take() + '|' + P.LineNo.Value() + '|' + P.Descr.Value() + '|' + Format(P.First()) + Format(P.Last()) + Format(P.Next()) + '|' + L.Take();
        P.Gone.Invoke();
        S += '|' + L.Take();
        P.Close();
        exit(S + '|' + L.Take());
    end;
    procedure WsViewNoUpdate(): Text
    var P: TestPage "GN Ws"; L: Codeunit "GN Log"; S: Text;
    begin
        Seed();
        P.OpenView();
        L.Take();
        P.GoneNoUpdate.Invoke();
        S := L.Take() + '|' + Format(P.First());
        P.Close();
        exit(S + '|' + L.Take());
    end;
    procedure ListEdit(): Text
    var P: TestPage "GN List"; L: Codeunit "GN Log"; S: Text;
    begin
        Seed();
        P.OpenEdit();
        L.Take();
        P.Gone.Invoke();
        exit(L.Take() + '|' + P.LineNo.Value() + '|' + Format(P.First()));
    end;
    procedure ListView()
    var P: TestPage "GN List";
    begin
        Seed();
        P.OpenView();
        P.Gone.Invoke();
    end;
}
"#;

#[test]
fn a_worksheet_without_the_new_row_line_that_lost_its_only_row_shows_a_blank_rec() {
    assert_eq!(
        text(run_in(SRC, "C", "WsEdit")),
        "R:10000;N:0;C:0;|||NoNoNo||N:0;C:0;|Q;X;"
    );
}

#[test]
fn a_worksheet_opened_for_view_does_the_same() {
    assert_eq!(text(run_in(SRC, "C", "WsViewNoUpdate")), "R:10000;N:0;C:0;|No|Q;X;");
}

#[test]
fn a_list_without_the_new_row_line_does_the_same() {
    assert_eq!(text(run_in(SRC, "C", "ListEdit")), "R:10000;N:0;C:0;||No");
}

#[test]
fn a_list_without_the_new_row_line_opened_for_view_is_refused() {
    assert!(matches!(
        run_in(SRC, "C", "ListView"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
