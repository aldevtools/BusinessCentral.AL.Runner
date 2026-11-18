//! A ListPart whose own procedure, called from a host control's OnValidate, copies filters onto its Rec and runs
//! `CurrPage.Update(false)` (al-runner issue 8328, the reporter's repro unchanged in `REPRO`). On a host opened for edit (the
//! reporter's `Page.Run` with a PageHandler, and `OpenEdit`) the part held its unfiltered rows and answered from them; which rows
//! BC shows there is not pinned, so the part's move is refused. The view-mode open and a pending update whose view still starts
//! with the held rows keep main's outcome.

use al_testrun::Outcome;

const REPRO: &str = r#"
table 50100 "Part Probe Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[50]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50101 "Part Probe Lines"
{
    PageType = ListPart;
    SourceTable = "Part Probe Row";
    Editable = false;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }

    procedure SetFilterFrom(var Source: Record "Part Probe Row")
    begin
        Rec.CopyFilters(Source);
        CurrPage.Update(false);
    end;
}

page 50102 "Part Probe Host"
{
    PageType = ListPlus;
    SourceTable = "Part Probe Row";
    SourceTableTemporary = true;
    InsertAllowed = false;
    DeleteAllowed = false;
    layout
    {
        area(Content)
        {
            group(Options)
            {
                field(NoFilter; NoFilter)
                {
                    Caption = 'No. Filter';
                    trigger OnValidate()
                    begin
                        Rec.SetFilter("No.", NoFilter);
                        CurrPage.Lines.Page.SetFilterFrom(Rec);
                    end;
                }
            }
            part(Lines; "Part Probe Lines") { }
        }
    }

    var
        NoFilter: Text;
}

codeunit 50103 "Part Probe Assert"
{
    procedure AreEqual(Expected: Variant; Actual: Variant; Msg: Text)
    begin
        if Format(Expected) <> Format(Actual) then
            Error('Assert.AreEqual failed. Expected:<%1>. Actual:<%2>. %3.', Expected, Actual, Msg);
    end;
}

codeunit 50104 "Part Probe Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit "Part Probe Assert";

    [Test]
    procedure OpenViewPartShowsFilteredRow()
    var
        Host: TestPage "Part Probe Host";
    begin
        InsertRows();
        Host.OpenView();
        Host.NoFilter.SetValue('B2');
        Assert.AreEqual('filter=B2 first=B2 next=none', WalkPart(Host), 'OpenView');
        Host.Close();
    end;

    [Test]
    [HandlerFunctions('HostHandler')]
    procedure PageRunPartShowsFilteredRow()
    begin
        InsertRows();
        Page.Run(Page::"Part Probe Host");
    end;

    [PageHandler]
    procedure HostHandler(var Host: TestPage "Part Probe Host")
    begin
        Host.NoFilter.SetValue('B2');
        Assert.AreEqual('filter=B2 first=B2 next=none', WalkPart(Host), 'Page.Run + PageHandler');
    end;

    local procedure WalkPart(var Host: TestPage "Part Probe Host") Seen: Text
    begin
        Seen := 'filter=' + Host.Lines.Filter.GetFilter("No.");
        Host.Lines.First();
        Seen += ' first=' + Host.Lines."No.".Value();
        if Host.Lines.Next() then
            Seen += ' next=' + Host.Lines."No.".Value()
        else
            Seen += ' next=none';
    end;

    local procedure InsertRows()
    begin
        InsertRow('B1', 'First');
        InsertRow('B2', 'Second');
        InsertRow('B3', 'Third');
    end;

    local procedure InsertRow(No: Code[20]; Name: Text[50])
    var
        Row: Record "Part Probe Row";
    begin
        if Row.Get(No) then
            exit;
        Row."No." := No;
        Row.Name := Name;
        Row.Insert(false);
    end;
}
"#;

