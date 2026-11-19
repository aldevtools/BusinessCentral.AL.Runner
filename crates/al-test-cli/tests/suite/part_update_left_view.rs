//! A ListPart whose own procedure, called from a host control's OnValidate, copies filters onto its Rec and runs
//! `CurrPage.Update(false)` (al-runner issue 8328, the reporter's repro unchanged in `REPRO`). On a host opened for edit (the
//! reporter's `Page.Run` with a PageHandler, and `OpenEdit`) the part kept answering from the rows it held; which rows BC
//! shows there is not pinned, so every test call that reads or writes a row of the part is refused (`READERS`). The view-mode
//! open, a pending update whose view still starts with the held rows, rows the test stores after the update, the test's own
//! filter and a part with OnFindRecord keep main's outcome.

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

const READERS: &str = r#"
page 50106 "Part Probe Edit Lines"
{
    PageType = ListPart;
    SourceTable = "Part Probe Row";
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
    actions
    {
        area(Processing)
        {
            action(Ping)
            {
                trigger OnAction()
                begin
                    Message(Rec."No.");
                end;
            }
        }
    }

    procedure SetFilterFrom(var Source: Record "Part Probe Row")
    begin
        Rec.CopyFilters(Source);
        CurrPage.Update(false);
    end;

    procedure JustUpdate()
    begin
        CurrPage.Update(false);
    end;
}

page 50107 "Part Probe Edit Host"
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
                    trigger OnValidate()
                    begin
                        Rec.SetFilter("No.", NoFilter);
                        CurrPage.Lines.Page.SetFilterFrom(Rec);
                    end;
                }
                field(Cmd; Cmd)
                {
                    trigger OnValidate()
                    begin
                        CurrPage.Lines.Page.JustUpdate();
                    end;
                }
            }
            part(Lines; "Part Probe Edit Lines") { }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Push)
            {
                trigger OnAction()
                var
                    Source: Record "Part Probe Row";
                begin
                    Source.SetFilter("No.", 'B2');
                    CurrPage.Lines.Page.SetFilterFrom(Source);
                end;
            }
        }
    }

    var
        NoFilter: Text;
        Cmd: Text;
}

table 50108 "Part Probe Head"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50109 "Part Probe Buffer Lines"
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
            }
        }
    }

    trigger OnFindRecord(Which: Text): Boolean
    var
        Found: Boolean;
    begin
        TempBuf.Copy(Rec);
        Found := TempBuf.Find(Which);
        if Found then
            Rec := TempBuf;
        exit(Found);
    end;

    trigger OnNextRecord(Steps: Integer): Integer
    var
        Moved: Integer;
    begin
        TempBuf.Copy(Rec);
        Moved := TempBuf.Next(Steps);
        if Moved <> 0 then
            Rec := TempBuf;
        exit(Moved);
    end;

    procedure Load()
    var
        I: Integer;
    begin
        if TempBuf.IsEmpty() then
            for I := 1 to 3 do begin
                TempBuf."No." := 'T' + Format(I);
                TempBuf.Insert();
            end;
        CurrPage.Update(false);
    end;

    var
        TempBuf: Record "Part Probe Row" temporary;
}

page 50110 "Part Probe Buffer Host"
{
    PageType = Card;
    SourceTable = "Part Probe Head";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            part(Lines; "Part Probe Buffer Lines") { }
        }
    }

    trigger OnAfterGetCurrRecord()
    begin
        CurrPage.Lines.Page.Load();
    end;
}

page 50112 "Part Probe Card Host"
{
    PageType = Card;
    SourceTable = "Part Probe Head";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            part(Lines; "Part Probe Edit Lines") { }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Push)
            {
                trigger OnAction()
                var
                    Source: Record "Part Probe Row";
                begin
                    Source.SetFilter("No.", 'B2');
                    CurrPage.Lines.Page.SetFilterFrom(Source);
                end;
            }
        }
    }
}

