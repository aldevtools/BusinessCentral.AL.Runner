// The corpus probe ListNewLineWritesOtherTable.al of corpus pull requests 1527 and 1536 (al-runner issue 8735), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, and the writes of another table before
//   CurrPage.SaveRecord() or CurrPage.Update(true) in the control's OnValidate, are what is under test
// Fixtures used: own tables 97651, 97652, 97655, 97656 and 97667, pages 97653, 97658 and 97668, codeunits 97654, 97657, 97659
//   and 97669, test codeunit 97660
// Probe (al-runner issue 8735): a List page with AutoSplitKey stands on its new line; the SetValue on Name runs the table
//   field's OnValidate, which inserts the row (Insert(true), an AutoIncrement key field), then the control's OnValidate, which
//   saves the row (CurrPage.SaveRecord(), then CurrPage.Update(false)). Before the save another table is written: in the
//   repro of 8735 the table field inserts two rows of it after the insert; the second page varies one step at a time (the
//   control inserts a row of it before the save, the table field inserts a row of it and the control calls
//   CurrPage.Update(true), the table field or the control modifies a row of it; and the variations that decide the rule: the
//   table field inserts a row of it before the insert, deletes its rows, or the control also modifies the line through another
//   record before the save). The page is opened with Page.RunModal on a record filtered to one group, empty or with one row
//   before the new line. Readings: what the page shows after the Name and the Value SetValue, and the rows of both tables after
//   OK. A third page, after the test codeunit, holds the shapes of the review of the runner pull request 8806.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 97651 "LT NW Copy Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { AutoIncrement = true; }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            begin
                if "Entry No." = 0 then
                    Insert(true);
                CopyDefaults();
            end;
        }
        field(4; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    // Writes the default detail rows of the line into another table.
    local procedure CopyDefaults()
    var
        Detail: Record "LT NW Copy Detail";
        Index: Integer;
    begin
        for Index := 1 to 2 do begin
            Detail.Init();
            Detail."Entry No." := 0;
            Detail."Line Entry No." := "Entry No.";
            Detail.Text := Name + Format(Index);
            Detail.Insert();
        end;
    end;
}

table 97652 "LT NW Copy Detail"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "Line Entry No."; Integer) { }
        field(3; Text; Text[30]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

page 97653 "LT NW Copy Lines"
{
    PageType = List;
    SourceTable = "LT NW Copy Line";
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
    }
}

codeunit 97654 "LT NW Copy Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NW Copy Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NW Copy Lines", Line);
    end;
}

table 97655 "LT NW Var Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { AutoIncrement = true; }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            var
                Mode: Codeunit "LT NW Mode";
            begin
                if Mode.Get() = 4 then
                    AddDetail('Early');
                if "Entry No." = 0 then
                    Insert(true);
                case Mode.Get() of
                    2, 6:
                        AddDetail(Name);
                    3:
                        TouchDetails();
                    5:
                        begin
                            DeleteDetails();
                            AddDetail(Name);
                        end;
                end;
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    local procedure AddDetail(NewText: Text[30])
    var
        Detail: Record "LT NW Var Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
    end;

    local procedure TouchDetails()
    var
        Detail: Record "LT NW Var Detail";
    begin
        if Detail.FindSet() then
            repeat
                Detail.Text := 'Touched';
                Detail.Modify();
            until Detail.Next() = 0;
    end;

    local procedure DeleteDetails()
    var
        Detail: Record "LT NW Var Detail";
    begin
        Detail.DeleteAll();
    end;
}

table 97656 "LT NW Var Detail"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; Text; Text[30]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

codeunit 97657 "LT NW Mode"
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

page 97658 "LT NW Var Lines"
{
    PageType = List;
    SourceTable = "LT NW Var Line";
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
                    var
                        Mode: Codeunit "LT NW Mode";
                    begin
                        case Mode.Get() of
                            1:
                                begin
                                    AddDetail(Rec.Name);
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            2:
                                CurrPage.Update(true);
                            6:
                                begin
                                    StampOther(Rec);
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            7:
                                begin
                                    TouchDetails();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            else begin
                                CurrPage.SaveRecord();
                                CurrPage.Update(false);
                            end;
                        end;
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    local procedure AddDetail(NewText: Text[30])
    var
        Detail: Record "LT NW Var Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
    end;

    local procedure TouchDetails()
    var
        Detail: Record "LT NW Var Detail";
    begin
        if Detail.FindSet() then
            repeat
                Detail.Text := 'ByPage';
                Detail.Modify();
            until Detail.Next() = 0;
    end;

    local procedure StampOther(Line: Record "LT NW Var Line")
    var
        Other: Record "LT NW Var Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Note := 'Stamped';
        Other.Modify();
    end;
}

