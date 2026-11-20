//! al-runner issue 8537: a ListPart on a temporary table that the host's OnAfterGetCurrRecord clears and refills, each step ending
//! with the part's own `CurrPage.Update(false)`, then the test's own `Filter.SetFilter` on the part and a move or a read. The
//! corpus probe (tests/al-language/pages/parts/PartRefillTestFilter.al: corpus pull request 1474, merged, not pinned yet, plus
//! the two-field key cell of corpus pull request 1475, open; green 10/10 on a local Business Central 29.1) and the shapes probe
//! (tests/al-language/pages/parts/PartRefillShapes.al, corpus pull request 1487, open; green 15/15 there), with a stand-in for
//! the corpus's Assert, and the reporter's repro (green on Business Central 29.1).
//! The calls the probe did not make after the test's filter, and the pages and tables outside the facts of
//! `part_refill_rule.rs`, keep the refusal of `part_update_left_view.rs`. When the probes are pinned, delete their tests.

use al_builtins::assumptions::Assumption;
use al_testrun::Outcome;

/// Each test of `codeunit` in `src` (with the stand-in Assert), its outcome and the assumptions it ran through.
fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
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

const PROBE: &str = include_str!("../fixtures/outside-fb/Issue8537Probe.al");
const REPRO: &str = include_str!("../fixtures/outside-fb/Issue8537.al");
/// The shapes probe (corpus PR 1487, open: tests/al-language/pages/parts/PartRefillShapes.al, green 15/15 on a local Business
/// Central 29.1): each cell varies the measured Worksheet cell by one fact of a partner page, and one cell has them all.
const SHAPES: &str = include_str!("../fixtures/outside-fb/Issue8537Shapes.al");

/// The shapes probe with its Base Application "Load Buffer" (which the suite does not compile) swapped for an own table with
/// the fields the probe uses and the same primary key (Period Type, Period Start).
fn shapes() -> String {
    const LOAD_BUFFER: &str = r#"
table 90460 "LT PS Load Buffer"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Period Type"; Option) { OptionMembers = Day,Week,Month,Quarter,Year,"Accounting Period"; }
        field(2; "Period Start"; Date) { }
        field(3; "Allocated Qty."; Decimal) { }
    }

    keys
    {
        key(PK; "Period Type", "Period Start") { Clustered = true; }
    }
}
"#;
    assert!(SHAPES.contains("\"Load Buffer\""));
    format!(
        "{}\n{LOAD_BUFFER}",
        SHAPES.replace("\"Load Buffer\"", "\"LT PS Load Buffer\"")
    )
}

fn outcome_of<'a>(
    list: &'a [(String, Outcome, Vec<Assumption>)],
    name: &str,
) -> &'a (String, Outcome, Vec<Assumption>) {
    list.iter()
        .find(|(n, ..)| n == name)
        .unwrap_or_else(|| panic!("no test {name} in {list:#?}"))
}

fn assert_flagged_pass(list: &[(String, Outcome, Vec<Assumption>)], name: &str) {
    let (_, o, marks) = outcome_of(list, name);
    assert!(
        *o == Outcome::Pass && marks.as_slice() == [Assumption::PartLeftViewTestFilter],
        "{name}: {o:?} {marks:?}"
    );
}

