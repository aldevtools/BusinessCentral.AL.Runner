//! The reporter's repros of al-runner issues 6114 to 6119, 7958, 8186 and 8285 (green on real BC), run here until the corpus writer's pins land:
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

/// The corpus probe of issue 8186 (tests/al-language/pages/currpage/ListNewLineCodeInsertUpdate.al) with a stand-in for the
/// corpus's Assert, and the reporter's repro: each test's outcome and the assumptions it ran through.
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
        al_testrun::host::RunClock::begin(),
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

/// Every test of the probe passes with no assumption, except those on the host with two CardPart FactBoxes: its parts read at
/// steps the runner does not model (at the open and at `New()` already), so its open refuses (ListNewLineCodeInsertUpdate.al:718,
/// :736). The reporter's repro, a RunModal filtered to one group on the page without triggers, is no pinned cell: refused.
#[test]
fn issue_8186_code_inserted_new_line_lands_on_the_row() {
    let bad: Vec<_> = issue_8186_outcomes(ISSUE_8186_PROBE, "LT NI Tests")
        .into_iter()
        .filter(|(n, o, marks)| match n.contains("PartsHost") {
            true => !matches!(o, Outcome::NotRun(why) if why.contains("two CardPart FactBoxes")),
            false => *o != Outcome::Pass || !marks.is_empty(),
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
    let repro = issue_8186_outcomes(
        include_str!("../fixtures/outside-fb/Issue8186.al"),
        "Repro Update New Line Tests",
    );
    let bad: Vec<_> = repro
        .iter()
        .filter(|(_, o, _)| !matches!(o, Outcome::NotRun(why) if why.contains("pins OpenEdit only")))
        .collect();
    assert!(!repro.is_empty() && bad.is_empty(), "{bad:#?}");
}

/// The parts host with one fact flipped (a SubPageLink of the key's first field only; a third page property) keeps main's open:
/// none of its tests carries the refusal of the pinned host.
#[test]
fn issue_8186_parts_host_neighbors_keep_main_open() {
    let (before, host) = ISSUE_8186_PROBE.split_once("page 97036 ").expect("the parts host");
    for (what, src) in [
        (
            "Editable",
            format!(
                "{before}page 97036 {}",
                host.replacen("PageType = List;", "PageType = List;\n    Editable = true;", 1)
            ),
        ),
        (
            "one link entry",
            ISSUE_8186_PROBE.replacen(
                "SubPageLink = \"Group Code\" = field(\"Group Code\"), \"Entry No.\" = field(\"Entry No.\");",
                "SubPageLink = \"Group Code\" = field(\"Group Code\");",
                1,
            ),
        ),
    ] {
        assert!(src != ISSUE_8186_PROBE, "{what}: the variant changes the probe");
        let o = issue_8186_outcomes(&src, "LT NI Tests");
        let hosts: Vec<_> = o.iter().filter(|(n, _, _)| n.contains("PartsHost")).collect();
        assert!(!hosts.is_empty(), "{what}");
        let refused: Vec<_> = hosts
            .iter()
            .filter(|(_, r, _)| matches!(r, Outcome::NotRun(why) if why.contains("two CardPart FactBoxes")))
            .collect();
        assert!(refused.is_empty(), "{what}: {refused:#?}");
    }
}

/// The shapes the probe does not measure keep the refusal: DelayedInsert, a Worksheet, `Update(true)` written out (under OpenEdit
/// and the unfiltered `Page.RunModal`: ListNewLineSaveRecordUpdate.al:494-526 pins it for its own page's code only), a page with
/// a part.
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
            // Under the Update(true) variant the page's code is not the code ListNewLineSaveRecordUpdate.al measures (its page 97472
            // runs Update(true) under OpenEdit, :494-526): refused as well.
            .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(_)))
            .collect();
        assert!(split.is_empty(), "{what}: {split:#?}");
    }
}

