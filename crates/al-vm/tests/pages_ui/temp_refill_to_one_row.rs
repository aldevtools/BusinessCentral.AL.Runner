//! An action refills a temporary Worksheet of 3 or 25 rows with one row (pages/lt-outside-worksheet/RmTempRefill.al
//! SH_RefillFewer_*): the
//! client reads that row three times and runs OnAfterGetCurrRecord (twice and nothing more without that trigger), then lists
//! only that row.

const PAGE: &str = r#"
page 51491 "TR Ws"
{
    PageType = Worksheet; SourceTable = "TR Row"; SourceTableTemporary = true;
    InsertAllowed = false; ModifyAllowed = false; DeleteAllowed = false;
    layout { area(Content) { repeater(Rows) { Editable = false; field(Name; Rec.Name) { } } } }
    actions { area(Processing) { action(Refill) { trigger OnAction() begin Fill(1); end; } } }
    trigger OnOpenPage() begin Fill(RowsAtOpen); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Rec.Name); end;
    CURRENT
    var L: Codeunit "TR Log"; RowsAtOpen: Integer;
    procedure SetRows(N: Integer) begin RowsAtOpen := N; end;
    local procedure Fill(N: Integer) var I: Integer;
    begin
        Rec.Reset(); Rec.DeleteAll();
        for I := 1 to N do begin Rec.Init(); Rec."Entry No." := I; Rec.Name := StrSubstNo('R%1', I); Rec.Insert(); end;
        if Rec.FindFirst() then;
    end;
}
table 51490 "TR Row" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } } keys { key(PK; "Entry No.") { } } }
codeunit 51492 "TR Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
codeunit 51493 C
{
    procedure Run(N: Integer; StandOn: Integer): Text
    var Ws: TestPage "TR Ws"; P: Page "TR Ws"; L: Codeunit "TR Log"; I: Integer; s: Text; Rows: Text;
    begin
        Ws.Trap(); P.SetRows(N); P.Run();
        Ws.First();
        for I := 2 to StandOn do Ws.Next();
        L.Take();
        Ws.Refill.Invoke();
        s := L.Take() + '|' + Ws.Name.Value() + '|';
        if Ws.First() then repeat Rows += Ws.Name.Value() + ' '; until not Ws.Next();
        Ws.Close();
        exit(s + Rows);
    end;
    procedure Three(): Text begin exit(Run(3, 1)); end;
    procedure TwentyFive(): Text begin exit(Run(25, 1)); end;
    procedure FromRow23(): Text begin exit(Run(25, 23)); end;
}
"#;

fn src(current: bool) -> String {
    PAGE.replace(
        "CURRENT",
        if current {
            "trigger OnAfterGetCurrRecord() begin L.Add('C:' + Rec.Name); end;"
        } else {
            ""
        },
    )
}

fn run(current: bool, routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(&src(current), "C", routine))
}

#[test]
fn with_on_after_get_curr_record_the_client_reads_the_one_row_three_times() {
    assert_eq!(run(true, "Three"), "R:R1;R:R1;R:R1;R:R1;C:R1;|R1|R1 ");
    assert_eq!(run(true, "TwentyFive"), "R:R1;R:R1;R:R1;R:R1;C:R1;|R1|R1 ");
    assert_eq!(run(true, "FromRow23"), "R:R23;R:R1;R:R1;R:R1;C:R1;|R1|R1 ");
}

#[test]
fn without_it_the_client_reads_the_one_row_twice() {
    assert_eq!(run(false, "Three"), "R:R1;R:R1;R:R1;|R1|R1 ");
    assert_eq!(run(false, "TwentyFive"), "R:R1;R:R1;R:R1;|R1|R1 ");
}