/// The probe's eight refill tests that meet the refusal pass through the assumption. Two never meet it and pass unmarked, as on
/// main: the host's Next reads the part's rows again before the test's filter, and without a new filter the rows differ from
/// the held ones.
#[test]
fn issue_8537_the_tests_filter_reads_the_refilled_rows() {
    let list = outcomes(PROBE, "LT PR Tests");
    for name in [
        "PartRefill_WorksheetActionThenFilterFirst_FindsRefilledRow",
        "PartRefill_WorksheetActionThenFilterRead_ShowsRefilledRow",
        "PartRefill_WorksheetActionThenFilterFirstNextPrevious_StayOnRefilledRow",
        "PartRefill_WorksheetActionThenFilterLast_FindsRefilledRow",
        "PartRefill_ListActionThenFilterFirst_FindsRefilledRow",
        "PartRefill_BackgroundTaskActionThenFilterFirst_FindsRefilledRow",
        "PartRefill_TriggerPartActionThenFilterFirst_FindsRefilledRow",
        "PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow",
    ] {
        assert_flagged_pass(&list, name);
    }
    for name in [
        "PartRefill_WorksheetHostNextThenFilterFirst_FindsRowOfNextResource",
        "PartRefill_WorksheetActionThenFirstWithoutFilter_ResetDropsTheFilter",
    ] {
        let (_, o, marks) = outcome_of(&list, name);
        assert!(*o == Outcome::Pass && marks.is_empty(), "{name}: {o:?} {marks:?}");
    }
}

#[test]
fn issue_8537_reporters_repro_passes_flagged() {
    let list = outcomes(REPRO, "Repro Part Refill Tests");
    assert_flagged_pass(&list, "RefreshRefillsPartThenFirstFindsFilteredRow");
}

/// Calls the probe did not make after the test's filter, on the probe's objects. Each one sets up the measured shape (the
/// action refilled the part, then the test's own filter) and then makes one call no cell made.
const NEIGHBORS: &str = r#"
codeunit 97150 "LT PR Neighbors"
{
    Subtype = Test;

    local procedure Refilled(var Host: TestPage "LT PR Load Host")
    var
        Op: Record "LT PR Op";
        Booking: Record "LT PR Booking";
    begin
        Op.DeleteAll();
        Booking.DeleteAll();
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        Add(1, TheDay(), 10);
        Add(2, TheDay() + 1, 5);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()));
    end;

    local procedure Add(EntryNo: Integer; Day: Date; Qty: Decimal)
    var
        Booking: Record "LT PR Booking";
    begin
        Booking."Entry No." := EntryNo;
        Booking."Resource No." := 'R1';
        Booking.Day := Day;
        Booking.Qty := Qty;
        Booking.Insert();
    end;

    local procedure TheDay(): Date
    begin
        exit(DMY2Date(15, 6, 2026));
    end;

    [Test]
    procedure NextRightAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.Next();
    end;

    [Test]
    procedure PreviousRightAfterFirst()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.First();
        Host.Load.Previous();
    end;

    [Test]
    procedure NextAfterLast()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.Last();
        Host.Load.Next();
    end;

    [Test]
    procedure GoToKeyAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.GoToKey(TheDay());
    end;

    [Test]
    procedure AssertEqualsAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load."Allocated Qty.".AssertEquals(30);
    end;

    [Test]
    procedure AscendingAfterTheFilter()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.Filter.Ascending(false);
        Host.Load.First();
    end;

    [Test]
    procedure RangeFilter()
    var
        Op: Record "LT PR Op";
        Booking: Record "LT PR Booking";
        Host: TestPage "LT PR Load Host";
    begin
        Op.DeleteAll();
        Booking.DeleteAll();
        Op."No." := 'OP1';
        Op."Resource No." := 'R1';
        Op.Insert();
        Add(1, TheDay(), 10);
        Add(2, TheDay() + 1, 5);
        Host.OpenView();
        Host.GoToRecord(Op);
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()) + '..' + Format(TheDay() + 1));
        Host.Load.First();
        Add(3, TheDay(), 20);
        Host.Refresh.Invoke();
        Host.Load.Filter.SetFilter("Period Start", Format(TheDay()) + '..' + Format(TheDay() + 1));
        Host.Load.First();
    end;

    [Test]
    procedure HostReadBetweenFilterAndFirst()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host."Resource No.".Value();
        Host.Load.First();
    end;

    [Test]
    procedure OtherPageBetweenFilterAndFirst()
    var
        Host: TestPage "LT PR Load Host";
        Other: TestPage "LT PR Load List Host";
    begin
        Refilled(Host);
        Other.OpenView();
        Other.Close();
        Host.Load.First();
    end;

    [Test]
    procedure HostReadBetweenFirstAndARead()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Load.First();
        Host."No.".Value();
        Host.Load."Allocated Qty.".AsDecimal();
    end;

    [Test]
    procedure FirstAfterReopen()
    var
        Host: TestPage "LT PR Load Host";
    begin
        Refilled(Host);
        Host.Close();
        Host.OpenView();
        Host.Load.First();
    end;
}
"#;