/// `Previous()` on the landed row answers as pinned, apart from the landing before it, with no assumption (a SingleInstance
/// codeunit keeps the page open between the two calls from Rust).
#[test]
fn issue_8186_previous_on_the_landed_row_answers_false() {
    use al_builtins::assumptions::{clear_hits, take_hits};
    let steps = r#"
codeunit 97098 "LT NI Steps"
{
    SingleInstance = true;

    var
        Lines: TestPage "LT NI Lines";
        Log: Codeunit "LT NI Log";

    procedure Land(): Text
    var
        Line: Record "LT NI Line";
    begin
        Line.DeleteAll();
        Log.Reset(false);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        exit(Lines.Name.Value() + '/' + Lines.Value.Value());
    end;

    procedure Prev(): Text
    begin
        Log.Take();
        exit(Log.Bool(Lines.Previous()) + '/' + Log.Take());
    end;
}
codeunit 97099 "LT NI Driver"
{
    procedure Land(): Text
    var
        Steps: Codeunit "LT NI Steps";
    begin
        exit(Steps.Land());
    end;

    procedure Prev(): Text
    var
        Steps: Codeunit "LT NI Steps";
    begin
        exit(Steps.Prev());
    end;
}
"#;
    let src = format!(
        "{ISSUE_8186_PROBE}\n{steps}\n{}",
        include_str!("../fixtures/outside-sc/Assert.al")
    );
    let mut b = al_testkit::Built::from_source(&src);
    clear_hits();
    assert_eq!(al_testkit::text(b.run_in("LT NI Driver", "Land")), "X/V");
    assert!(take_hits().is_empty(), "the landing");
    // ListNewLineCodeInsertUpdate.al:463-464: false, no trigger.
    assert_eq!(al_testkit::text(b.run_in("LT NI Driver", "Prev")), "F/");
    assert!(take_hits().is_empty(), "the Previous");
}

const ISSUE_8186_NEIGHBORS: &str = include_str!("../fixtures/outside-fb/Issue8186Neighbors.al");

/// The outcome and marks of each test of `codeunits` in Issue8186Neighbors.al (with the probe's objects), by test name.
fn issue_8186_neighbor_outcomes(
    codeunits: &[&str],
) -> Vec<(String, Outcome, Vec<al_builtins::assumptions::Assumption>)> {
    let src = format!("{ISSUE_8186_PROBE}\n{ISSUE_8186_NEIGHBORS}");
    codeunits.iter().flat_map(|c| issue_8186_outcomes(&src, c)).collect()
}

/// The refusal main gives the neighbors it refuses: the new path does not take them.
const MAIN_8186_REFUSAL: &str = "CurrPage.Update on a page that shows no row";

/// An OnValidate that fails after `CurrPage.Update()` or `CurrPage.Update(false)` moved the page onto the row (the error rolls the
/// insert back; `asserterror` around the SetValue): not measured, refused at the end of the validation.
#[test]
fn issue_8186_error_after_the_landing_is_refused() {
    let o = issue_8186_neighbor_outcomes(&["LT NI Neighbor Tests"]);
    for name in ["ErrorAfterUpdate_Update", "ErrorAfterUpdate_UpdateFalse"] {
        let (_, r, _) = o.iter().find(|(n, _, _)| n == name).expect("the test");
        assert!(
            matches!(r, Outcome::NotRun(why) if why.contains("an error ended the validation")),
            "{name}: {r:?}"
        );
    }
}

/// Shapes next to the measured cells keep main's refusal and carry no mark: a filter that leaves the inserted row out (the test's
/// `Filter.SetFilter`, a RunModal record filtered on another field), two rows before the line, a row after it, the control's
/// OnValidate inserting Rec itself, Trap and `Page.Run`, a read before the SetValue. `Update(false)` on the page without AutoSplitKey
/// keeps main's answer.
#[test]
fn issue_8186_unmeasured_views_and_opens_keep_main_refusal() {
    let o = issue_8186_neighbor_outcomes(&["LT NI Neighbor Tests", "LT NI Neighbor Tests 2"]);
    for name in [
        "FilterExcludesTheLine",
        "RunModalFilterExcludesTheLine",
        "TwoRowsBefore",
        "RowAfter",
        "ControlValidateInserts",
        "TrapAndPageRun",
        "ReadBeforeTheSetValue",
    ] {
        let (_, r, marks) = o.iter().find(|(n, _, _)| n == name).expect("the test");
        assert!(
            matches!(r, Outcome::NotRun(why) if why.contains(MAIN_8186_REFUSAL)) && marks.is_empty(),
            "{name}: {r:?} {marks:?}"
        );
    }
    // Main answers `Update(false)` on the page without AutoSplitKey: the close inserts the line a second time.
    let (_, r, marks) = o
        .iter()
        .find(|(n, _, _)| n == "NoAutoSplitKey_UpdateFalse")
        .expect("the test");
    assert!(
        matches!(r, Outcome::Fail(why) if why.contains("already exists")) && marks.is_empty(),
        "{r:?} {marks:?}"
    );
}

