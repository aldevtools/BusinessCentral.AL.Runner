//! GoToKey and GoToRecord of a row an action inserted while a Worksheet list is open find it whatever the number of rows, before
//! every row, after every row and in the middle, with and without `CurrPage.Update()`, with both row triggers and with neither
//! (corpus pages/long-list-open/Y7344ListFirstWindow.al, al-runner issue 7344). The source is the corpus file without its
//! observer; `RunBoth`/`RunNone` take the row count, the action (1 InsA, 2 InsANoUpdate, 3 InsZ, 4 InsM) and GoToRecord or GoToKey.

use al_builtins::value::Value;

const SRC: &str = r#"
table 99700 "LT FW Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    procedure AddRow(NewNo: Code[20])
    var
        Line: Record "LT FW Line";
    begin
        Line."No." := NewNo;
        Line.Description := 'Inserted ' + NewNo;
        Line.Insert();
    end;
}

page 99701 "LT FW Both"
{
    PageType = Worksheet;
    SourceTable = "LT FW Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(InsA)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('A');
                    CurrPage.Update();
                end;
            }
            action(InsANoUpdate)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('A');
                end;
            }
            action(InsZ)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('Z');
                    CurrPage.Update();
                end;
            }
            action(InsM)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('R1010A');
                    CurrPage.Update();
                end;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
    end;

    trigger OnAfterGetCurrRecord()
    begin
    end;
}

page 99702 "LT FW None"
{
    PageType = Worksheet;
    SourceTable = "LT FW Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(InsA)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('A');
                    CurrPage.Update();
                end;
            }
            action(InsANoUpdate)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('A');
                end;
            }
            action(InsZ)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('Z');
                    CurrPage.Update();
                end;
            }
            action(InsM)
            {
                ApplicationArea = All;
                trigger OnAction()
                begin
                    Rec.AddRow('R1010A');
                    CurrPage.Update();
                end;
            }
        }
    }
}

codeunit 99703 "LT FW Tests"
{
    local procedure Seed(Count: Integer)
    var
        Line: Record "LT FW Line";
        i: Integer;
    begin
        Line.DeleteAll();
        for i := 1 to Count do begin
            Line."No." := 'R' + Format(1000 + i);
            Line.Description := 'Existing';
            Line.Insert();
        end;
    end;

    [TryFunction]
    local procedure TryBothKey(var P: TestPage "LT FW Both"; RowKey: Code[20]; var Found: Boolean)
    begin
        Found := P.GoToKey(RowKey);
    end;

    [TryFunction]
    local procedure TryBothRecord(var P: TestPage "LT FW Both"; RowKey: Code[20]; var Found: Boolean)
    var
        Line: Record "LT FW Line";
    begin
        Line.Get(RowKey);
        Found := P.GoToRecord(Line);
    end;

    procedure RunBoth(Count: Integer; Action: Integer; ByRecord: Boolean): Text
    var
        P: TestPage "LT FW Both";
        RowKey: Code[20];
        Found: Boolean;
        Ok: Boolean;
        Shown: Text;
    begin
        Seed(Count);
        P.OpenEdit();
        case Action of
            1:
                begin
                    P.InsA.Invoke();
                    RowKey := 'A';
                end;
            2:
                begin
                    P.InsANoUpdate.Invoke();
                    RowKey := 'A';
                end;
            3:
                begin
                    P.InsZ.Invoke();
                    RowKey := 'Z';
                end;
            4:
                begin
                    P.InsM.Invoke();
                    RowKey := 'R1010A';
                end;
        end;
        ClearLastError();
        if ByRecord then
            Ok := TryBothRecord(P, RowKey, Found)
        else
            Ok := TryBothKey(P, RowKey, Found);
        if Ok then
            Shown := 'found=' + Format(Found)
        else
            Shown := 'error:' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Shown + ' now=' + P."No.".Value());
    end;

    [TryFunction]
    local procedure TryNoneKey(var P: TestPage "LT FW None"; RowKey: Code[20]; var Found: Boolean)
    begin
        Found := P.GoToKey(RowKey);
    end;

    [TryFunction]
    local procedure TryNoneRecord(var P: TestPage "LT FW None"; RowKey: Code[20]; var Found: Boolean)
    var
        Line: Record "LT FW Line";
    begin
        Line.Get(RowKey);
        Found := P.GoToRecord(Line);
    end;

    procedure RunNone(Count: Integer; Action: Integer; ByRecord: Boolean): Text
    var
        P: TestPage "LT FW None";
        RowKey: Code[20];
        Found: Boolean;
        Ok: Boolean;
        Shown: Text;
    begin
        Seed(Count);
        P.OpenEdit();
        case Action of
            1:
                begin
                    P.InsA.Invoke();
                    RowKey := 'A';
                end;
            2:
                begin
                    P.InsANoUpdate.Invoke();
                    RowKey := 'A';
                end;
            3:
                begin
                    P.InsZ.Invoke();
                    RowKey := 'Z';
                end;
            4:
                begin
                    P.InsM.Invoke();
                    RowKey := 'R1010A';
                end;
        end;
        ClearLastError();
        if ByRecord then
            Ok := TryNoneRecord(P, RowKey, Found)
        else
            Ok := TryNoneKey(P, RowKey, Found);
        if Ok then
            Shown := 'found=' + Format(Found)
        else
            Shown := 'error:' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Shown + ' now=' + P."No.".Value());
    end;
}
"#;

fn run(routine: &str, rows: i32, action: i32, by_record: bool) -> String {
    let mut b = al_testkit::Built::from_source(SRC);
    let (o, r) = b.routine_in("LT FW Tests", routine);
    match b.call(
        o,
        r,
        vec![Value::Integer(rows), Value::Integer(action), Value::Boolean(by_record)],
    ) {
        Ok((v, _)) => al_testkit::text(Ok(v)),
        Err(e) => format!("signal: {e:?}"),
    }
}

fn expect(routine: &str, counts: &[i32], action: i32, row: &str) {
    let want = format!("found=Yes now={row}");
    let mut wrong = Vec::new();
    for &n in counts {
        for by_record in [false, true] {
            let got = run(routine, n, action, by_record);
            if got != want {
                wrong.push(format!("{n} rows, by record {by_record}: {got}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{routine} action {action}: {wrong:#?}");
}

#[test]
fn both_triggers_insert_before_every_row_with_update() {
    // Y7344ListFirstWindow.al Y7344_Both_InsertBeforeAll_Update.
    expect("RunBoth", &[1, 19, 20, 21, 60], 1, "A");
}

#[test]
fn both_triggers_insert_before_every_row_without_update() {
    // Y7344_Both_InsertBeforeAll_NoUpdate.
    expect("RunBoth", &[1, 19, 20, 21, 60], 2, "A");
}

#[test]
fn both_triggers_insert_after_every_row() {
    // Y7344_Both_InsertAfterAll_Update.
    expect("RunBoth", &[19, 20, 60], 3, "Z");
}

#[test]
fn both_triggers_insert_in_the_middle() {
    // Y7344_Both_InsertMiddle_Update.
    expect("RunBoth", &[19, 20, 60], 4, "R1010A");
}

#[test]
fn no_row_triggers_insert_before_every_row() {
    // Y7344_None_InsertBeforeAll_Update.
    expect("RunNone", &[19, 20, 60], 1, "A");
}