codeunit 97659 "LT NW Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NW Var Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NW Var Lines", Line);
    end;
}

codeunit 97660 "LT NW Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        ShownAfterName: Text;
        ShownAfterValue: Text;

    [Test]
    [HandlerFunctions('CopyHandler')]
    procedure NewLine_TableFieldWritesAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NW Copy Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/V', ShownAfterValue);
        Check('rows', 'A/V;', CopyRows());
        Check('details', 'A1;A2;', CopyDetails());
    end;

    [Test]
    [HandlerFunctions('CopyHandler')]
    procedure NewLine_TableFieldWritesAnotherTableBeforeSaveRecord_RowBefore_ShowsAndSavesTheRow()
    var
        Line: Record "LT NW Copy Line";
        Opener: Codeunit "LT NW Copy Opener";
    begin
        Initialize();
        Line."Group Code" := 'G1';
        Line.Name := 'B';
        Line.Insert();
        Commit();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/V', ShownAfterValue);
        Check('rows', 'B/;A/V;', CopyRows());
        Check('details', 'A1;A2;', CopyDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ControlWritesAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(1);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ControlWritesAnotherTableBeforeSaveRecord_RowBefore_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarRow('B');
        RunVar(1);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B//;A//V;', VarRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldWritesAnotherTableThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(2);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldModifiesAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarDetail('Old');
        RunVar(3);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'Touched;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldModifiesAnotherTableBeforeSaveRecord_RowBefore_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarRow('B');
        AddVarDetail('Old');
        RunVar(3);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B//;A//V;', VarRows());
        Check('details', 'Touched;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ControlModifiesAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarDetail('Old');
        RunVar(7);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'ByPage;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldWritesAnotherTableBeforeTheInsert_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(4);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'Early;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldDeletesAnotherTablesRowsBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarDetail('Old');
        RunVar(5);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_AnotherTableAndTheLineWrittenBeforeSaveRecord_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunVar(6);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('details', 'A;', VarDetails());
    end;

    [ModalPageHandler]
    procedure CopyHandler(var Lines: TestPage "LT NW Copy Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Value.Value();
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure VarHandler(var Lines: TestPage "LT NW Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        CopyLine: Record "LT NW Copy Line";
        CopyDetail: Record "LT NW Copy Detail";
        VarLine: Record "LT NW Var Line";
        VarDetail: Record "LT NW Var Detail";
        MixLine: Record "LT NW Mix Line";
        Mode: Codeunit "LT NW Mode";
    begin
        CopyLine.DeleteAll();
        CopyDetail.DeleteAll();
        VarLine.DeleteAll();
        VarDetail.DeleteAll();
        MixLine.DeleteAll();
        Mode.Set(0);
        Commit();
        ShownAfterName := '';
        ShownAfterValue := '';
    end;

    local procedure AddVarRow(Name: Text[30])
    var
        Line: Record "LT NW Var Line";
    begin
        Line."Group Code" := 'G1';
        Line.Name := Name;
        Line.Insert();
        Commit();
    end;

    local procedure AddVarDetail(NewText: Text[30])
    var
        Detail: Record "LT NW Var Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
        Commit();
    end;

    local procedure RunVar(M: Integer)
    var
        Mode: Codeunit "LT NW Mode";
        Opener: Codeunit "LT NW Var Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
        Mode.Set(0);
    end;

    local procedure CopyRows() Rows: Text
    var
        Line: Record "LT NW Copy Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Rows += Line.Name + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure CopyDetails() Details: Text
    var
        Line: Record "LT NW Copy Line";
        Detail: Record "LT NW Copy Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.SetRange(Name, 'A');
        Line.FindFirst();
        Detail.SetRange("Line Entry No.", Line."Entry No.");
        if Detail.FindSet() then
            repeat
                Details += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure VarRows() Rows: Text
    var
        Line: Record "LT NW Var Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Rows += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure VarDetails() Details: Text
    var
        Detail: Record "LT NW Var Detail";
    begin
        if Detail.FindSet() then
            repeat
                Details += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;

    // The third page (review of the runner pull request 8806): the control writes another table, then CurrPage.Update(true);
    // the table field modifies another table, then CurrPage.Update(true); the control writes another table and modifies the
    // line through another record before the save; the table field and the control both write another table before the save;
    // the control modifies the line, then writes another table; a FlowField of the page's table over the table the table field
    // writes.
    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_ControlWritesAnotherTableThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMix(8);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_TableFieldModifiesAnotherTableThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarDetail('Old');
        RunMix(9);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'Touched;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_ControlWritesAnotherTableAndTheLineBeforeSaveRecord_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMix(10);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_TableFieldAndControlWriteAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMix(11);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'A;ByPage;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_ControlWritesTheLineThenAnotherTableBeforeSaveRecord_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMix(12);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'A;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('MixCountHandler')]
    procedure NewLine_FlowFieldOverTheTableTheTableFieldWrites_ShowsTheCount()
    begin
        Initialize();
        RunMix(13);
        Check('shown after the Name SetValue', 'A//1', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V/1', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'A;', VarDetails());
    end;

    // The follow-ups of the review of the runner pull request 8806: the control modifies another table, then
    // CurrPage.Update(true); the table field and the control both write another table, then CurrPage.Update(true).
    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_ControlModifiesAnotherTableThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddVarDetail('Old');
        RunMix(14);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'ByPage;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('MixHandler')]
    procedure NewLine_TableFieldAndControlWriteAnotherTableThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMix(15);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', MixRows());
        Check('details', 'A;ByPage;', VarDetails());
    end;

    [ModalPageHandler]
    procedure MixCountHandler(var Lines: TestPage "LT NW Mix Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines."Detail Count".Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value() + '/' + Lines."Detail Count".Value();
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure MixHandler(var Lines: TestPage "LT NW Mix Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.OK().Invoke();
    end;

    local procedure RunMix(M: Integer)
    var
        Mode: Codeunit "LT NW Mode";
        Opener: Codeunit "LT NW Mix Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
        Mode.Set(0);
    end;

    local procedure MixRows() Rows: Text
    var
        Line: Record "LT NW Mix Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Rows += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;
}

table 97667 "LT NW Mix Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer) { AutoIncrement = true; }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            var
                Mode: Codeunit "LT NW Mode";
                Detail: Record "LT NW Var Detail";
            begin
                if "Entry No." = 0 then
                    Insert(true);
                case Mode.Get() of
                    9:
                        if Detail.FindSet() then
                            repeat
                                Detail.Text := 'Touched';
                                Detail.Modify();
                            until Detail.Next() = 0;
                    11, 13, 15:
                        begin
                            Detail.Init();
                            Detail.Text := Name;
                            Detail.Insert();
                        end;
                end;
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Value; Text[30]) { }
        field(6; "Detail Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("LT NW Var Detail");
        }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

page 97668 "LT NW Mix Lines"
{
    PageType = List;
    SourceTable = "LT NW Mix Line";
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
                    var
                        Mode: Codeunit "LT NW Mode";
                    begin
                        case Mode.Get() of
                            8:
                                begin
                                    AddDetail(Rec.Name);
                                    CurrPage.Update(true);
                                end;
                            9:
                                CurrPage.Update(true);
                            10:
                                begin
                                    AddDetail(Rec.Name);
                                    StampOther(Rec);
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            11:
                                begin
                                    AddDetail('ByPage');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            12:
                                begin
                                    StampOther(Rec);
                                    AddDetail(Rec.Name);
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            13:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            14:
                                begin
                                    TouchDetails();
                                    CurrPage.Update(true);
                                end;
                            15:
                                begin
                                    AddDetail('ByPage');
                                    CurrPage.Update(true);
                                end;
                        end;
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field("Detail Count"; Rec."Detail Count") { ApplicationArea = All; }
            }
        }
    }

    local procedure AddDetail(NewText: Text[30])
    var
        Detail: Record "LT NW Var Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
    end;

    local procedure TouchDetails()
    var
        Detail: Record "LT NW Var Detail";
    begin
        if Detail.FindSet() then
            repeat
                Detail.Text := 'ByPage';
                Detail.Modify();
            until Detail.Next() = 0;
    end;

    local procedure StampOther(Line: Record "LT NW Mix Line")
    var
        Other: Record "LT NW Mix Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Note := 'Stamped';
        Other.Modify();
    end;
}

codeunit 97669 "LT NW Mix Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NW Mix Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NW Mix Lines", Line);
    end;
}
