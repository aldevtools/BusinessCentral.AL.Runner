// al-runner issue 8285: shapes next to the corpus probe (Issue8285Probe.al, whose table, log and part these tests use) that the
// probe does not measure. The new path must not take them: each keeps the runner's behavior from before the change, or is
// refused.

// A page without page triggers whose control's OnValidate makes a call sequence the probe does not measure (the log's flag picks
// it: see SetMode).
page 97480 "LT NS Calls"
{
    PageType = List;
    SourceTable = "LT NS Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        case Mode.Get() of
                            1:
                                CurrPage.SaveRecord();
                            2:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update();
                                end;
                            3:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(true);
                                end;
                            4:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            5:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                    Error('Boom after the update');
                                end;
                            6:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Close();
                                end;
                        end;
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    var
        Mode: Codeunit "LT NS Mode";
}

codeunit 97481 "LT NS Mode"
{
    SingleInstance = true;

    var
        Current: Integer;

    procedure Set(NewMode: Integer)
    begin
        Current := NewMode;
    end;

    procedure Get(): Integer
    begin
        exit(Current);
    end;
}

// Page 97472's five triggers without AutoSplitKey, with Update(true).
page 97482 "LT NS Lines No Split"
{
    PageType = List;
    SourceTable = "LT NS Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.Update(true);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        exit(true);
    end;

    trigger OnAfterGetRecord()
    begin
    end;

    trigger OnAfterGetCurrRecord()
    begin
    end;
}

// The FactBox host of page 97474, varied one fact at a time by the Rust tests (`Issue8285Neighbors` in outside_fb_repros.rs).
page 97483 "LT NS Host"
{
    PageType = List;
    SourceTable = "LT NS Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Name; Rec.Name)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        CurrPage.SaveRecord();
                        CurrPage.Update(false);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Peek; "LT NS Peek")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NS Log";
}

codeunit 97484 "LT NS Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "LT NS Mode";

    local procedure Initialize(RowsBefore: Integer; NewMode: Integer)
    var
        Line: Record "LT NS Line";
        i: Integer;
    begin
        Line.DeleteAll();
        for i := 1 to RowsBefore do begin
            Line.Init();
            Line."Entry No." := 0;
            Line.Name := 'A' + Format(i);
            Line.Insert();
        end;
        Commit();
        Mode.Set(NewMode);
    end;

    local procedure CheckRows(Expected: Text)
    var
        Line: Record "LT NS Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        if Result <> Expected then
            Error('Rows: expected %1, found %2.', Expected, Result);
    end;

    local procedure DriveCalls(NewMode: Integer)
    var
        Lines: TestPage "LT NS Calls";
    begin
        Initialize(1, NewMode);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('A1/,X/V,');
    end;

    [Test]
    procedure SaveRecordWithoutUpdate()
    begin
        DriveCalls(1);
    end;

    [Test]
    procedure SaveRecordThenUpdate()
    begin
        DriveCalls(2);
    end;

    [Test]
    procedure SaveRecordThenUpdateTrue()
    begin
        DriveCalls(3);
    end;

    [Test]
    procedure SaveRecordTwice()
    begin
        DriveCalls(4);
    end;

    [Test]
    procedure ErrorAfterSaveRecord()
    var
        Lines: TestPage "LT NS Calls";
    begin
        Initialize(1, 5);
        Lines.OpenEdit();
        Lines.New();
        asserterror Lines.Name.SetValue('X');
        Lines.Close();
        CheckRows('A1/,');
    end;

    [Test]
    procedure SaveRecordThenClose()
    var
        Lines: TestPage "LT NS Calls";
    begin
        Initialize(1, 6);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        CheckRows('A1/,X/,');
    end;

    [Test]
    procedure UpdateTrueWithoutAutoSplitKey()
    var
        Lines: TestPage "LT NS Lines No Split";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('A1/,X/V,');
    end;
}

// The FactBox host of page 97483 after the landing: the measured close, and the moves and closes the probe does not measure.
codeunit 97485 "LT NS Host Neighbor Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "LT NS Mode";

    local procedure Initialize(RowsBefore: Integer; NewMode: Integer)
    var
        Line: Record "LT NS Line";
        i: Integer;
    begin
        Line.DeleteAll();
        for i := 1 to RowsBefore do begin
            Line.Init();
            Line."Entry No." := 0;
            Line.Name := 'A' + Format(i);
            Line.Insert();
        end;
        Commit();
        Mode.Set(NewMode);
    end;

    local procedure CheckRows(Expected: Text)
    var
        Line: Record "LT NS Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        if Result <> Expected then
            Error('Rows: expected %1, found %2.', Expected, Result);
    end;

    local procedure DriveHost(RowsBefore: Integer)
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(RowsBefore, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
    end;

    [Test]
    procedure Host_RowBefore_Close()
    begin
        DriveHost(1);
        CheckRows('A1/,X/V,');
    end;

    [Test]
    procedure Host_RowBefore_NextFirst()
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Next();
        Lines.Close();
        CheckRows('A1/,X/V,');
    end;

    [Test]
    procedure Host_EmptyView_NextFirst()
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(0, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Next();
        Lines.Close();
        CheckRows('X/V,');
    end;

    [Test]
    procedure Host_RowBefore_PreviousWithoutEdit()
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Previous();
        Lines.Close();
        CheckRows('A1/,X/,');
    end;

    [Test]
    procedure Host_RowBefore_CloseWithoutEdit()
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Close();
        CheckRows('A1/,X/,');
    end;

    [Test]
    procedure Host_RowBefore_GoToRecord()
    var
        Lines: TestPage "LT NS Host";
        Line: Record "LT NS Line";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Line.FindFirst();
        Lines.GoToRecord(Line);
    end;

    [Test]
    procedure Host_RowBefore_SecondNameSetValue()
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Name.SetValue('Y');
    end;

    [Test]
    procedure Host_RowBefore_PartWrite()
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(1, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Peek.Value.SetValue('P');
    end;
}

// Two rows before the line on the FactBox host: not measured.
codeunit 97486 "LT NS Host Rows Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mode: Codeunit "LT NS Mode";

    local procedure Initialize(RowsBefore: Integer; NewMode: Integer)
    var
        Line: Record "LT NS Line";
        i: Integer;
    begin
        Line.DeleteAll();
        for i := 1 to RowsBefore do begin
            Line.Init();
            Line."Entry No." := 0;
            Line.Name := 'A' + Format(i);
            Line.Insert();
        end;
        Commit();
        Mode.Set(NewMode);
    end;

    local procedure CheckRows(Expected: Text)
    var
        Line: Record "LT NS Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        if Result <> Expected then
            Error('Rows: expected %1, found %2.', Expected, Result);
    end;

    local procedure DriveHost(RowsBefore: Integer)
    var
        Lines: TestPage "LT NS Host";
    begin
        Initialize(RowsBefore, 0);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
    end;

    [Test]
    procedure Host_TwoRowsBefore_Close()
    begin
        DriveHost(2);
        CheckRows('A1/,A2/,X/V,');
    end;
}