/// Page and table facts the probe does not vary keep main's refusal on every page of the probe: ModifyAllowed = false, a
/// descending SourceTableView, a primary key of the AutoIncrement field alone.
#[test]
fn issue_8186_unmeasured_page_and_key_keep_main_refusal() {
    for (what, src) in [
        (
            "ModifyAllowed",
            ISSUE_8186_PROBE.replace("PageType = List;", "PageType = List;\n    ModifyAllowed = false;"),
        ),
        (
            "SourceTableView",
            ISSUE_8186_PROBE.replace(
                "SourceTable = \"LT NI Line\";",
                "SourceTable = \"LT NI Line\";\n    SourceTableView = order(descending);",
            ),
        ),
        (
            "AutoIncrement key",
            ISSUE_8186_PROBE.replace("key(PK; \"Group Code\", \"Entry No.\")", "key(PK; \"Entry No.\")"),
        ),
    ] {
        let bad: Vec<_> = issue_8186_outcomes(&src, "LT NI Tests")
            .into_iter()
            .filter(|(_, r, marks)| !matches!(r, Outcome::NotRun(_)) || !marks.is_empty())
            .collect();
        assert!(bad.is_empty(), "{what}: {bad:#?}");
    }
}

/// The landing is forgotten when the page closes: the second OpenEdit of the same TestPage variable stands on the row the first
/// open's line became, and `Previous()` there saves the Value typed, as on a plain list.
#[test]
fn issue_8186_reopen_forgets_the_landing() {
    let o = issue_8186_neighbor_outcomes(&["LT NI Neighbor Tests"]);
    let (_, r, _) = o
        .iter()
        .find(|(n, _, _)| n == "ReopenAfterTheLanding_PreviousSaves")
        .expect("the test");
    assert_eq!(*r, Outcome::Pass);
}

const ISSUE_8285_PROBE: &str = include_str!("../fixtures/outside-fb/Issue8285Probe.al");
const ISSUE_8285_NEIGHBORS: &str = include_str!("../fixtures/outside-fb/Issue8285Neighbors.al");

/// The corpus probe of issue 8285 (pinned: tests/al-language/pages/currpage/ListNewLineSaveRecordUpdate.al) and the reporter's
/// repro: `CurrPage.SaveRecord()` then `CurrPage.Update(false)`, and `CurrPage.Update(true)`, on the new line the table field
/// inserted, also on the host with a CardPart FactBox. Every test passes: the probe's with no assumption, the repro's (whose code
/// is not the probe's) under list-new-line-save-record-repro.
#[test]
fn issue_8285_save_record_and_update_true_land_on_the_row() {
    use al_builtins::assumptions::Assumption::ListNewLineSaveRecordRepro;
    for (src, codeunit, want) in [
        (ISSUE_8285_PROBE, "LT NS Tests", vec![]),
        (
            include_str!("../fixtures/outside-fb/Issue8285.al"),
            "Repro New Line Update Tests",
            vec![ListNewLineSaveRecordRepro],
        ),
    ] {
        let o = issue_8186_outcomes(src, codeunit);
        let bad: Vec<_> = o
            .iter()
            .filter(|(_, r, marks)| *r != Outcome::Pass || *marks != want)
            .collect();
        assert!(!o.is_empty() && bad.is_empty(), "{codeunit}: {bad:#?}");
    }
}

/// The refusal of a SaveRecord or Update(true) cell whose code is not the measured code (`code_inserted_pinned.rs`).
const UNMEASURED_CODE: &str =
    "is not the code ListNewLineSaveRecordUpdate.al or the repro of al-runner issue 8285 runs";

/// The outcome and marks of each test of `codeunit` in Issue8285Neighbors.al, with the probe's objects; `host` changes the
/// FactBox host page 97483 (the part of the file from its declaration on), `probe` the probe's objects.
fn issue_8285_neighbor_outcomes(
    codeunit: &str,
    probe: impl Fn(&str) -> String,
    host: impl Fn(&str) -> String,
) -> Vec<(String, Outcome, Vec<al_builtins::assumptions::Assumption>)> {
    let at = ISSUE_8285_NEIGHBORS.find("page 97483").expect("the host page");
    let (before, from_host) = ISSUE_8285_NEIGHBORS.split_at(at);
    let src = format!("{}\n{before}{}", probe(ISSUE_8285_PROBE), host(from_host));
    issue_8186_outcomes(&src, codeunit)
}

