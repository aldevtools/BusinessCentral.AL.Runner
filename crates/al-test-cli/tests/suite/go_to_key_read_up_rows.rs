//! GoToKey in a ModalPageHandler on a list that opened on the last row of its descending view in a secondary key (al-runner issue
//! 8172; corpus testpage/WdModalGoToKeyDescSecondary.al). The corpus measures two cells of rows held and rows above them: 21 held
//! with 9 above (30 rows, :146 and :165) and 5 held with none above (5 rows, :184). Every other count is refused.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62790 "GKU Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Posting Date"; Date) { } }
    keys { key(PK; "No.") { Clustered = true; } key(ByDate; "Posting Date") { } }
}
codeunit 62791 "GKU Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ','; Lines += Line; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
page 62792 "GKU Docs"
{
    PageType = List; SourceTable = "GKU Doc"; SourceTableView = sorting("Posting Date") order(descending);
    Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(No; Rec."No.") { } field(PostingDate; Rec."Posting Date") { } } } }
    trigger OnAfterGetRecord() var Log: Codeunit "GKU Log"; begin Log.Add('R:' + Rec."No."); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "GKU Log"; begin Log.Add('C:' + Rec."No."); end;
}
"#;

/// The corpus test's shape (WdModalGoToKeyDescSecondary.al:131-160, :220-230: a blank record handed to RunModal) for `rows` rows and the wanted key `want`.
fn outcome(rows: u32, want: &str) -> Outcome {
    outcome_from(rows, want, "Clear(Doc);")
}

/// The same with `before` in place of the Clear before RunModal.
fn outcome_from(rows: u32, want: &str, before: &str) -> Outcome {
    let src = format!(
        "{OBJECTS}
codeunit 62793 T
{{
    Subtype = Test;
    var WantedNo: Code[20]; GoLog: Text;
    [Test] [HandlerFunctions('PickHandler')] procedure Go() var Doc: Record \"GKU Doc\"; i: Integer; begin
        Doc.DeleteAll();
        for i := 1 to {rows} do begin
            Doc.\"No.\" := 'D' + Format(i, 2, '<Integer,2><Filler Character,0>');
            Doc.\"Posting Date\" := 20260101D + i;
            Doc.Insert();
        end;
        {before}
        WantedNo := '{want}';
        if Page.RunModal(Page::\"GKU Docs\", Doc) <> Action::LookupOK then Error('answer');
        if Doc.\"No.\" <> '{want}' then Error('picked %1', Doc.\"No.\");
    end;
    [ModalPageHandler] procedure PickHandler(var Docs: TestPage \"GKU Docs\") var Log: Codeunit \"GKU Log\"; begin
        Log.Take();
        if not Docs.GoToKey(WantedNo) then Error('GoToKey');
        GoLog := Log.Take();
        Docs.OK().Invoke();
    end;
}}"
    );
    al_testkit::outcomes_in(&src, "T").remove(0).1
}

fn refused(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(why) if why.contains("GoToKey on a list that opened on the last row of its descending view"))
}

// The two measured cells run: 30 rows (:146 D01, :165 D30) and 5 rows (:184 D02).
#[test]
fn the_measured_counts_run() {
    assert_eq!(outcome(30, "D01"), Outcome::Pass);
    assert_eq!(outcome(30, "D30"), Outcome::Pass);
    assert_eq!(outcome(5, "D02"), Outcome::Pass);
}

// 35 rows: 14 rows above the 21 held, which no case measures (al-runner issue 8172).
#[test]
fn rows_above_that_no_case_measured_refuse() {
    for rows in [31, 35, 41] {
        let o = outcome(rows, "D01");
        assert!(refused(&o), "{rows} rows: {o:?}");
    }
}

// More rows above than a window holds keeps the refusal it had (the reporter's 60 rows).
#[test]
fn more_rows_above_than_a_window_refuse() {
    let o = outcome(60, "D01");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("with more rows above the rows it holds than a window")),
        "{o:?}"
    );
}

// No row above, but a number of rows held that no case measures: 6 (fewer than a window) and 21 (the open read filled it).
#[test]
fn rows_held_that_no_case_measured_refuse() {
    for rows in [6, 21] {
        let o = outcome(rows, "D02");
        assert!(refused(&o), "{rows} rows: {o:?}");
    }
}

// A list that opens on the first row of its view (RunModal gets the last row inserted) reads no rows up and keeps main's path: a
// GoToKey with 14 rows below the window runs as before.
#[test]
fn a_list_opened_on_its_first_row_keeps_its_path() {
    assert_eq!(outcome_from(35, "D01", ""), Outcome::Pass);
}
