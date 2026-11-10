//! A Worksheet on the new-row line of a batch without lines whose header control switches to another batch without lines (the
//! Movement Worksheet: the name's OnValidate calls CurrPage.SaveRecord, sets the name and location filters in filter group 2,
//! calls Rec.Find('-') and CurrPage.Update(false); al-runner issue 6924). After the OnValidate the page shows the new-row line
//! of the new batch: OnNewRecord and OnAfterGetCurrRecord run twice with the new filters' values, nothing is read or stored,
//! and an action then sees the new batch in Rec. Measured on BC 27.5, 28.5 and 29.0 by corpus pull request 1282
//! (tests/al-language/pages/journal-batch-switch/EmptyToEmptyBatchSwitch.al: MovementWs_EmptyToEmpty_ThenAction, _NoAction,
//! _SecondTest, MovementWs_SameName_ThenAction, JournalWs_EmptyToEmpty_ThenAction); the objects below are that file's.

const SRC: &str = r#"
table 70432 "LT EE Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Template; Code[10]) { }
        field(2; Name; Code[10]) { }
        field(3; Location; Code[10]) { }
        field(4; "Line No."; Integer) { }
    }
    keys
    {
        key(PK; Template, Name, Location, "Line No.") { Clustered = true; }
    }

    procedure SetUpNewLine(NewTemplate: Code[10]; NewName: Code[10]; NewLocation: Code[10])
    begin
        Template := NewTemplate;
        Name := NewName;
        Location := NewLocation;
        "Line No." := 10000;
    end;

    // The shape of Warehouse Worksheet Line "WBR Calculate ..." in a partner extension: the name record is read with the keys of Rec.
    procedure Describe(var Line: Record "LT EE Line"): Text
    var
        WorksheetName: Record "LT EE Name";
    begin
        Line.TestField(Template);
        WorksheetName.Get(Line.Template, Line.Name, Line.Location);
        exit(Line.Template + '/' + Line.Name + '/' + Line.Location);
    end;
}

table 70433 "LT EE Name"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Template; Code[10]) { }
        field(2; Name; Code[10]) { }
        field(3; Location; Code[10]) { }
    }
    keys
    {
        key(PK; Template, Name, Location) { Clustered = true; }
    }
}

page 70435 "LT EE Movement Worksheet"
{
    PageType = Worksheet;
    SourceTable = "LT EE Line";
    DelayedInsert = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(CurrentName; CurrentName)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('V:start');
                        CurrPage.SaveRecord();
                        SetName();
                        CurrPage.Update(false);
                        Log.Add('V:done');
                    end;
                }
                field(CurrentLocation; CurrentLocation)
                {
                    ApplicationArea = All;
                }
            }
            repeater(Lines)
            {
                field(Template; Rec.Template) { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Location; Rec.Location) { ApplicationArea = All; }
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadRec)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('A:' + Rec.Name + '/' + Rec.Location);
                    Log.SetResult(Rec.Describe(Rec));
                end;
            }
        }
    }

    trigger OnOpenPage()
    var
        WorksheetName: Record "LT EE Name";
    begin
        CurrentTemplate := 'T1';
        Rec.FilterGroup := 2;
        Rec.SetRange(Template, CurrentTemplate);
        Rec.FilterGroup := 0;
        WorksheetName.SetRange(Template, CurrentTemplate);
        WorksheetName.FindFirst();
        CurrentName := WorksheetName.Name;
        CurrentLocation := WorksheetName.Location;
        SetName();
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('R:' + Rec.Name + '/' + Rec.Location);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C:' + Rec.Name + '/' + Rec.Location);
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('N:' + Rec.Name + '/' + Rec.Location);
        Rec.SetUpNewLine(CurrentTemplate, CurrentName, CurrentLocation);
    end;

    local procedure SetName()
    begin
        Rec.FilterGroup := 2;
        Rec.SetRange(Name, CurrentName);
        Rec.SetRange(Location, CurrentLocation);
        Rec.FilterGroup := 0;
        if Rec.Find('-') then;
    end;

    var
        Log: Codeunit "LT EE Log";
        CurrentTemplate: Code[10];
        CurrentName: Code[10];
        CurrentLocation: Code[10];
}