fn same(s: &str) -> String {
    s.to_string()
}

/// The refusal of a call off the FactBox host's row that the probe does not measure.
const FACTBOX_MOVE_REFUSAL: &str = "a call on a List page with a CardPart FactBox, on the row its new line became";

/// Call sequences the probe does not measure are refused: SaveRecord without the Update(false) after it, SaveRecord then
/// Update() or Update(true), two SaveRecord calls, an error after SaveRecord and Update(false) (`asserterror` around the
/// SetValue), SaveRecord then CurrPage.Close(); and a change of Rec's filters or of the table, or a further CurrPage.Update or
/// SaveRecord, after SaveRecord or Update(true). Their page's OnValidate is not the probe's code, so each is refused at its first
/// SaveRecord or Update(true) (review of #8305, round 3); main answers the first and the last without the modify BC runs at the
/// SaveRecord (ListNewLineSaveRecordUpdate.al:445 `P.MOD(X/);T.M(X/);P.SR;`). `Update(true)` on the page without AutoSplitKey
/// keeps main's refusal.
#[test]
fn issue_8285_unmeasured_calls_are_refused() {
    let o = issue_8285_neighbor_outcomes("LT NS Neighbor Tests", same, same);
    for (name, why) in [
        ("SaveRecordWithoutUpdate", UNMEASURED_CODE),
        ("SaveRecordThenUpdate", UNMEASURED_CODE),
        ("SaveRecordThenUpdateTrue", UNMEASURED_CODE),
        ("SaveRecordTwice", UNMEASURED_CODE),
        ("ErrorAfterSaveRecord", UNMEASURED_CODE),
        ("SaveRecordThenClose", UNMEASURED_CODE),
        ("SaveRecordFilterUpdateFalse", UNMEASURED_CODE),
        ("SaveRecordInsertUpdateFalse", UNMEASURED_CODE),
        ("SaveRecordUpdateFalseSaveRecord", UNMEASURED_CODE),
        ("SaveRecordUpdateFalseUpdate", UNMEASURED_CODE),
        ("UpdateTrueUpdateFalse", UNMEASURED_CODE),
        ("UpdateTrueSaveRecord", UNMEASURED_CODE),
        ("UpdateTrueFilter", UNMEASURED_CODE),
        ("UpdateTrueInsert", UNMEASURED_CODE),
        ("UpdateTrueWithoutAutoSplitKey", MAIN_8186_REFUSAL),
    ] {
        let (_, r, marks) = o.iter().find(|(n, _, _)| n == name).expect("the test");
        assert!(matches!(r, Outcome::NotRun(w) if w.contains(why)), "{name}: {r:?}");
        if why == MAIN_8186_REFUSAL {
            assert!(marks.is_empty(), "{name}: {marks:?}");
        }
    }
}

/// On the FactBox host the landing ends only as pinned: the Close after the Value SetValue passes, with no assumption; Next as the first
/// move (with a row before and on an empty view), Previous or Close with no SetValue since, a GoToRecord, a second SetValue on
/// Name and a SetValue on the part are refused at that call (main refuses each of them at the landing). On the empty view after
/// Previous() answered false only Next() goes on (ListNewLineSaveRecordUpdate.al:683); a second Previous, a SetValue, New() and
/// Close() are refused. So are a SetValue of the value the row holds, and Close() in the handler of Page.RunModal (BC raises
/// there). Two rows before the line, and a read of the part before New(), keep main's refusal.
#[test]
fn issue_8285_factbox_host_leaves_the_row_only_as_measured() {
    let o = issue_8285_neighbor_outcomes("LT NS Host Neighbor Tests", same, same);
    let find = |name: &str| o.iter().find(|(n, _, _)| n == name).expect("the test");
    let (_, r, marks) = find("Host_RowBefore_Close");
    assert!(*r == Outcome::Pass && marks.is_empty(), "{r:?} {marks:?}");
    for name in [
        "Host_RowBefore_NextFirst",
        "Host_EmptyView_NextFirst",
        "Host_RowBefore_PreviousWithoutEdit",
        "Host_RowBefore_CloseWithoutEdit",
        "Host_RowBefore_GoToRecord",
        "Host_RowBefore_SecondNameSetValue",
        "Host_RowBefore_PartWrite",
        "Host_EmptyView_PreviousThenPrevious",
        "Host_EmptyView_PreviousThenSetValue",
        "Host_EmptyView_PreviousThenNew",
        "Host_EmptyView_PreviousThenClose",
        "Host_RowBefore_SameValueThenClose",
        "Host_RunModal_RowBefore_HandlerClose",
    ] {
        let (_, r, _) = find(name);
        assert!(
            matches!(r, Outcome::NotRun(w) if w.contains(FACTBOX_MOVE_REFUSAL)),
            "{name}: {r:?}"
        );
    }
    let (_, r, marks) = find("Host_EmptyView_PreviousThenNext");
    assert!(*r == Outcome::Pass && marks.is_empty(), "{r:?} {marks:?}");
    let (_, r, marks) = find("Host_PartReadBeforeNew");
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains(MAIN_8186_REFUSAL)) && marks.is_empty(),
        "{r:?} {marks:?}"
    );
    let o = issue_8285_neighbor_outcomes("LT NS Host Rows Tests", same, same);
    let (_, r, marks) = &o[0];
    assert!(
        matches!(r, Outcome::NotRun(w) if w.contains(MAIN_8186_REFUSAL)) && marks.is_empty(),
        "{r:?} {marks:?}"
    );
}

