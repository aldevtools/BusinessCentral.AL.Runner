//! GoToKey and GoToRecord at the edges of the client's row scan (pages/gotokey_find.rs): a find list with all four row triggers
//! (a key in the second block of rows, the first row, the current row, a miss from a later row) and a list with
//! OnAfterGetCurrRecord only (a miss). Each test names the corpus test of pages/lt-y61-page-probes/Y61GoToKeyEdges.al that pins it.

const SRC: &str = r#"
table 69980 "GE Log" { fields { field(1; Id; Integer) { } field(2; Txt; Text[250]) { } } keys { key(PK; Id) { } } }
codeunit 69981 "GE Log Mgt"
{
    procedure Add(Value: Text)
    var L: Record "GE Log"; N: Integer;
    begin
        if L.FindLast() then N := L.Id;
        L.Init(); L.Id := N + 1; L.Txt := CopyStr(Value, 1, 250); L.Insert();
    end;
    procedure Take(): Text
    var L: Record "GE Log"; R: Text;
    begin
        if L.FindSet() then
            repeat
                if R <> '' then R += ';';
                R += L.Txt;
            until L.Next() = 0;
        L.DeleteAll();
        exit(R);
    end;
}
table 69982 "GE Row" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
page 69983 "GE Find"
{
    PageType = List; SourceTable = "GE Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnFindRecord(Which: Text): Boolean
    var L: Codeunit "GE Log Mgt"; Found: Boolean;
    begin
        Found := Rec.Find(Which);
        L.Add('F:' + Which + '=' + Format(Found));
        exit(Found);
    end;
    trigger OnNextRecord(Steps: Integer): Integer
    var L: Codeunit "GE Log Mgt"; Moved: Integer;
    begin
        Moved := Rec.Next(Steps);
        L.Add('N:' + Format(Steps) + '=' + Format(Moved));
        exit(Moved);
    end;
    trigger OnAfterGetRecord() var L: Codeunit "GE Log Mgt"; begin L.Add('R:' + CopyStr(Rec."No.", 2)); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "GE Log Mgt"; begin L.Add('C:' + CopyStr(Rec."No.", 2)); end;
}
page 69984 "GE Curr"
{
    PageType = List; SourceTable = "GE Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnAfterGetCurrRecord() var L: Codeunit "GE Log Mgt"; begin L.Add('C:' + CopyStr(Rec."No.", 2)); end;
}
page 69985 "GE Curr Temp"
{
    PageType = List; SourceTable = "GE Row"; SourceTableTemporary = true; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage()
    begin
        Rec.Init(); Rec."No." := 'E1'; Rec.Insert();
        Rec.Init(); Rec."No." := 'E2'; Rec.Insert();
        Rec.FindFirst();
    end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "GE Log Mgt"; begin L.Add('C:' + CopyStr(Rec."No.", 2)); end;
}
codeunit 69986 C
{
    procedure Seed(N: Integer)
    var Row: Record "GE Row"; i: Integer; L: Codeunit "GE Log Mgt";
    begin
        Row.DeleteAll();
        for i := 1 to N do begin Row.Init(); Row."No." := 'N' + PadStr('', 3 - StrLen(Format(i)), '0') + Format(i); Row.Insert(); end;
        L.Take();
    end;

    [TryFunction]
    local procedure TryFind(var P: TestPage "GE Find"; No: Code[20])
    begin
        P.GoToKey(No);
    end;

    [TryFunction]
    local procedure TryCurr(var P: TestPage "GE Curr"; No: Code[20])
    begin
        P.GoToKey(No);
    end;

    [TryFunction]
    local procedure TryTemp(var P: TestPage "GE Curr Temp"; var Row: Record "GE Row")
    begin
        P.GoToRecord(Row);
    end;

    local procedure Unused(Ok: Boolean): Text
    begin
        if Ok then exit('ok');
        exit(GetLastErrorText() + '/' + GetLastErrorCode());
    end;

    procedure Find45SecondBlock(): Text
    var P: TestPage "GE Find"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(45);
        P.OpenView();
        L.Take();
        R := Format(P.GoToKey('N033')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        R += '|' + Format(P.Next()) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;

    procedure Find25FirstRow(): Text
    var P: TestPage "GE Find"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(25);
        P.OpenView();
        L.Take();
        R := Format(P.GoToKey('N001')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        P.GoToKey('N010');
        L.Take();
        R += '|' + Format(P.GoToKey('N001')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;

    procedure Find25CurrentRow(): Text
    var P: TestPage "GE Find"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(25);
        P.OpenView();
        P.GoToKey('N010');
        L.Take();
        R := Format(P.GoToKey('N010')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        R += '|' + Format(P.Next()) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;

    procedure Find25Miss(): Text
    var P: TestPage "GE Find"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(25);
        P.OpenView();
        P.GoToKey('N010');
        L.Take();
        R := Format(P.GoToKey('NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        P.GoToKey('N010');
        L.Take();
        R += '|' + Unused(TryFind(P, 'NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;

    procedure CurrMissFirst(): Text
    var P: TestPage "GE Curr"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(3);
        P.OpenView();
        L.Take();
        R := Format(P.GoToKey('NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        R += '|' + Unused(TryCurr(P, 'NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;

    procedure CurrMissLater(): Text
    var P: TestPage "GE Curr"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(3);
        P.OpenView();
        P.GoToKey('N003');
        L.Take();
        R := Format(P.GoToKey('NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        P.GoToKey('N002');
        L.Take();
        R += '|' + Unused(TryCurr(P, 'NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;

    procedure CurrMissOne(): Text
    begin
        exit(CurrMissFew(1));
    end;

    procedure CurrMissNone(): Text
    begin
        exit(CurrMissFew(0));
    end;

    local procedure CurrMissFew(N: Integer): Text
    var P: TestPage "GE Curr"; L: Codeunit "GE Log Mgt";
    begin
        Seed(N);
        P.OpenView();
        L.Take();
        exit(Format(P.GoToKey('NOPE')) + '|' + L.Take() + '|' + P.NoCtl.Value());
    end;

    procedure TempHitMiss(): Text
    var P: TestPage "GE Curr Temp"; Row: Record "GE Row"; L: Codeunit "GE Log Mgt"; R: Text;
    begin
        Seed(0);
        P.OpenView();
        L.Take();
        Row."No." := 'E2';
        R := Format(P.GoToRecord(Row)) + '|' + L.Take() + '|' + P.NoCtl.Value();
        Row."No." := 'E9';
        R += '|' + Format(P.GoToRecord(Row)) + '|' + L.Take() + '|' + P.NoCtl.Value();
        R += '|' + Unused(TryTemp(P, Row)) + '|' + L.Take() + '|' + P.NoCtl.Value();
        exit(R);
    end;
}
"#;

const NOT_FOUND: &str = "The row does not exist on the TestPage./TestRowNotFound";

/// The move of the find list onto row `n` from the row before it: `R:x;F:==Yes;R:x;C:x`.
fn step(n: usize) -> String {
    format!("R:{n:03};F:==Yes;R:{n:03};C:{n:03}")
}

/// The steps onto rows `from..=to`.
fn steps(from: usize, to: usize) -> Vec<String> {
    (from..=to).map(step).collect()
}

/// Arriving on the last row the client holds (`last`) with more to read: the row, then the rows up to `to` with OnNextRecord(1);
/// `eof` adds the call that finds no more.
fn block(last: usize, to: usize, eof: bool) -> Vec<String> {
    let mut v = vec![format!("R:{last:03}")];
    for n in last + 1..=to {
        v.push("N:1=1".into());
        v.push(format!("R:{n:03}"));
    }
    if eof {
        v.push("N:1=0".into());
    }
    v
}

fn run(proc_name: &str) -> Vec<String> {
    let mut built = al_testkit::Built::from_source(SRC);
    let out = al_testkit::text(built.run_in("C", proc_name));
    out.split('|').map(str::to_string).collect()
}

/// Y61GoToKeyEdges.al:212 G613_Find_45Rows_KeyInSecondBlock: the walk reads one more block of 20 rows (to row 40, without the call
/// that would end the view) on arriving on row 20, and goes on to row 33; Next then moves on as usual.
#[test]
fn a_key_in_the_second_block_of_a_find_list_walks_into_it() {
    let out = run("Find45SecondBlock");
    let mut want = steps(2, 19);
    want.extend(block(20, 40, false));
    want.extend(steps(20, 33));
    assert_eq!(out[0], "Yes");
    assert_eq!(out[1], want.join(";"));
    assert_eq!(out[2], "N033");
    assert_eq!(&out[3..], ["Yes", &step(34), "N034"]);
}

/// Y61GoToKeyEdges.al:241 G613_Find_25Rows_KeyIsFirstRow: the first row while standing on it reads nothing; from row 10 it is one move.
#[test]
fn the_first_row_of_a_find_list_from_itself_and_from_a_later_row() {
    let out = run("Find25FirstRow");
    assert_eq!(&out[..3], ["Yes", "", "N001"]);
    assert_eq!(&out[3..], ["Yes", &step(1), "N001"]);
}

/// Y61GoToKeyEdges.al:270 G613_Find_25Rows_KeyIsCurrentRow_NotFirst: the walk starts again from the first row.
#[test]
fn the_current_row_of_a_find_list_walks_from_the_first_row() {
    let out = run("Find25CurrentRow");
    assert_eq!(&out[..3], ["Yes", &steps(1, 10).join(";"), "N010"]);
    assert_eq!(&out[3..], ["Yes", &step(11), "N011"]);
}

/// Y61GoToKeyEdges.al:299 G613_Find_25Rows_Miss_FromLaterRow: a miss walks every row (reading the rest of the view on row 20) and moves
/// back to the row it started from; with the result unused the same walk (the view now held whole) raises the row error.
#[test]
fn a_miss_on_a_find_list_from_a_later_row_walks_back_to_it() {
    let out = run("Find25Miss");
    let mut first = steps(1, 19);
    first.extend(block(20, 25, true));
    first.extend(steps(20, 25));
    first.push(step(10));
    assert_eq!(&out[..3], ["No", &first.join(";"), "N010"]);
    let mut second = steps(1, 25);
    second.push(step(10));
    assert_eq!(&out[3..], [NOT_FOUND, &second.join(";"), "N010"]);
}

/// Y61GoToKeyEdges.al:334 G613_Current_Miss_FromFirstRow: every other row, then back to the first.
#[test]
fn a_miss_on_a_current_only_list_from_the_first_row() {
    let out = run("CurrMissFirst");
    assert_eq!(
        out,
        [
            "No",
            "C:002;C:003;C:001",
            "N001",
            NOT_FOUND,
            "C:002;C:003;C:001",
            "N001"
        ]
    );
}

/// Y61GoToKeyEdges.al:408 G613_Current_Miss_FromLaterRow: from the last row the walk ends there; from row 2 it goes back to it.
#[test]
fn a_miss_on_a_current_only_list_from_a_later_row() {
    let out = run("CurrMissLater");
    assert_eq!(
        out,
        [
            "No",
            "C:001;C:002;C:003",
            "N003",
            NOT_FOUND,
            "C:001;C:002;C:003;C:002",
            "N002"
        ]
    );
}

/// Y61GoToKeyEdges.al:444 G613_Current_Miss_OneRow and :463 G613_Current_Miss_Empty: No and nothing runs.
#[test]
fn a_miss_on_a_current_only_list_of_one_row_or_none() {
    assert_eq!(run("CurrMissOne"), ["No", "", "N001"]);
    assert_eq!(run("CurrMissNone"), ["No", "", ""]);
}

/// Y61GoToKeyEdges.al:365 G613_Current_Temp_GoToRecord_HitAndMiss: a temporary source filled in OnOpenPage.
#[test]
fn go_to_record_on_a_temporary_current_only_list() {
    let out = run("TempHitMiss");
    assert_eq!(
        out,
        ["Yes", "C:2", "E2", "No", "C:1;C:2", "E2", NOT_FOUND, "C:1;C:2", "E2"]
    );
}
