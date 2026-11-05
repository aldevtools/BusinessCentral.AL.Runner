//! A marking OnDrillDown that walks Rec itself, turns MarkedOnly on, leaves Rec on an unmarked row and calls
//! CurrPage.Update(false), on a list with a part: GoToRecord finds the marked row the refill settles on (al-runner issue 6946;
//! corpus pages/onopenpage-moves-rec/DrillDownMarkedRecLeft.al, measured on BC 27.5, 28.5 and 29.0).

const SRC: &str = r#"
codeunit 69980 "DM Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin if Entries <> '' then Entries += ';'; Entries += T; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
table 69981 "DM Op"
{
    fields { field(1; Id; Integer) { } field(2; Name; Code[20]) { } field(3; Late; Boolean) { } }
    keys { key(PK; Id) { } key(ByName; Name) { } }
}
page 69982 "DM Part" { PageType = ListPart; SourceTable = "DM Op"; Editable = false; layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } } }
page 69983 "DM List Name"
{
    PageType = List; SourceTable = "DM Op"; SourceTableView = sorting(Name); Editable = false; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(Tiles) { field(LeaveLast; LateTxt) { trigger OnDrillDown() begin Log.Add('d' + Format(Rec.Id)); MarkLate(); CurrPage.Update(false); Log.Add('u' + Format(Rec.Id)); end; } }
            repeater(Rows) { field(Id; Rec.Id) { } }
            part(Details; "DM Part") { SubPageLink = Id = field(Id); }
        }
    }
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C' + Format(Rec.Id)); end;
    local procedure MarkLate()
    begin
        Rec.ClearMarks();
        if Rec.FindSet() then repeat if Rec.Late then Rec.Mark(true); until Rec.Next() = 0;
        Rec.MarkedOnly(true);
    end;
    var Log: Codeunit "DM Log"; LateTxt: Text;
}
page 69984 "DM List Pk"
{
    PageType = List; SourceTable = "DM Op"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(Tiles) { field(LeaveLast; LateTxt) { trigger OnDrillDown() begin Log.Add('d' + Format(Rec.Id)); MarkLate(); CurrPage.Update(false); Log.Add('u' + Format(Rec.Id)); end; } }
            repeater(Rows) { field(Id; Rec.Id) { } }
            part(Details; "DM Part") { SubPageLink = Id = field(Id); }
        }
    }
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C' + Format(Rec.Id)); end;
    local procedure MarkLate()
    begin
        Rec.ClearMarks();
        if Rec.FindSet() then repeat if Rec.Late then Rec.Mark(true); until Rec.Next() = 0;
        Rec.MarkedOnly(true);
    end;
    var Log: Codeunit "DM Log"; LateTxt: Text;
}
codeunit 69985 C
{
    local procedure Seed()
    var Op: Record "DM Op"; Log: Codeunit "DM Log"; i: Integer;
    begin
        for i := 1 to 4 do begin Op.Init(); Op.Id := i; Op.Name := CopyStr('ABCD', i, 1); Op.Late := i in [2, 3]; Op.Insert(); end;
        Log.Take();
    end;
    procedure Name(First: Integer; Second: Integer): Text
    var P: TestPage "DM List Name"; Op: Record "DM Op"; Log: Codeunit "DM Log"; T: Text;
    begin
        Seed();
        P.OpenView();
        Log.Take();
        P.LeaveLast.Drilldown();
        T := '[' + P.Id.Value() + ']:' + Log.Take();
        Op.Get(First);
        T += '|' + Format(P.GoToRecord(Op)) + '[' + P.Id.Value() + ']:' + Log.Take() + '|' + P.Details.Id.Value();
        Op.Get(Second);
        T += '|' + Format(P.GoToRecord(Op)) + '[' + P.Id.Value() + ']:' + Log.Take();
        P.Close();
        exit(T);
    end;
    procedure Pk(First: Integer; Second: Integer): Text
    var P: TestPage "DM List Pk"; Op: Record "DM Op"; Log: Codeunit "DM Log"; T: Text;
    begin
        Seed();
        P.OpenView();
        Log.Take();
        P.LeaveLast.Drilldown();
        T := '[' + P.Id.Value() + ']:' + Log.Take();
        Op.Get(First);
        T += '|' + Format(P.GoToRecord(Op)) + '[' + P.Id.Value() + ']:' + Log.Take() + '|' + P.Details.Id.Value();
        Op.Get(Second);
        T += '|' + Format(P.GoToRecord(Op)) + '[' + P.Id.Value() + ']:' + Log.Take();
        P.Close();
        exit(T);
    end;
}
"#;

fn run(routine: &str, first: i32, second: i32) -> String {
    use al_builtins::value::Value;
    let (v, _) = al_testkit::run_with(SRC, "C", routine, vec![Value::Integer(first), Value::Integer(second)]).unwrap();
    al_testkit::text(Ok(v))
}

// DrillDownMarkedRecLeft.al Name_LeaveUnmarked_GoToMarkedThenUnmarked: the reporter's shape.
#[test]
fn in_another_key_goto_finds_the_marked_row_before_recs_row() {
    assert_eq!(
        run("Name", 3, 4),
        "[1]:d1;u4|Yes[3]:R4;R3;R2;R3;R3;R3;C3|3|No[3]:R2;R2;R2;C2;R3;R3;R3;C3"
    );
}

// Name_LeaveUnmarked_GoToUnmarked: GoToRecord of the row Rec was left on walks the marked rows and answers false.
#[test]
fn in_another_key_goto_of_recs_unmarked_row_answers_false() {
    assert_eq!(
        run("Name", 4, 1),
        "[1]:d1;u4|No[3]:R4;R3;R2;R3;R3;R3;C3;R2;R2;R2;C2;R3;R3;R3;C3|3|No[3]:R2;R2;R2;C2;R3;R3;R3;C3"
    );
}

// Pk_LeaveUnmarked_GoToMarkedThenUnmarked and Pk_LeaveUnmarked_GoToFirstMarked.
#[test]
fn in_the_primary_key_the_same() {
    assert_eq!(
        run("Pk", 3, 4),
        "[1]:d1;u4|Yes[3]:R3;R2;R3;R3;C3|3|No[3]:R2;R2;C2;R3;R3;C3"
    );
    assert_eq!(
        run("Pk", 2, 3),
        "[1]:d1;u4|No[3]:R3;R2;R3;R3;C3;R2;R2;C2;R3;R3;C3|3|Yes[3]:R2;R2;C2;R3;R3;C3"
    );
}
