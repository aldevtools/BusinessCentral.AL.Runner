//! An action that puts another stored row into Rec with Rec.Get, without CurrPage.Update, on a list of thirty rows (more than the
//! client's window of twenty) with OnAfterGetRecord and OnAfterGetCurrRecord and without OnFindRecord or OnNextRecord. How far the
//! list was walked with Next before the action makes no difference: the list reads its rows again from the row the action left
//! (corpus testpage/PnGRefresh.al, the BW_ tests of al-runner issue 7288, measured on BC 29.0).
const SRC: &str = r#"
table 51520 "BW Row"
{
    fields { field(1; Id; Integer) { } field(2; Grp; Code[10]) { } }
    keys { key(PK; Id) { } }
}
codeunit 51521 "BW Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end;
}
page 51522 "BW List"
{
    PageType = List;
    SourceTable = "BW Row";
    InsertAllowed = false;
    DeleteAllowed = false;
    layout { area(Content) { repeater(R) { field(NoF; Rec.Grp) { Editable = false; } field(CellF; CellText) { Editable = false; } } } }
    actions
    {
        area(Processing)
        {
            action(GetRow2) { trigger OnAction() begin Rec.Get(2); L.Add('A:' + Rec.Grp + '/x=' + xRec.Grp); end; }
            action(GetRow24) { trigger OnAction() begin Rec.Get(24); L.Add('A:' + Rec.Grp + '/x=' + xRec.Grp); end; }
            action(GetRow28) { trigger OnAction() begin Rec.Get(28); L.Add('A:' + Rec.Grp + '/x=' + xRec.Grp); end; }
        }
    }
    var L: Codeunit "BW Log"; CellText: Text;
    trigger OnAfterGetRecord() begin CellText := 'cell of ' + Rec.Grp; L.Add('G:' + Rec.Grp); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Rec.Grp + '/x=' + xRec.Grp); end;
}
codeunit 51523 "BW Test"
{
    var L: Codeunit "BW Log";

    local procedure Walk(var P: TestPage "BW List"; FromRow: Integer; ToRow: Integer; Filt: Text): Text
    var R: Record "BW Row"; I: Integer;
    begin
        R.DeleteAll();
        for I := 1 to 30 do begin R.Init(); R.Id := I; R.Grp := 'R' + Format(I); R.Insert(); end;
        P.OpenEdit();
        if Filt <> '' then P.Filter.SetFilter(Id, Filt);
        P.First();
        for I := FromRow + 1 to ToRow do P.Next();
        L.Take();
        exit(P.NoF.Value());
    end;

    local procedure After(var P: TestPage "BW List"): Text
    var T: Text;
    begin
        T := L.Take() + '|' + P.NoF.Value() + ',' + P.CellF.Value();
        T += '|' + Format(P.Next()) + '|' + L.Take() + '|' + P.NoF.Value() + ',' + P.CellF.Value();
        T += '|' + Format(P.Previous()) + '|' + L.Take() + '|' + P.NoF.Value() + ',' + P.CellF.Value();
        P.Close();
        exit(T + '|' + L.Take());
    end;

    procedure Get2(FromRow: Integer): Text
    var P: TestPage "BW List"; S: Text;
    begin S := Walk(P, 1, FromRow, ''); P.GetRow2.Invoke(); exit(S + '|' + After(P)); end;

    procedure Get24(FromRow: Integer): Text
    var P: TestPage "BW List"; S: Text;
    begin S := Walk(P, 1, FromRow, ''); P.GetRow24.Invoke(); exit(S + '|' + After(P)); end;

    procedure Get28(FromRow: Integer): Text
    var P: TestPage "BW List"; S: Text;
    begin S := Walk(P, 1, FromRow, ''); P.GetRow28.Invoke(); exit(S + '|' + After(P)); end;

    procedure Get2Filtered(): Text
    var P: TestPage "BW List"; S: Text;
    begin S := Walk(P, 3, 25, '3..30'); P.GetRow2.Invoke(); exit(S + '|' + After(P)); end;
}
"#;

fn run(routine: &str, arg: i32) -> String {
    let args = if arg == 0 {
        vec![]
    } else {
        vec![al_builtins::value::Value::Integer(arg)]
    };
    al_testkit::text(al_testkit::run_with(SRC, "BW Test", routine, args).map(|(v, _)| v))
}

/// The reads after the action that leaves R2, and the steps after it (testpage/PnGRefresh.al BW_GetRow2_FromRow3_InsideFirstWindow,
/// _FromRow20_LastRowOfFirstWindow, _FromRow21_FirstRowPastFirstWindow, _FromRow25_BeyondFirstWindow, _FromRow30_LastRowOfView).
fn row2_from(n: i32) -> String {
    format!(
        "R{n}|G:R{n};A:R2/x=R{n};G:R2;G:R2;G:R3;G:R4;G:R5;G:R6;G:R7;G:R8;G:R9;G:R10;G:R11;G:R12;G:R13;G:R14;G:R15;G:R16;G:R17;G:R18;G:R19;G:R20;G:R21;G:R2;G:R1;G:R2;G:R2;C:R2/x=R{n}|R2,cell of R2|Yes|G:R3;G:R3;C:R3/x=R2|R3,cell of R3|Yes|G:R2;G:R2;C:R2/x=R3|R2,cell of R2|"
    )
}

#[test]
fn an_action_that_leaves_row_2_from_inside_the_first_window_reads_the_list_again_from_row_2() {
    assert_eq!(run("Get2", 3), row2_from(3));
    assert_eq!(run("Get2", 20), row2_from(20));
}

#[test]
fn the_same_from_a_row_past_the_first_window() {
    assert_eq!(run("Get2", 21), row2_from(21));
    assert_eq!(run("Get2", 25), row2_from(25));
    assert_eq!(run("Get2", 30), row2_from(30));
}

/// The shapes the runtime does not build stay refused: a row near the end of the view, read back 20 rows only
/// (testpage/PnGRefresh.al BW_GetRow24_FromRow25_RowAboveInWindow, BW_GetRow28_FromRow25_RowBelowInView), a row past the first window
/// left from inside it, read twice (BW_GetRow28_FromRow3_RowPastFirstWindow: refused at the Next after it), and a row outside the
/// filter (BW_Filtered_GetRow2_FromRow25_RowOutsideFilter).
#[test]
fn other_rows_left_by_the_action_stay_refused() {
    for (routine, arg) in [("Get24", 25), ("Get28", 25), ("Get28", 3), ("Get2Filtered", 0)] {
        let args = if arg == 0 {
            vec![]
        } else {
            vec![al_builtins::value::Value::Integer(arg)]
        };
        let r = al_testkit::run_with(SRC, "BW Test", routine, args);
        assert!(
            matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{routine}({arg}): {:?}",
            r.map(|(v, _)| v)
        );
    }
}
