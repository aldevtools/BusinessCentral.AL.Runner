//! First() after an action deleted the row a plain list stood on (corpus pages/W1Update.al:574 List_ActionDeletesOneOfTwoRows_WithoutUpdate).

const SRC: &str = r#"
table 51800 "ADC Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51801 "ADC Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51802 "ADC List" { PageType = List; SourceTable = "ADC Row";
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } } } }
    actions { area(Processing) { action(DelNoUpdate) { trigger OnAction() begin Rec.Delete(); L.Add('done'); end; }
      action(DelGet) { trigger OnAction() begin Rec.Delete(); Rec.Get(2); end; } } }
    trigger OnAfterGetRecord() begin L.Add('r' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin L.Add('c' + Format(Rec.Id)); end;
    var L: Codeunit "ADC Log"; }
codeunit 51803 C
{
    procedure Run(Edit: Boolean; GetOther: Boolean): Text
    var R: Record "ADC Row"; L: Codeunit "ADC Log"; P: TestPage "ADC List"; s: Text;
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R.Insert();
        if Edit then P.OpenEdit() else P.OpenView();
        if Edit then P.GoToKey(1);
        L.Take();
        if GetOther then P.DelGet.Invoke() else P.DelNoUpdate.Invoke();
        s := L.Take() + '|' + P.IdCtl.Value();
        s += '|' + Format(P.First()) + '|' + P.IdCtl.Value() + '|' + Format(P.Next()) + '|' + L.Take();
        exit(s);
    end;
}
"#;

fn run(edit: bool, get_other: bool) -> String {
    al_testkit::text(
        al_testkit::run_with(
            SRC,
            "C",
            "Run",
            vec![
                al_builtins::value::Value::Boolean(edit),
                al_builtins::value::Value::Boolean(get_other),
            ],
        )
        .map(|(v, _)| v),
    )
}

// W1Update.al:577-580: the action log, Id after the action, First answers Yes and stands on 2.
#[test]
fn first_after_the_action_deleted_the_current_row_stands_on_the_other_row() {
    let want = "r1;done;r2;r2;c2;r2;r2;c2;|2|Yes|2|No|";
    assert_eq!(run(false, false), want);
}

// RtFilteredGoneRow.al:803 TD_DeleteOneOfTwo_MoveToOther_NoUpdate: the action moved Rec to the other row itself; First and Next
// read nothing (`walk log` empty).
#[test]
fn the_action_that_moves_rec_to_the_other_row_reads_it_as_a_move() {
    assert_eq!(run(false, true), "r1;r2;r2;c2;r2;r2;c2;|2|Yes|2|No|");
}
