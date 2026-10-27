//! A Worksheet with the new-row line (AutoSplitKey, DelayedInsert) opened on an empty table: an action inserts two lines through
//! another record variable without CurrPage.Update, then a second action deletes all lines and calls CurrPage.Update(false)
//! (al-runner issue 5960, the reporter's repro green on real BC, the flow of page 7372 "Bin Creation Worksheet"; registered
//! assumption `empty-worksheet-filled-by-action`, awaiting writer SH's RmWorksheetGoneRow.al
//! SH_WsWithNew_EmptyOpen_FillThenDeleteAllUpdate), or the test walks the rows (SH_WsWithNew_EmptyOpen_FillThenWalk).

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 51520 "EW Line" { fields { field(1; "Line No."; Integer) { } field(2; Description; Text[50]) { } } keys { key(PK; "Line No.") { } } }
codeunit 51521 "EW Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51522 "EW Ws"
{
    PageType = Worksheet; SourceTable = "EW Line"; AutoSplitKey = true; DelayedInsert = true;
    layout { area(Content) { repeater(Lines) { field(LineNo; Rec."Line No.") { } field(Descr; Rec.Description) { } } } }
    actions
    {
        area(Processing)
        {
            action(Calculate)
            {
                trigger OnAction()
                var Line: Record "EW Line";
                begin
                    Line."Line No." := 10000; Line.Description := 'First'; Line.Insert();
                    Line."Line No." := 20000; Line.Description := 'Second'; Line.Insert();
                end;
            }
            action(Create) { trigger OnAction() var Line: Record "EW Line"; begin Line.DeleteAll(); CurrPage.Update(false); end; }
        }
    }
    trigger OnAfterGetRecord() begin L.Add('R:' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Format(Rec."Line No.")); end;
    trigger OnClosePage() begin L.Add('X'); end;
    var L: Codeunit "EW Log";
}
codeunit 51523 C
{
    local procedure Count(): Text var Line: Record "EW Line"; begin exit(Format(Line.Count())); end;
    local procedure Open(var Ws: TestPage "EW Ws"): Text
    var L: Codeunit "EW Log"; s: Text;
    begin
        Ws.OpenEdit();
        s := Ws.LineNo.Value() + '|' + Ws.Descr.Value() + '|' + L.Take() + '|';
        Ws.Calculate.Invoke();
        exit(s + L.Take() + '|' + Ws.LineNo.Value() + '|' + Ws.Descr.Value() + '|' + Count() + '|');
    end;
    procedure DeleteAll(): Text
    var L: Codeunit "EW Log"; Ws: TestPage "EW Ws"; s: Text;
    begin
        s := Open(Ws);
        Ws.Create.Invoke();
        s += L.Take() + '|' + Ws.LineNo.Value() + '|' + Ws.Descr.Value() + '|' + Count() + '|' + Format(Ws.First()) + '|';
        Ws.Close();
        exit(s + L.Take() + '|' + Count());
    end;
    procedure Walk(): Text
    var L: Codeunit "EW Log"; Ws: TestPage "EW Ws"; s: Text; Rows: Text;
    begin
        s := Open(Ws);
        if Ws.First() then repeat Rows += Ws.LineNo.Value() + ','; until not Ws.Next();
        s += Rows + '|' + L.Take();
        Ws.Close();
        exit(s + '|' + Count());
    end;
}
"#;

const OPEN: &str = "0||N:0;C:0;N:0;C:0;|R:10000;R:20000;R:10000;R:10000;C:10000;N:0;C:0;|0||2|";

fn run(routine: &str) -> String {
    clear_hits();
    let s = al_testkit::text(al_testkit::run_in(SRC, "C", routine));
    assert!(take_hits().contains(&Assumption::EmptyWorksheetFilledByAction));
    s
}

#[test]
fn a_second_action_that_deletes_all_lines_stands_on_the_new_line() {
    assert_eq!(run("DeleteAll"), format!("{OPEN}R:0;N:0;C:0;N:0;C:0;|0||0|No|X;|0"));
}

#[test]
fn a_walk_lists_both_lines_then_the_new_line() {
    assert_eq!(
        run("Walk"),
        format!("{OPEN}10000,20000,0,|R:10000;R:10000;C:10000;R:20000;R:20000;C:20000;N:0;C:0;|2")
    );
}
