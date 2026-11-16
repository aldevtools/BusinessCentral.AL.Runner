//! The reporter's repros of al-runner issues 6114 to 6119, 7958 and 8186 (green on real BC), run here until the corpus writer's pins land:
//! copies of the app and test files in fixtures/outside-fb, with a stand-in for Microsoft's Library Assert. Each test names
//! the outcome every test codeunit of a repro must have. Delete this file and the fixtures when the pins are in the corpus.

use al_test_cli::report::Outcome;

fn outcomes(file: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let src = format!("{}\n{}", file, include_str!("../fixtures/outside-fb/LibraryAssert.al"));
    let o = al_testkit::outcomes_in(&src, codeunit);
    assert!(!o.is_empty());
    o
}

fn all_pass(file: &str, codeunit: &str) {
    let o = outcomes(file, codeunit);
    let bad: Vec<_> = o.iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Issue 7958: a Price column (DecimalPlaces = 0 : 5) over a variable set from Round keeps the precision's decimals in the
/// XML dataset (`2.50`, `3.00`, `2.50`, `1.20000`), while `Format(..., 0, 9)` of the same values drops them. Measured by the
/// reporter on BC 29.1; tests/al-language/reports/dataset/DatasetDecimalRoundScaleSources.al:255
/// (Dataset_Decimal_RoundScale_VariableColumnWithDecimalPlaces_FourRows) pins it.
#[test]
fn issue_7958_round_keeps_the_precision_decimals_in_the_dataset() {
    all_pass(
        include_str!("../fixtures/outside-fb/Issue7958.al"),
        "Repro Round Scale Tests",
    );
}

/// Issue 6119: Expand and collapse on a tree page with OnAfterGetRecord and OnAfterGetCurrRecord run no row trigger.
#[test]
fn issue_6119_tree_expand_with_row_triggers() {
    all_pass(include_str!("../fixtures/outside-fb/Issue6119.al"), "Tree Probe Tests");
}

/// Issue 6117: GoToKey and GoToRecord onto the last row of a full block of rows read, with more rows after it.
#[test]
fn issue_6117_gotokey_last_row_of_full_block() {
    let file = include_str!("../fixtures/outside-fb/Issue6117.al");
    all_pass(file, "Block Edge Tests");
    all_pass(file, "Block Edge Controls");
}

/// Issue 6114: Page.Run with a filtered record that names no row, on a list whose SourceTableView is order(descending), enters
/// on the first row of the view; RunModal enters on its last row.
#[test]
fn issue_6114_page_run_descending_view_enters_first_row() {
    all_pass(include_str!("../fixtures/outside-fb/Issue6114.al"), "View Order Tests");
}

/// Issue 6118: TestField.Lookup whose OnValidate calls a ListPart procedure that runs CurrPage.Update(false) on the part shows the
/// picked value.
#[test]
fn issue_6118_lookup_onvalidate_updates_the_part() {
    all_pass(
        include_str!("../fixtures/outside-fb/Issue6118.al"),
        "Repro Lookup Part Update Tests",
    );
}

/// Issue 6115: with a ListPart on the page, GoToRecord of Rec's row after a marking OnDrillDown, in a secondary key (the
/// reporter's test) and in the primary key, with the part's row read after it as corpus writer TW pins it
/// (pages/onopenpage-moves-rec/SuRefillKey.al:518 TW_NameKeyWithPart_GoToRecordOfRecsRow `part` `D1`, :562
/// TW_IdKeyWithPart_GoToRecordOfRecsRow `goto 3` `Yes/3:R3;R1;R3;R3;C3;`, `part` `D3`). A GoToRecord of another row walks the
/// marked rows and answers false, in either key (TW's :541 and :585; al-runner issue 6141).
#[test]
fn issue_6115_marked_refill_with_a_part() {
    let file = include_str!("../fixtures/outside-fb/Issue6115.al");
    all_pass(file, "Refill Probe Key Tests");
    all_pass(file, "Refill Probe Id Tests");
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

/// The corpus probe of issue 8186 (corpus pull request 1425, open: tests/al-language/pages/currpage/ListNewLineCodeInsertUpdate.al,
/// measured on BC 29.1) with a stand-in for the corpus's Assert, and the reporter's repro: each test's outcome and the assumptions
/// it ran through.
fn issue_8186_outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<al_builtins::assumptions::Assumption>)> {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

const ISSUE_8186_PROBE: &str = include_str!("../fixtures/outside-fb/Issue8186Probe.al");

/// Every test passes, flagged by the assumption list-new-line-code-insert-update, except those on the host with CardPart
/// FactBoxes: its parts read at steps the runner does not model (at the open and at `New()` already, on main too), so the new path
/// does not take them.
#[test]
fn issue_8186_code_inserted_new_line_lands_on_the_row() {
    use al_builtins::assumptions::Assumption::ListNewLineCodeInsertUpdate;
    for (src, codeunit) in [
        (ISSUE_8186_PROBE, "LT NI Tests"),
        (
            include_str!("../fixtures/outside-fb/Issue8186.al"),
            "Repro Update New Line Tests",
        ),
    ] {
        let bad: Vec<_> = issue_8186_outcomes(src, codeunit)
            .into_iter()
            .filter(|(n, o, marks)| match n.contains("PartsHost") {
                true => *o == Outcome::Pass || marks.contains(&ListNewLineCodeInsertUpdate),
                false => *o != Outcome::Pass || !marks.contains(&ListNewLineCodeInsertUpdate),
            })
            .collect();
        assert!(bad.is_empty(), "{codeunit}: {bad:#?}");
    }
}

/// The shapes the probe does not measure keep the refusal: DelayedInsert, a Worksheet, `Update(true)` written out, a page with a
/// part.
#[test]
fn issue_8186_unmeasured_neighbors_stay_refused() {
    let with_part = "area(FactBoxes) { part(Peek; \"LT NI Peek X\") { SubPageLink = \"Entry No.\" = field(\"Entry No.\"); } }\n        area(Content)";
    let peek = "page 97041 \"LT NI Peek X\" { PageType = CardPart; SourceTable = \"LT NI Line\"; layout { area(Content) { field(Name; Rec.Name) { } } } }";
    for (what, src) in [
        (
            "DelayedInsert",
            ISSUE_8186_PROBE.replace(
                "AutoSplitKey = true;",
                "AutoSplitKey = true;\n    DelayedInsert = true;",
            ),
        ),
        (
            "Worksheet",
            ISSUE_8186_PROBE.replace("PageType = List;", "PageType = Worksheet;"),
        ),
        (
            "Update(true)",
            ISSUE_8186_PROBE.replace("CurrPage.Update();", "CurrPage.Update(true);"),
        ),
        (
            "a part",
            format!("{}\n{peek}", ISSUE_8186_PROBE.replacen("area(Content)", with_part, 1)),
        ),
    ] {
        let o = issue_8186_outcomes(&src, "LT NI Tests");
        // The tests on page 97032, which the variants change (its Update(false) cells keep `Update(false)` under the Update(true)
        // variant, and stay measured).
        let split: Vec<_> = o
            .iter()
            .filter(|(n, _, _)| {
                n.starts_with("ListNewLine_FieldValidateInserts") || n.starts_with("ListNewLine_RunModal_")
            })
            .filter(|(n, _, _)| what != "Update(true)" || !n.contains("UpdateFalse"))
            .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
            .collect();
        assert!(split.is_empty(), "{what}: {split:#?}");
    }
}