codeunit 50113 "Part Probe Card Action"
{
    Subtype = Test;

    var
        Assert: Codeunit "Part Probe Assert";

    [Test]
    procedure CardHostActionReadsThePartAgain()
    var
        Host: TestPage "Part Probe Card Host";
        Head: Record "Part Probe Head";
        Row: Record "Part Probe Row";
        Seen: Text;
    begin
        Row.DeleteAll();
        Row."No." := 'B1';
        Row.Insert(false);
        Row."No." := 'B2';
        Row.Insert(false);
        Head.DeleteAll();
        Head."No." := 'H1';
        Head.Insert();
        Host.OpenEdit();
        Host.Push.Invoke();
        Host.Lines.First();
        Seen := 'first=' + Host.Lines."No.".Value();
        Assert.AreEqual('first=B2', Seen, 'host action');
    end;

}

codeunit 50111 "Part Probe Readers"
{
    Subtype = Test;

    var
        Assert: Codeunit "Part Probe Assert";

    [Test]
    procedure FieldRead()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Assert.AreEqual('B2', Host.Lines."No.".Value(), 'field read');
    end;

    [Test]
    procedure FieldAssertEquals()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines."No.".AssertEquals('B2');
    end;

    [Test]
    procedure FieldSetValue()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.Name.SetValue('X');
    end;

    [Test]
    [HandlerFunctions('PingHandler')]
    procedure PartAction()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.Ping.Invoke();
    end;

    [MessageHandler]
    procedure PingHandler(Msg: Text[1024])
    begin
    end;

    [Test]
    procedure PartNext()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.Next();
    end;

    [Test]
    procedure PartLast()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.Last();
    end;

    [Test]
    procedure PartPrevious()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.Previous();
    end;

    [Test]
    procedure PartGoToRecord()
    var
        Host: TestPage "Part Probe Edit Host";
        Row: Record "Part Probe Row";
    begin
        Filtered(Host);
        Row.Get('B2');
        Host.Lines.GoToRecord(Row);
    end;

    [Test]
    procedure PartFindFirstField()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.FindFirstField("No.", 'B2');
    end;

    [Test]
    procedure PartFindNextField()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.FindNextField("No.", 'B2');
    end;

    [Test]
    procedure PartNew()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.New();
    end;

    [Test]
    procedure PartGetFilter()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Assert.AreEqual('B2', Host.Lines.Filter.GetFilter("No."), 'GetFilter');
    end;

    [Test]
    procedure PartSetFilterThenRead()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        Filtered(Host);
        Host.Lines.Filter.SetFilter("No.", 'B3');
        Assert.AreEqual('B3', Host.Lines."No.".Value(), 'SetFilter then read');
    end;

    [Test]
    procedure ListPlusHostActionFilters()
    var
        Host: TestPage "Part Probe Edit Host";
    begin
        InsertRows();
        Host.OpenEdit();
        Host.Push.Invoke();
        Host.Lines.First();
    end;

    [Test]
    procedure TestInsertsAfterTheUpdate()
    var
        Host: TestPage "Part Probe Edit Host";
        Row: Record "Part Probe Row";
        Seen: Text;
    begin
        InsertRows();
        Host.OpenEdit();
        Host.Cmd.SetValue('UPD');
        Row."No." := 'A0';
        Row.Insert(false);
        Host.Lines.First();
        Seen := 'first=' + Host.Lines."No.".Value();
        Host.Lines.Next();
        Seen += ' next=' + Host.Lines."No.".Value();
        Assert.AreEqual('first=B1 next=B2', Seen, 'test inserts after the update');
    end;

    [Test]
    procedure TestInsertsBeforeTheUpdate()
    var
        Host: TestPage "Part Probe Edit Host";
        Row: Record "Part Probe Row";
    begin
        InsertRows();
        Host.OpenEdit();
        Row."No." := 'A0';
        Row.Insert(false);
        Host.Cmd.SetValue('UPD');
        Host.Lines.First();
    end;

    [Test]
    procedure BufferPartKeepsItsRows()
    var
        Host: TestPage "Part Probe Buffer Host";
        Head: Record "Part Probe Head";
        Row: Record "Part Probe Row";
        Seen: Text;
    begin
        Row.DeleteAll();
        Head.DeleteAll();
        Head."No." := 'H1';
        Head.Insert();
        Host.OpenEdit();
        Host.Lines.First();
        Seen := 'first=' + Host.Lines."No.".Value();
        Host.Lines.Next();
        Seen += ' next=' + Host.Lines."No.".Value();
        Host.Lines.Last();
        Seen += ' last=' + Host.Lines."No.".Value();
        Assert.AreEqual('first=T1 next=T2 last=T3', Seen, 'OnFindRecord part');
    end;

    local procedure Filtered(var Host: TestPage "Part Probe Edit Host")
    begin
        InsertRows();
        Host.OpenEdit();
        Host.NoFilter.SetValue('B2');
    end;

    local procedure InsertRows()
    var
        Row: Record "Part Probe Row";
    begin
        Row.DeleteAll();
        Row."No." := 'B1';
        Row.Name := 'First';
        Row.Insert(false);
        Row."No." := 'B2';
        Row.Name := 'Second';
        Row.Insert(false);
        Row."No." := 'B3';
        Row.Name := 'Third';
        Row.Insert(false);
    end;
}
"#;