page 70436 "LT EE Journal Worksheet"
{
    PageType = Worksheet;
    SourceTable = "LT EE Line";
    AutoSplitKey = true;
    DelayedInsert = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(CurrentName; CurrentName)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('V:start');
                        CurrPage.SaveRecord();
                        SetName();
                        CurrPage.Update(false);
                        Log.Add('V:done');
                    end;
                }
                field(CurrentLocation; CurrentLocation)
                {
                    ApplicationArea = All;
                }
            }
            repeater(Lines)
            {
                field(Template; Rec.Template) { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
                field(Location; Rec.Location) { ApplicationArea = All; }
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadRec)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('A:' + Rec.Template + '/' + Rec.Name + '/' + Rec.Location);
                end;
            }
        }
    }

    trigger OnOpenPage()
    var
        WorksheetName: Record "LT EE Name";
    begin
        CurrentTemplate := 'T1';
        Rec.FilterGroup := 2;
        Rec.SetRange(Template, CurrentTemplate);
        Rec.FilterGroup := 0;
        WorksheetName.SetRange(Template, CurrentTemplate);
        WorksheetName.FindFirst();
        CurrentName := WorksheetName.Name;
        CurrentLocation := WorksheetName.Location;
        SetName();
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('R:' + Rec.Name + '/' + Rec.Location);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('C:' + Rec.Name + '/' + Rec.Location);
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('N:' + Rec.Template + '/' + Rec.Name + '/' + Rec.Location);
    end;

    local procedure SetName()
    begin
        Rec.FilterGroup := 2;
        Rec.SetRange(Name, CurrentName);
        Rec.SetRange(Location, CurrentLocation);
        Rec.FilterGroup := 0;
        if Rec.Find('-') then;
    end;

    var
        Log: Codeunit "LT EE Log";
        CurrentTemplate: Code[10];
        CurrentName: Code[10];
        CurrentLocation: Code[10];
}

codeunit 70434 "LT EE Log"
{
    SingleInstance = true;

    var
        Entries: Text;
        Result: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take() Taken: Text
    begin
        Taken := Entries;
        Entries := '';
    end;

    procedure SetResult(NewResult: Text)
    begin
        Result := NewResult;
    end;

    procedure TakeResult() Taken: Text
    begin
        Taken := Result;
        Result := '';
    end;
}