/// Facts of the FactBox host and its part the probe does not vary keep main's refusal, without the mark: a part trigger, a
/// ListPart, a property on the part page or on its field, a link on one key field, UpdatePropagation, a second part, an
/// OnAfterGetCurrRecord on the host, Editable = false on a host control, and CurrPage.Update(false) or Update(true) without the
/// SaveRecord on the host.
#[test]
fn issue_8285_unmeasured_factbox_shapes_keep_main_refusal() {
    let peek = "page 97475 \"LT NS Peek\"\n{\n    PageType = CardPart;";
    let link = "SubPageLink = \"Group Code\" = field(\"Group Code\"), \"Entry No.\" = field(\"Entry No.\");";
    let host_save = "CurrPage.SaveRecord();\n                        CurrPage.Update(false);";
    let host_agr = "    trigger OnAfterGetRecord()\n    begin\n        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Value + ')');\n    end;";
    let second_part = "        area(FactBoxes)\n        {";
    // What a variant is called, and how it changes the source.
    type Variant = (&'static str, Box<dyn Fn(&str) -> String>);
    let probe_variants: [Variant; 4] = [
        (
            "a part trigger",
            Box::new(move |s: &str| {
                s.replacen(
                    peek,
                    &format!("{peek}\n    trigger OnAfterGetCurrRecord()\n    begin\n    end;\n"),
                    1,
                )
            }),
        ),
        (
            "a ListPart",
            Box::new(move |s: &str| s.replacen(peek, &peek.replace("CardPart", "ListPart"), 1)),
        ),
        (
            "a part page property",
            Box::new(move |s: &str| s.replacen(peek, &format!("{peek}\n    Editable = false;"), 1)),
        ),
        (
            "a part field property",
            Box::new(|s: &str| {
                s.replacen(
                    "            field(Name; Rec.Name) { ApplicationArea = All; }\n            field(Value; Rec.Value) { ApplicationArea = All; }\n        }\n    }\n}",
                    "            field(Name; Rec.Name) { ApplicationArea = All; Editable = false; }\n            field(Value; Rec.Value) { ApplicationArea = All; }\n        }\n    }\n}",
                    1,
                )
            }),
        ),
    ];
    let host_variants: [Variant; 7] = [
        (
            "a link on one field",
            Box::new(move |s: &str| s.replacen(link, "SubPageLink = \"Entry No.\" = field(\"Entry No.\");", 1)),
        ),
        (
            "UpdatePropagation",
            Box::new(move |s: &str| s.replacen(link, &format!("{link}\n                UpdatePropagation = Both;"), 1)),
        ),
        (
            "a second part",
            Box::new(move |s: &str| {
                s.replacen(
                    second_part,
                    &format!(
                        "{second_part}\n            part(Peek2; \"LT NS Peek\") {{ ApplicationArea = All; {link} }}"
                    ),
                    1,
                )
            }),
        ),
        (
            "OnAfterGetCurrRecord on the host",
            Box::new(move |s: &str| {
                s.replacen(
                    host_agr,
                    &format!("{host_agr}\n\n    trigger OnAfterGetCurrRecord()\n    begin\n    end;"),
                    1,
                )
            }),
        ),
        (
            "Editable = false on a host control",
            Box::new(|s: &str| {
                s.replacen(
                    "field(Value; Rec.Value) { ApplicationArea = All; }",
                    "field(Value; Rec.Value) { ApplicationArea = All; Editable = false; }",
                    1,
                )
            }),
        ),
        (
            "Update(false) alone",
            Box::new(move |s: &str| s.replacen(host_save, "CurrPage.Update(false);", 1)),
        ),
        (
            "Update(true) alone",
            Box::new(move |s: &str| s.replacen(host_save, "CurrPage.Update(true);", 1)),
        ),
    ];
    // Main's outcome of each variant: its refusal at the landing; `Update(false)` alone fails, as on main, when the close inserts
    // the line a second time; a second part is refused at the open, as on main.
    let check = |what: &str, o: Vec<(String, Outcome, Vec<al_builtins::assumptions::Assumption>)>| {
        let (_, r, marks) = o
            .iter()
            .find(|(n, _, _)| n == "Host_RowBefore_Close")
            .expect("the test");
        let main = match what {
            "Update(false) alone" => matches!(r, Outcome::Fail(w) if w.contains("already exists")),
            // Two CardPart FactBoxes linked to the two-field key: main refuses the open (`refuse_code_inserted_parts_host`).
            "a second part" => matches!(r, Outcome::NotRun(w) if w.contains("two CardPart FactBoxes")),
            _ => matches!(r, Outcome::NotRun(w) if w.contains(MAIN_8186_REFUSAL)),
        };
        assert!(main && marks.is_empty(), "{what}: {r:?} {marks:?}");
    };
    for (what, change) in &probe_variants {
        let changed = change(ISSUE_8285_PROBE);
        assert_ne!(changed, ISSUE_8285_PROBE, "{what}");
        check(
            what,
            issue_8285_neighbor_outcomes("LT NS Host Neighbor Tests", |s| change(s), same),
        );
    }
    for (what, change) in &host_variants {
        let at = ISSUE_8285_NEIGHBORS.find("page 97483").expect("the host page");
        assert_ne!(
            change(&ISSUE_8285_NEIGHBORS[at..]),
            &ISSUE_8285_NEIGHBORS[at..],
            "{what}"
        );
        check(
            what,
            issue_8285_neighbor_outcomes("LT NS Host Neighbor Tests", same, |s| change(s)),
        );
    }
}

/// Code the probe does not run, on its own pages and table, is refused at the first SaveRecord or Update(true) (review of #8305,
/// round 3): main refused these, and BC has not been measured with them. Each variant changes one body of the probe; every test
/// on the changed page (on every page, for the table) is refused, and none passes.
#[test]
fn issue_8285_other_code_is_refused() {
    // What a variant is called, the text it replaces, its replacement, and the tests the change reaches, by name.
    type Variant = (&'static str, &'static str, &'static str, fn(&str) -> bool);
    let variants: [Variant; 5] = [
        (
            "OnModifyRecord exits false",
            "        Log.Add('P.MOD(' + Rec.Name + '/' + Rec.Value + ')');\n        exit(true);",
            "        Log.Add('P.MOD(' + Rec.Name + '/' + Rec.Value + ')');\n        exit(false);",
            |n| !n.contains("_Quiet_") && !n.contains("_PartHost_"),
        ),
        (
            "a page trigger assigns a field",
            "        Log.Add('P.AGR(' + Rec.Name + ')');",
            "        Log.Add('P.AGR(' + Rec.Name + ')');\n        Rec.Value := 'Z';",
            |n| !n.contains("_Quiet_") && !n.contains("_PartHost_"),
        ),
        (
            "a table trigger writes",
            "        Log.Add('T.M(' + Name + '/' + Value + ')');",
            "        Log.Add('T.M(' + Name + '/' + Value + ')');\n        Value := Value + '!';",
            |_| true,
        ),
        (
            "code before SaveRecord in the control's OnValidate",
            "                            CurrPage.SaveRecord();\n                            CurrPage.Update(false);",
            "                            Rec.Value := 'P';\n                            CurrPage.SaveRecord();\n                            CurrPage.Update(false);",
            |n| n.contains("_Quiet_"),
        ),
        (
            "code after Update(false) on the FactBox host",
            "                        CurrPage.SaveRecord();\n                        CurrPage.Update(false);",
            "                        CurrPage.SaveRecord();\n                        CurrPage.Update(false);\n                        Rec.Value := 'H';",
            |n| n.contains("_PartHost_"),
        ),
    ];
    for (what, from, to, reached) in variants {
        assert_eq!(ISSUE_8285_PROBE.matches(from).count(), 1, "{what}");
        let o = issue_8186_outcomes(&ISSUE_8285_PROBE.replacen(from, to, 1), "LT NS Tests");
        let mine: Vec<_> = o.iter().filter(|(n, _, _)| reached(n)).collect();
        assert!(!mine.is_empty(), "{what}");
        let bad: Vec<_> = mine
            .iter()
            .filter(|(_, r, _)| !matches!(r, Outcome::NotRun(w) if w.contains(UNMEASURED_CODE)))
            .collect();
        assert!(bad.is_empty(), "{what}: {bad:#?}");
    }
}

/// The reporter's repro pages, opened other than as list-new-line-save-record-repro measured them (Page.RunModal on one group
/// with one row before the new line, Issue8285.al:146-211): by `TestPage.OpenEdit`, and by Page.RunModal on an empty group. Each
/// is refused at its SaveRecord or Update(true), without the mark (review of #8305, round 4). Main refuses each too.
#[test]
fn issue_8285_repro_opened_otherwise_is_refused() {
    let tests = r#"
codeunit 50121 "Repro Unmeasured Open Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SaveUpdate_OpenEdit_RowBefore()
    var
        Lines: TestPage "Repro Lines Save Update";
    begin
        AddRowA();
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
    end;

    [Test]
    procedure UpdateTrue_OpenEdit_RowBefore()
    var
        Lines: TestPage "Repro Lines Update True";
    begin
        AddRowA();
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
    end;

    [Test]
    procedure SaveUpdate_OpenEdit_Empty()
    var
        ReproLine: Record "Repro Line";
        Lines: TestPage "Repro Lines Save Update";
    begin
        ReproLine.DeleteAll();
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
    end;

    [Test]
    [HandlerFunctions('SaveUpdateHandler')]
    procedure SaveUpdate_RunModal_EmptyGroup()
    var
        ReproLine: Record "Repro Line";
        Opener: Codeunit "Repro Line Opener";
    begin
        ReproLine.DeleteAll();
        Opener.Open(Page::"Repro Lines Save Update", 'G2');
    end;

    [Test]
    [HandlerFunctions('WithFactBoxHandler')]
    procedure WithFactBox_RunModal_EmptyGroup()
    var
        ReproLine: Record "Repro Line";
        Opener: Codeunit "Repro Line Opener";
    begin
        ReproLine.DeleteAll();
        Opener.Open(Page::"Repro Lines With FactBox", 'G1');
    end;

    local procedure AddRowA()
    var
        ReproLine: Record "Repro Line";
    begin
        ReproLine.DeleteAll();
        ReproLine.Init();
        ReproLine.Name := 'A';
        ReproLine.Insert(true);
    end;

    [ModalPageHandler]
    procedure SaveUpdateHandler(var Lines: TestPage "Repro Lines Save Update")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure WithFactBoxHandler(var Lines: TestPage "Repro Lines With FactBox")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;
}
"#;
    let src = format!("{}\n{tests}", include_str!("../fixtures/outside-fb/Issue8285.al"));
    let o = issue_8186_outcomes(&src, "Repro Unmeasured Open Tests");
    assert_eq!(o.len(), 5, "{o:#?}");
    let bad: Vec<_> = o
        .iter()
        .filter(|(_, r, marks)| !matches!(r, Outcome::NotRun(w) if w.contains(UNMEASURED_CODE)) || !marks.is_empty())
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

/// An OnAfterGetRecord on the FactBox host that changes Rec: not the probe's code, so the SaveRecord is refused.
#[test]
fn issue_8285_factbox_host_trigger_that_changes_rec_is_refused() {
    let agr = "        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Value + ')');";
    let o = issue_8285_neighbor_outcomes("LT NS Host Neighbor Tests", same, |s| {
        s.replacen(agr, &format!("{agr}\n        Rec.Value := 'Z';"), 1)
    });
    let (_, r, _) = o
        .iter()
        .find(|(n, _, _)| n == "Host_RowBefore_Close")
        .expect("the test");
    assert!(matches!(r, Outcome::NotRun(w) if w.contains(UNMEASURED_CODE)), "{r:?}");
}