fn outcomes(codeunit: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(&format!("{REPRO}\n{NEIGHBOURS}\n{READERS}"), codeunit)
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
                "{how}() on a part whose view no longer started with the rows it holds when its own CurrPage.Update was asked"
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

fn assert_reader_refused(name: &str, how: &str) {
    let list = outcomes("Part Probe Readers");
    assert_refused_move(outcome_of(&list, name), name, how);
}

#[test]
fn a_field_read_on_the_filtered_part_is_refused() {
    assert_reader_refused("FieldRead", "Value");
}

#[test]
fn assert_equals_on_the_filtered_part_is_refused() {
    assert_reader_refused("FieldAssertEquals", "AssertEquals");
}

#[test]
fn set_value_on_the_filtered_part_is_refused() {
    assert_reader_refused("FieldSetValue", "SetValue");
}

#[test]
fn an_action_of_the_filtered_part_is_refused() {
    assert_reader_refused("PartAction", "Invoke");
}

#[test]
fn the_other_moves_on_the_filtered_part_are_refused() {
    for (name, how) in [
        ("PartNext", "Next"),
        ("PartLast", "Last"),
        ("PartPrevious", "Previous"),
        ("PartGoToRecord", "GoToRecord"),
        ("PartFindFirstField", "FindFirstField"),
        ("PartFindNextField", "FindNextField"),
        ("PartNew", "New"),
    ] {
        assert_reader_refused(name, how);
    }
}

#[test]
fn get_filter_on_the_filtered_part_keeps_mains_outcome() {
    let list = outcomes("Part Probe Readers");
    assert_eq!(outcome_of(&list, "PartGetFilter"), Outcome::Pass);
}

#[test]
fn the_tests_own_set_filter_on_the_filtered_part_keeps_mains_outcome() {
    let list = outcomes("Part Probe Readers");
    assert_eq!(outcome_of(&list, "PartSetFilterThenRead"), Outcome::Pass);
}

#[test]
fn a_card_host_action_that_reads_the_part_again_keeps_mains_outcome() {
    let list = outcomes("Part Probe Card Action");
    assert_eq!(outcome_of(&list, "CardHostActionReadsThePartAgain"), Outcome::Pass);
}

#[test]
fn a_list_plus_host_action_that_filters_the_part_is_refused() {
    assert_reader_refused("ListPlusHostActionFilters", "First");
}

#[test]
fn rows_the_test_stores_after_the_update_keep_mains_outcome() {
    let list = outcomes("Part Probe Readers");
    assert_eq!(outcome_of(&list, "TestInsertsAfterTheUpdate"), Outcome::Pass);
}

/// The gate cannot tell who changed the rows before the part asked for its update: rows the test stored then refuse too.
#[test]
fn rows_the_test_stores_before_the_update_are_refused() {
    assert_reader_refused("TestInsertsBeforeTheUpdate", "First");
}

#[test]
fn a_part_with_on_find_record_keeps_mains_outcome() {
    let list = outcomes("Part Probe Readers");
    assert_eq!(outcome_of(&list, "BufferPartKeepsItsRows"), Outcome::Pass);
}
