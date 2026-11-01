//! The reads of a descending list that serves its rows from OnFindRecord and OnNextRecord, with OnAfterGetCurrRecord, when a
//! TestPage opens it and moves Next, Last, Previous and First (misc/lt-probes-needed-6/PnF06FindListMoves.al, codeunit 73874,
//! the PW_FindList_Descending_* tests).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
codeunit 62800 "FD Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take() R: Text begin R := Entries; Entries := ''; end;
}
table 62801 "FD Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
page 62802 "FD Desc"
{
    PageType = List; SourceTable = "FD Row";
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } } } }
    var Log: Codeunit "FD Log";
    trigger OnOpenPage() begin Rec.Ascending(false); end;
    trigger OnFindRecord(Which: Text): Boolean begin Log.Add('F(' + Which + ')'); exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer begin Log.Add('N(' + Format(Steps) + ')'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() begin Log.Add('R' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C'); end;
}
"#;

const TESTS: &str = r#"
    var Log: Codeunit "FD Log";
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    local procedure Moves(Rows: Integer; Open: Text; Next: Text; Last: Text; Prev: Text; First: Text)
    var Row: Record "FD Row"; P: TestPage "FD Desc"; i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to Rows do begin Row.Id := i; Row.Insert(); end;
        Log.Take();
        P.OpenView(); Check(Open, Log.Take());
        P.Next(); Check(Next, Log.Take());
        P.Last(); Check(Last, Log.Take());
        P.Previous(); Check(Prev, Log.Take());
        P.First(); Check(First, Log.Take());
        P.Close();
    end;
    // PnF06FindListMoves.al:348 PW_FindList_Descending_1Rows.
    [Test] procedure OneRow() begin Moves(1, 'F(=><);R1;N(1);N(-1);F(=);R1;C;', '', '', '', ''); end;
    // :362 PW_FindList_Descending_2Rows.
    [Test] procedure TwoRows() begin Moves(2, 'F(=><);R1;N(1);N(-1);R2;N(-1);R1;F(=);R1;C;', '', '', 'R2;F(=);R2;C;', ''); end;
    // :404 PW_FindList_Descending_21Rows.
    [Test] procedure TwentyOneRows() begin
        Moves(21, 'F(=><);R1;N(1);N(-1);R2;N(-1);R3;N(-1);R4;N(-1);R5;N(-1);R6;N(-1);R7;N(-1);R8;N(-1);R9;N(-1);R10;N(-1);R11;N(-1);R12;N(-1);R13;N(-1);R14;N(-1);R15;N(-1);R16;N(-1);R17;N(-1);R18;N(-1);R19;N(-1);R20;N(-1);R21;R1;F(=);R1;C;',
            '', '', 'R2;F(=);R2;C;', 'R21;N(-1);R21;F(=);R21;C;'); end;
"#;

#[test]
fn descending_find_list_reads_at_one_two_and_twenty_one_rows() {
    let src = format!("{OBJECTS}\ncodeunit 62890 T {{ Subtype = Test; {TESTS} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 3);
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// Previous on the top row of a descending find list with more than one row is not pinned (only one row is, :348): refused.
#[test]
fn previous_on_the_top_row_of_a_two_row_descending_find_list_is_refused() {
    let src = format!(
        "{OBJECTS}\ncodeunit 62891 T {{ Subtype = Test;
        [Test] procedure Top() var Row: Record \"FD Row\"; P: TestPage \"FD Desc\"; begin
            Row.DeleteAll(); Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();
            P.OpenView(); P.Previous(); P.Previous(); P.Close(); end;
        }}"
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 1);
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("Previous")),
        "{o:?}"
    );
}
