//! An action deletes the only row of a List with Editable = false opened for view, or empties a temporary Worksheet that shows
//! its rows as a tree (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:481 SH_ListViewNotEditable_OpenView_DeleteCurrentUpdate,
//! :509 and :535 SH_TreeWorksheetTemp_*, pages with InsertAllowed, DeleteAllowed and ModifyAllowed false). With DeleteAllowed
//! left out they are the registered assumptions `read-only-view-list-gone-only-row` and `tree-worksheet-gone-only-row-blank-rec`.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 51500 "GV Line" { fields { field(1; "Line No."; Integer) { } field(2; Description; Text[50]) { } field(3; Indentation; Integer) { } } keys { key(PK; "Line No.") { } } }
codeunit 51501 "GV Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51502 "GV List"
{
    PageType = List; SourceTable = "GV Line"; Editable = false; InsertAllowed = false; DeleteAllowed = false; ModifyAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNo; Rec."Line No.") { } field(Descr; Rec.Description) { } } } }
    actions { area(Processing) { action(Retry) { trigger OnAction() var Line: Record "GV Line"; begin Line.Get(Rec."Line No."); Line.Delete(); CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec."Line No.")); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "GV Log";
}
page 51503 "GV Tree"
{
    PageType = Worksheet; SourceTable = "GV Line"; SourceTableTemporary = true; InsertAllowed = false; DeleteAllowed = false; ModifyAllowed = false;
    layout { area(Content) { repeater(Lines) { Editable = false; IndentationColumn = Rec.Indentation; IndentationControls = Descr; ShowAsTree = true; field(LineNo; Rec."Line No.") { } field(Descr; Rec.Description) { } } } }
    actions { area(Processing) { action(Finish) { trigger OnAction() begin Rec.Reset(); Rec.DeleteAll(); end; } } }
    trigger OnOpenPage() begin Rec."Line No." := 10000; Rec.Description := 'Only line'; Rec.Insert(); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec."Line No.")); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "GV Log";
}
codeunit 51504 C
{
    procedure ViewList(): Text
    var Line: Record "GV Line"; L: Codeunit "GV Log"; P: TestPage "GV List"; s: Text;
    begin
        Line."Line No." := 10000; Line.Description := 'Only line'; Line.Insert();
        P.OpenView();
        L.Take();
        P.Retry.Invoke();
        s := L.Take() + '|' + P.LineNo.Value() + '|' + P.Descr.Value() + '|' + Format(Line.Count()) + '|' + Format(P.First()) + '|' + L.Take();
        P.Close();
        exit(s + '|' + L.Take());
    end;
    procedure Tree(): Text
    var L: Codeunit "GV Log"; P: TestPage "GV Tree"; s: Text;
    begin
        P.OpenEdit();
        L.Take();
        P.Finish.Invoke();
        s := L.Take() + '|' + P.LineNo.Value() + '|' + P.Descr.Value() + '|' + Format(P.First()) + '|' + L.Take();
        P.Finish.Invoke();
        s += '|' + L.Take() + '|' + Format(P.First());
        P.Close();
        exit(s + '|' + L.Take());
    end;
}
"#;

#[test]
fn a_read_only_list_opened_for_view_raises_nothing_and_shows_no_row() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "ViewList")),
        "R:10000;|||0|No||X;"
    );
    assert!(!take_hits().contains(&Assumption::ReadOnlyViewListGoneOnlyRow));
    let src = SRC.replacen(
        "Editable = false; InsertAllowed = false; DeleteAllowed = false;",
        "Editable = false; InsertAllowed = false;",
        1,
    );
    clear_hits();
    al_testkit::run_in(&src, "C", "ViewList").ok();
    assert!(take_hits().contains(&Assumption::ReadOnlyViewListGoneOnlyRow));
}

#[test]
fn a_tree_worksheet_stands_on_a_blank_rec() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Tree")),
        "R:10000;N:0;C:0;|||No||N:0;C:0;|No|X;"
    );
    assert!(!take_hits().contains(&Assumption::TreeWorksheetGoneOnlyRowBlankRec));
    let src = SRC.replacen(
        "SourceTableTemporary = true; InsertAllowed = false; DeleteAllowed = false;",
        "SourceTableTemporary = true; InsertAllowed = false;",
        1,
    );
    clear_hits();
    al_testkit::run_in(&src, "C", "Tree").ok();
    assert!(take_hits().contains(&Assumption::TreeWorksheetGoneOnlyRowBlankRec));
}