const NEIGHBOURS: &str = r#"
codeunit 50105 "Part Probe Neighbours"
{
    Subtype = Test;

    var
        Assert: Codeunit "Part Probe Assert";

    [Test]
    procedure OpenEditPartShowsFilteredRow()
    var
        Host: TestPage "Part Probe Host";
    begin
        InsertRows();
        Host.OpenEdit();
        Host.NoFilter.SetValue('B2');
        Assert.AreEqual('filter=B2 first=B2 next=none', WalkPart(Host), 'OpenEdit');
        Host.Close();
    end;

    [Test]
    procedure OpenEditUpdateWithSameRowsKeepsHeldRows()
    var
        Host: TestPage "Part Probe Host";
    begin
        InsertRows();
        Host.OpenEdit();
        Host.NoFilter.SetValue('');
        Assert.AreEqual('filter= first=B1 next=B2', WalkPart(Host), 'OpenEdit, no filter');
        Host.Close();
    end;

    [Test]
    procedure OpenEditGoToKeyOnFilteredPart()
    var
        Host: TestPage "Part Probe Host";
    begin
        InsertRows();
        Host.OpenEdit();
        Host.NoFilter.SetValue('B2');
        Assert.AreEqual(true, Host.Lines.GoToKey('B2'), 'GoToKey');
        Host.Close();
    end;

    local procedure WalkPart(var Host: TestPage "Part Probe Host") Seen: Text
    begin
        Seen := 'filter=' + Host.Lines.Filter.GetFilter("No.");
        Host.Lines.First();
        Seen += ' first=' + Host.Lines."No.".Value();
        if Host.Lines.Next() then
            Seen += ' next=' + Host.Lines."No.".Value()
        else
            Seen += ' next=none';
    end;

    local procedure InsertRows()
    var
        Row: Record "Part Probe Row";
    begin
        Row.DeleteAll();
        Row."No." := 'B1';
        Row.Insert(false);
        Row."No." := 'B2';
        Row.Insert(false);
        Row."No." := 'B3';
        Row.Insert(false);
    end;
}
"#;

fn outcomes(codeunit: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(&format!("{REPRO}\n{NEIGHBOURS}"), codeunit)
}

fn outcome_of(list: &[(String, Outcome)], name: &str) -> Outcome {
    list.iter()
        .find(|(n, _)| n == name)
        .map(|(_, o)| o.clone())
        .unwrap_or_else(|| panic!("no test {name} in {list:?}"))
}

fn assert_refused(o: Outcome, name: &str) {
    assert_refused_move(o, name, "First");
}

fn assert_refused_move(o: Outcome, name: &str, how: &str) {
    match o {
        Outcome::NotRun(why) => assert!(
            why.contains(&format!(
                "{how}() on a part whose own CurrPage.Update waits for its refresh"
            )) && why.contains("not pinned"),
            "{name}: {why}"
        ),
        other => panic!("{name}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn page_run_with_a_page_handler_refuses_the_move_on_the_filtered_part() {
    let list = outcomes("Part Probe Tests");
    assert_refused(
        outcome_of(&list, "PageRunPartShowsFilteredRow"),
        "PageRunPartShowsFilteredRow",
    );
}

#[test]
fn open_edit_refuses_the_move_on_the_filtered_part() {
    let list = outcomes("Part Probe Neighbours");
    assert_refused(
        outcome_of(&list, "OpenEditPartShowsFilteredRow"),
        "OpenEditPartShowsFilteredRow",
    );
}

#[test]
fn open_view_keeps_mains_outcome() {
    let list = outcomes("Part Probe Tests");
    assert_eq!(outcome_of(&list, "OpenViewPartShowsFilteredRow"), Outcome::Pass);
}

#[test]
fn a_pending_update_whose_view_keeps_the_held_rows_keeps_mains_outcome() {
    let list = outcomes("Part Probe Neighbours");
    assert_eq!(
        outcome_of(&list, "OpenEditUpdateWithSameRowsKeepsHeldRows"),
        Outcome::Pass
    );
}

#[test]
fn go_to_key_on_the_filtered_part_is_refused() {
    let list = outcomes("Part Probe Neighbours");
    assert_refused_move(
        outcome_of(&list, "OpenEditGoToKeyOnFilteredPart"),
        "OpenEditGoToKeyOnFilteredPart",
        "GoToKey",
    );
}