fn assert_refused(list: &[(String, Outcome, Vec<Assumption>)], name: &str, how: &str) {
    match &outcome_of(list, name).1 {
        Outcome::NotRun(why) => assert!(
            why.contains(&format!(
                "{how}() on a part whose view no longer started with the rows it holds when its own CurrPage.Update was asked"
            )),
            "{name}: {why}"
        ),
        other => panic!("{name}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn issue_8537_calls_no_cell_made_after_the_tests_filter_stay_refused() {
    let list = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT PR Neighbors");
    for (name, how) in [
        ("NextRightAfterTheFilter", "Next"),
        ("PreviousRightAfterFirst", "Previous"),
        ("NextAfterLast", "Next"),
        ("GoToKeyAfterTheFilter", "GoToKey"),
        ("AssertEqualsAfterTheFilter", "AssertEquals"),
        ("AscendingAfterTheFilter", "First"),
        // A test call on the host or on another page between the filter and the part's calls ends the steps.
        ("HostReadBetweenFilterAndFirst", "First"),
        ("OtherPageBetweenFilterAndFirst", "First"),
        ("HostReadBetweenFirstAndARead", "AsDecimal"),
    ] {
        assert_refused(&list, name, how);
    }
}

/// Nothing waits, so main's answer stays, unmarked: the reopened page's part reads its rows fresh, and a two-day filter's
/// refill reads the rows the part held (its second Update finds them again, `note_part_update_left_view`).
#[test]
fn issue_8537_shapes_where_nothing_waits_keep_mains_answer() {
    let list = outcomes(&format!("{PROBE}\n{NEIGHBORS}"), "LT PR Neighbors");
    for name in ["FirstAfterReopen", "RangeFilter"] {
        let (_, o, marks) = outcome_of(&list, name);
        assert!(*o == Outcome::Pass && marks.is_empty(), "{name}: {o:?} {marks:?}");
    }
}

/// A part trigger set no cell had (OnAfterGetCurrRecord and OnAfterGetRecord) keeps the refusal at the probe's First.
#[test]
fn issue_8537_a_part_with_other_triggers_stays_refused() {
    let src = PROBE.replacen("trigger OnOpenPage()", "trigger OnAfterGetCurrRecord()", 1);
    assert_ne!(src, PROBE);
    let list = outcomes(&src, "LT PR Tests");
    assert_refused(
        &list,
        "PartRefill_TriggerPartActionThenFilterFirst_FindsRefilledRow",
        "First",
    );
}

/// A part over a stored table (no cell had one) keeps the refusal at the probe's First.
#[test]
fn issue_8537_a_part_over_a_stored_table_stays_refused() {
    let src = PROBE
        .replace("TableType = Temporary;", "")
        .replace("SourceTableTemporary = true;", "");
    assert!(!src.contains("TableType = Temporary") && !src.contains("SourceTableTemporary"));
    let list = outcomes(&src, "LT PR Tests");
    assert_refused(
        &list,
        "PartRefill_WorksheetActionThenFilterFirst_FindsRefilledRow",
        "First",
    );
}

/// The two-field key part whose refill inserts a Day and a Week row for each day (no cell had two rows in the filtered view):
/// the filter on Period Start leaves two rows, and First keeps the refusal.
#[test]
fn issue_8537_two_rows_left_by_the_filter_stay_refused() {
    let from = "            Rec.\"Period Type\" := Rec.\"Period Type\"::Day;\n            Evaluate(Rec.\"Period Start\", DayKey, 9);\n            Evaluate(Rec.\"Allocated Qty.\", Results.Get(DayKey), 9);\n            Rec.Insert();";
    assert!(PROBE.contains(from));
    let to =
        format!("{from}\n            Rec.\"Period Type\" := Rec.\"Period Type\"::Week;\n            Rec.Insert();");
    let list = outcomes(&PROBE.replacen(from, &to, 1), "LT PR Tests");
    assert_refused(
        &list,
        "PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow",
        "First",
    );
}

/// Every cell of the shapes probe passes through the assumption: the other parts and their code, the part control's Visible
/// variable, the host's code and row triggers, its view filters, a page background task and its error trigger, a Load Buffer
/// source, a group above the repeater, OnDrillDown and StyleExpr, enum parameters, and the rows inserted by a codeunit that gets
/// Rec by var.
#[test]
fn issue_8537_the_shapes_of_a_partner_page_pass_through_the_rule() {
    let list = outcomes(&format!("{PROBE}\n{}", shapes()), "LT PS Tests");
    assert_eq!(list.len(), 15, "{list:#?}");
    for (name, ..) in &list {
        assert_flagged_pass(&list, name);
    }
}

/// The facts the rule names, each changed on the probe's Worksheet cell (`LT PR Load Host` with `LT PR Load Part`, whose objects
/// come first in the probe, so each anchor's first occurrence is theirs), keep the refusal at the probe's First: a part with an
/// action, a part field with OnValidate on a field of Rec, the part control with UpdatePropagation, a host with OnNewRecord, a
/// ListPlus host; and the two-field key part with a third key field, or with its fields swapped so the filter is on the first.
#[test]
fn issue_8537_facts_outside_the_rule_stay_refused() {
    let worksheet = "PartRefill_WorksheetActionThenFilterFirst_FindsRefilledRow";
    let typed = "PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow";
    for (what, from, to, test) in [
        (
            "part action",
            "\n    var\n        ResourceNo: Code[20];",
            "\n    actions { area(Processing) { action(Poke) { ApplicationArea = All; trigger OnAction() begin end; } } }\n    var\n        ResourceNo: Code[20];",
            worksheet,
        ),
        (
            "OnValidate on a field of Rec",
            "field(\"Period Start\"; Rec.\"Period Start\") { ApplicationArea = All; }",
            "field(\"Period Start\"; Rec.\"Period Start\") { ApplicationArea = All; trigger OnValidate() begin end; }",
            worksheet,
        ),
        (
            "part control property",
            "part(Load; \"LT PR Load Part\") { ApplicationArea = All; }",
            "part(Load; \"LT PR Load Part\") { ApplicationArea = All; UpdatePropagation = Both; }",
            worksheet,
        ),
        (
            "host OnNewRecord",
            "    trigger OnAfterGetCurrRecord()\n    var\n        LoadCalc",
            "    trigger OnNewRecord(BelowxRec: Boolean)\n    begin\n    end;\n\n    trigger OnAfterGetCurrRecord()\n    var\n        LoadCalc",
            worksheet,
        ),
        (
            "ListPlus host",
            "PageType = Worksheet;",
            "PageType = ListPlus;",
            worksheet,
        ),
        (
            "three key fields",
            "field(3; \"Allocated Qty.\"; Decimal) { }\n    }\n\n    keys\n    {\n        key(PK; \"Period Type\", \"Period Start\") { Clustered = true; }",
            "field(3; \"Allocated Qty.\"; Decimal) { }\n        field(4; \"Period End\"; Date) { }\n    }\n\n    keys\n    {\n        key(PK; \"Period Type\", \"Period Start\", \"Period End\") { Clustered = true; }",
            typed,
        ),
        (
            "the filter on the first of two key fields",
            "key(PK; \"Period Type\", \"Period Start\") { Clustered = true; }",
            "key(PK; \"Period Start\", \"Period Type\") { Clustered = true; }",
            typed,
        ),
    ] {
        assert!(PROBE.contains(from), "{what}");
        let list = outcomes(&PROBE.replacen(from, to, 1), "LT PR Tests");
        assert_refused(&list, test, "First");
    }
}
