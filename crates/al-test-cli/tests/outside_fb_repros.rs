//! The reporter's repros of al-runner issues 6114 to 6119 (green on real BC), run here until the corpus writer's pins land:
//! copies of the app and test files in fixtures/outside-fb, with a stand-in for Microsoft's Library Assert. Each test names
//! the outcome every test codeunit of a repro must have. Delete this file and the fixtures when the pins are in the corpus.

use al_test_cli::report::Outcome;

fn outcomes(file: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let src = format!("{}\n{}", file, include_str!("fixtures/outside-fb/LibraryAssert.al"));
    let o = al_testkit::outcomes_in(&src, codeunit);
    assert!(!o.is_empty());
    o
}

fn all_pass(file: &str, codeunit: &str) {
    let o = outcomes(file, codeunit);
    let bad: Vec<_> = o.iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Issue 6119: Expand and collapse on a tree page with OnAfterGetRecord and OnAfterGetCurrRecord run no row trigger.
#[test]
fn issue_6119_tree_expand_with_row_triggers() {
    all_pass(include_str!("fixtures/outside-fb/Issue6119.al"), "Tree Probe Tests");
}

/// Issue 6117: GoToKey and GoToRecord onto the last row of a full block of rows read, with more rows after it.
#[test]
fn issue_6117_gotokey_last_row_of_full_block() {
    let file = include_str!("fixtures/outside-fb/Issue6117.al");
    all_pass(file, "Block Edge Tests");
    all_pass(file, "Block Edge Controls");
}

/// Issue 6114: Page.Run with a filtered record that names no row, on a list whose SourceTableView is order(descending), enters
/// on the first row of the view; RunModal enters on its last row.
#[test]
fn issue_6114_page_run_descending_view_enters_first_row() {
    all_pass(include_str!("fixtures/outside-fb/Issue6114.al"), "View Order Tests");
}

/// Issue 6118: TestField.Lookup whose OnValidate calls a ListPart procedure that runs CurrPage.Update(false) on the part shows the
/// picked value.
#[test]
fn issue_6118_lookup_onvalidate_updates_the_part() {
    all_pass(
        include_str!("fixtures/outside-fb/Issue6118.al"),
        "Repro Lookup Part Update Tests",
    );
}

/// Issue 6115: with a ListPart on the page, GoToRecord of Rec's row after a marking OnDrillDown, in a secondary key (the
/// reporter's test) and in the primary key, with the part's row read after it as corpus writer TW pins it
/// (pages/lt-qr-onopenpage-moves-rec/SuRefillKey.al:518 TW_NameKeyWithPart_GoToRecordOfRecsRow `part` `D1`, :562
/// TW_IdKeyWithPart_GoToRecordOfRecsRow `goto 3` `Yes/3:R3;R1;R3;R3;C3;`, `part` `D3`). A GoToRecord of another row stays refused
/// with a part, in either key (the walk TW's :541 and :585 pin is not built).
#[test]
fn issue_6115_marked_refill_with_a_part() {
    let file = include_str!("fixtures/outside-fb/Issue6115.al");
    let o = outcomes(file, "Refill Probe Key Tests");
    for (n, r) in &o {
        if n.ends_with("RecsRow") {
            assert_eq!(r, &Outcome::Pass, "{n}");
        } else {
            assert!(matches!(r, Outcome::NotRun(_)), "{n}: {r:?}");
        }
    }
    let o = outcomes(file, "Refill Probe Id Tests");
    for (n, r) in &o {
        if n.ends_with("RecsRow") {
            assert_eq!(r, &Outcome::Pass, "{n}");
        } else {
            assert!(matches!(r, Outcome::NotRun(_)), "{n}: {r:?}");
        }
    }
    let part_reads = format!("{file}\n{PART_READS}");
    all_pass(&part_reads, "Refill Probe Part Reads");
}

const PART_READS: &str = r#"
codeunit 50130 "Refill Probe Part Reads"
{
    Subtype = Test;

    var
        Assert: Codeunit "Library Assert";
        Log: Codeunit "Refill Probe Log";

    [Test]
    procedure NameKey()
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Name List";
    begin
        Seed();
        P.OpenView();
        P.FlaggedTile.Drilldown();
        Row.Get(1);
        Assert.AreEqual(true, P.GoToRecord(Row), 'goto 1');
        Assert.AreEqual('D1', P.Details.Description.Value(), 'part');
        P.Close();
    end;

    [Test]
    procedure IdKey()
    var
        Row: Record "Refill Probe Row";
        P: TestPage "Refill Probe Id List";
    begin
        Seed();
        P.OpenView();
        Log.Take();
        P.FlaggedTile.Drilldown();
        Assert.AreEqual('1:', P.Id.Value() + ':' + Log.Take(), 'drilldown');
        Row.Get(3);
        Assert.AreEqual('Yes/3:R3;R1;R3;R3;C3;', Format(P.GoToRecord(Row)) + '/' + P.Id.Value() + ':' + Log.Take(), 'goto 3');
        Assert.AreEqual('D3', P.Details.Description.Value(), 'part');
        P.Close();
    end;

    local procedure Seed()
    var
        Row: Record "Refill Probe Row";
        Detail: Record "Refill Probe Detail";
        i: Integer;
    begin
        Row.DeleteAll();
        Detail.DeleteAll();
        for i := 1 to 4 do begin
            Row.Init();
            Row.Id := i;
            Row.Name := 'N' + Format(5 - i);
            Row.Flag := i in [1, 3];
            Row.Insert();
            Detail.Init();
            Detail.Id := i;
            Detail."Line No." := 10000;
            Detail.Description := 'D' + Format(i);
            Detail.Insert();
        end;
        Log.Take();
    end;
}
"#;
