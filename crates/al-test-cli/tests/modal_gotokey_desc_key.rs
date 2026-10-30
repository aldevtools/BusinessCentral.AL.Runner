//! A modal lookup (Page.RunModal with a blank record) over a list that sorts descending on a key other than the primary key and
//! declares OnAfterGetRecord and OnAfterGetCurrRecord (the shape of Base App page 143 "Posted Sales Invoices", al-runner issue
//! 6363): the open reads the last row and the rows above it, GoToKey reads from the top row held up to the first and then moves
//! down row by row. Mirrors corpus master 930653fe, pages/lt-wd/WdModalGoToKeyDescSecondary.al.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62600 "MG Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Posting Date"; Date) { } }
    keys { key(PK; "No.") { Clustered = true; } key(ByDate; "Posting Date") { } }
}
codeunit 62601 "MG Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ','; Lines += Line; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
page 62602 "MG Docs Desc"
{
    PageType = List; SourceTable = "MG Doc"; SourceTableView = sorting("Posting Date") order(descending); Editable = false;
    layout { area(Content) { repeater(Rows) { field(No; Rec."No.") { } } } }
    trigger OnAfterGetRecord() var L: Codeunit "MG Log"; begin L.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "MG Log"; begin L.Add('C:' + Rec."No."); end;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        r#"{OBJECTS}
codeunit 62690 T
{{
    Subtype = Test;
    var Wanted: Code[20]; Opening: Text; GoLog: Text; Current: Text; Second: Boolean;
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    local procedure Seed(Rows: Integer)
    var D: Record "MG Doc"; i: Integer;
    begin
        D.DeleteAll();
        for i := 1 to Rows do begin
            D."No." := 'D' + Format(i, 2, '<Integer,2><Filler Character,0>');
            D."Posting Date" := 20260101D + i;
            D.Insert();
        end;
    end;
    [ModalPageHandler] procedure Pick(var P: TestPage "MG Docs Desc")
    var L: Codeunit "MG Log";
    begin
        Opening := L.Take();
        P.GoToKey(Wanted);
        GoLog := L.Take();
        Current := P.No.Value();
        if Second then P.GoToKey(Wanted);
        P.OK().Invoke();
    end;
    {tests}
}}"#
    );
    al_testkit::outcomes_in(&src, "T")
}

fn rows(from: usize, to: usize, times: usize, current: bool) -> Vec<String> {
    let mut out = Vec::new();
    let step: Vec<usize> = if from <= to {
        (from..=to).collect()
    } else {
        (to..=from).rev().collect()
    };
    for i in step {
        for _ in 0..times {
            out.push(format!("R:D{i:02}"));
        }
        if current {
            out.push(format!("C:D{i:02}"));
        }
    }
    out
}

#[test]
fn five_rows_open_on_the_last_and_gotokey_walks_down_from_the_first() {
    // WdModalGoToKeyDescSecondary.al WD_ModalGoToKey_Descending_FiveRows.
    let opening = [rows(1, 5, 1, false), rows(1, 1, 3, true)].concat().join(",");
    let walk = rows(5, 2, 3, true).join(",");
    let o = outcomes(&format!(
        r#"[Test] [HandlerFunctions('Pick')] procedure Five()
        var D: Record "MG Doc";
        begin
            Seed(5); Wanted := 'D02';
            Check('LookupOK', Format(Page.RunModal(Page::"MG Docs Desc", D)));
            Check('{opening}', Opening); Check('{walk}', GoLog); Check('D02', Current); Check('D02', D."No.");
        end;"#
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

#[test]
fn thirty_rows_read_the_rows_above_the_window_before_the_walk() {
    // WdModalGoToKeyDescSecondary.al WD_ModalGoToKey_Descending_Last.
    let opening = [rows(1, 21, 1, false), rows(1, 1, 3, true)].concat().join(",");
    let walk = [rows(21, 29, 1, false), rows(30, 1, 3, true)].concat().join(",");
    let o = outcomes(&format!(
        r#"[Test] [HandlerFunctions('Pick')] procedure Thirty()
        var D: Record "MG Doc";
        begin
            Seed(30); Wanted := 'D01';
            Check('LookupOK', Format(Page.RunModal(Page::"MG Docs Desc", D)));
            Check('{opening}', Opening); Check('{walk}', GoLog); Check('D01', Current);
        end;"#
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

#[test]
fn a_second_gotokey_after_the_walk_is_refused() {
    let o = outcomes(
        r#"[Test] [HandlerFunctions('Pick')] procedure Again()
        var D: Record "MG Doc";
        begin
            Seed(5); Wanted := 'D02'; Second := true;
            Page.RunModal(Page::"MG Docs Desc", D);
        end;"#,
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("not pinned")),
        "{o:?}"
    );
}