codeunit 70437 C
{
    var
        Log: Codeunit "LT EE Log";

    local procedure Seed()
    var
        WorksheetName: Record "LT EE Name";
        Line: Record "LT EE Line";
    begin
        Line.DeleteAll();
        WorksheetName.DeleteAll();
        AddName('A', 'L1');
        AddName('B', 'L2');
        AddName('C', 'L3');
        Log.Take();
        Log.TakeResult();
    end;

    local procedure AddName(Name: Code[10]; Location: Code[10])
    var
        WorksheetName: Record "LT EE Name";
    begin
        WorksheetName.Template := 'T1';
        WorksheetName.Name := Name;
        WorksheetName.Location := Location;
        WorksheetName.Insert();
    end;

    local procedure MovementSwitch(NewName: Text; NewLocation: Text; Invoke: Boolean) s: Text
    var
        W: TestPage "LT EE Movement Worksheet";
        Line: Record "LT EE Line";
    begin
        Seed();
        W.OpenEdit();
        s := 'open=' + W.Name.Value() + '/' + W.Location.Value() + '/' + W."Line No.".Value() + ':' + Log.Take();
        s += '|header=' + W.CurrentName.Value() + '/' + W.CurrentLocation.Value();
        W.CurrentLocation.SetValue(NewLocation);
        W.CurrentName.SetValue(NewName);
        s += '|validate=' + Log.Take();
        s += '|current=' + W.Template.Value() + '/' + W.Name.Value() + '/' + W.Location.Value() + '/' + W."Line No.".Value();
        s += '|read=' + Log.Take();
        if Invoke then begin
            W.ReadRec.Invoke();
            s += '|action=' + Log.Take();
            s += '|result=' + Log.TakeResult();
        end;
        s += '|first=' + Format(W.First()) + ':' + Log.Take();
        W.Close();
        s += '|close=' + Log.Take();
        s += '|stored=' + Format(Line.Count());
    end;

    procedure MovementThenAction(): Text begin exit(MovementSwitch('B', 'L2', true)); end;
    procedure MovementNoAction(): Text begin exit(MovementSwitch('B', 'L2', false)); end;
    procedure MovementSecond(): Text begin MovementSwitch('B', 'L2', true); exit(MovementSwitch('C', 'L3', true)); end;
    procedure MovementSameName(): Text begin exit(MovementSwitch('A', 'L1', true)); end;

    procedure Journal() s: Text
    var
        W: TestPage "LT EE Journal Worksheet";
        Line: Record "LT EE Line";
    begin
        Seed();
        W.OpenEdit();
        s := 'open=' + W.Name.Value() + '/' + W.Location.Value() + '/' + W."Line No.".Value() + ':' + Log.Take();
        W.CurrentLocation.SetValue('L2');
        W.CurrentName.SetValue('B');
        s += '|validate=' + Log.Take();
        s += '|current=' + W.Template.Value() + '/' + W.Name.Value() + '/' + W.Location.Value() + '/' + W."Line No.".Value();
        s += '|read=' + Log.Take();
        W.ReadRec.Invoke();
        s += '|action=' + Log.Take();
        W.Close();
        s += '|close=' + Log.Take();
        s += '|stored=' + Format(Line.Count());
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

fn movement(name: &str, with_action: bool) -> String {
    let action = if with_action {
        format!("|action=A:{name};N:{name};C:{name};N:{name};C:{name};|result=T1/{name}")
    } else {
        String::new()
    };
    format!(
        "open=A/L1/10000:N:A/L1;C:A/L1;N:A/L1;C:A/L1;|header=A/L1|validate=V:start;V:done;N:{name};C:{name};N:{name};C:{name};\
         |current=T1/{name}/10000|read={action}|first=No:|close=|stored=0"
    )
}

/// EmptyToEmptyBatchSwitch.al MovementWs_EmptyToEmpty_ThenAction: the action sees the new worksheet in Rec.
#[test]
fn a_movement_worksheet_switched_between_empty_worksheets_shows_the_new_one_to_an_action() {
    assert_eq!(run("MovementThenAction"), movement("B/L2", true));
}

/// EmptyToEmptyBatchSwitch.al MovementWs_EmptyToEmpty_NoAction.
#[test]
fn a_movement_worksheet_switched_between_empty_worksheets_runs_the_new_row_triggers_twice() {
    assert_eq!(run("MovementNoAction"), movement("B/L2", false));
}

/// EmptyToEmptyBatchSwitch.al MovementWs_EmptyToEmpty_SecondTest: a second page in the same codeunit, to the third worksheet.
#[test]
fn a_second_switch_in_the_same_codeunit_reads_the_same() {
    assert_eq!(run("MovementSecond"), movement("C/L3", true));
}

/// EmptyToEmptyBatchSwitch.al MovementWs_SameName_ThenAction: the name the page already shows.
#[test]
fn a_switch_to_the_name_the_page_shows_runs_the_new_row_triggers_twice_too() {
    assert_eq!(run("MovementSameName"), movement("A/L1", true));
}

/// EmptyToEmptyBatchSwitch.al JournalWs_EmptyToEmpty_ThenAction: with AutoSplitKey and an OnNewRecord that only logs, the key of
/// the new row comes from the filters.
#[test]
fn a_journal_worksheet_switched_between_empty_batches_takes_the_key_from_the_filters() {
    assert_eq!(
        run("Journal"),
        "open=A/L1/0:N:T1/A/L1;C:A/L1;N:T1/A/L1;C:A/L1;|validate=V:start;V:done;N:T1/B/L2;C:B/L2;N:T1/B/L2;C:B/L2;\
         |current=T1/B/L2/0|read=|action=A:T1/B/L2;N:T1/B/L2;C:B/L2;N:T1/B/L2;C:B/L2;|close=|stored=0"
    );
}
