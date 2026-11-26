// The corpus probe ListNewLineExtraCode.al of corpus pull request 1521 (al-runner issues 8712 and 8713), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, and the code around CurrPage.SaveRecord() and
//   CurrPage.Update in the control's OnValidate, are what is under test
// Fixtures used: own tables 97386, 97393, 97394 and 97399, pages 97388, 97391 and 97396, codeunits 97387, 97389, 97392, 97395 and
//   97397, test codeunit 97398
// Probe (al-runner issues 8712 and 8713): a List page with AutoSplitKey stands on its new line; the SetValue on Name runs the
//   table field's OnValidate, which inserts the row (Insert(true), an AutoIncrement key field), then the control's OnValidate,
//   whose code around CurrPage.SaveRecord() and CurrPage.Update differs per page: the repro of 8712 saves, modifies the row
//   through another record and calls Update(false); the repro of 8713 sets another field of Rec, saves and calls
//   Update(false); the third page varies one step at a time (a modify through another record before the save, a field the
//   table field sets, Update(true) after a field set, Update(true) after a modify through another record, a field set after
//   the save, a write of another table; the field the table field sets, the field set after the save and the write of
//   another table also with one row before the new line). The page is opened with Page.RunModal on a record filtered to one
//   group (empty, or with one row before the new line), or with TestPage.OpenEdit and a filter. Readings: what the page shows
//   after the Name and the Value SetValue, and the rows after OK or Close.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 97386 "LT NE Stamp Line"
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
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

codeunit 97387 "LT NE Stamp"
{
    procedure StampLine(Line: Record "LT NE Stamp Line")
    var
        Other: Record "LT NE Stamp Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Note := 'Stamped';
        Other.Modify();
    end;
}

page 97388 "LT NE Stamp Lines"
{
    PageType = List;
    SourceTable = "LT NE Stamp Line";
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
                        Stamp: Codeunit "LT NE Stamp";
                    begin
                        CurrPage.SaveRecord();
                        Stamp.StampLine(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 97389 "LT NE Stamp Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NE Stamp Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NE Stamp Lines", Line);
    end;
}

table 97399 "LT NE Resolve Line"
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
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

page 97391 "LT NE Resolve Lines"
{
    PageType = List;
    SourceTable = "LT NE Resolve Line";
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
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 97392 "LT NE Resolve Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NE Resolve Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NE Resolve Lines", Line);
    end;
}

table 97393 "LT NE Var Line"
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
                Mode: Codeunit "LT NE Mode";
            begin
                if "Entry No." = 0 then
                    Insert(true);
                if Mode.Get() = 2 then
                    Note := 'ByTable';
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

table 97394 "LT NE Var Note"
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

codeunit 97395 "LT NE Mode"
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

page 97396 "LT NE Var Lines"
{
    PageType = List;
    SourceTable = "LT NE Var Line";
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
                        Mode: Codeunit "LT NE Mode";
                    begin
                        case Mode.Get() of
                            1:
                                begin
                                    StampOther(Rec);
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            2:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            3:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.Update(true);
                                end;
                            4:
                                begin
                                    CurrPage.SaveRecord();
                                    StampOther(Rec);
                                    CurrPage.Update(true);
                                end;
                            5:
                                begin
                                    CurrPage.SaveRecord();
                                    Rec.Note := 'Late';
                                    CurrPage.Update(false);
                                end;
                            6:
                                begin
                                    CurrPage.SaveRecord();
                                    AddNote();
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

    local procedure StampOther(Line: Record "LT NE Var Line")
    var
        Other: Record "LT NE Var Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Note := 'Stamped';
        Other.Modify();
    end;

    local procedure AddNote()
    var
        VarNote: Record "LT NE Var Note";
    begin
        VarNote.Init();
        VarNote.Text := 'Added';
        VarNote.Insert();
    end;
}

codeunit 97397 "LT NE Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NE Var Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NE Var Lines", Line);
    end;
}

codeunit 97398 "LT NE Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        ShownAfterName: Text;
        ShownAfterValue: Text;

    [Test]
    [HandlerFunctions('StampHandler')]
    procedure NewLine_SaveRecordThenModifyThroughAnotherRecord_ShowsAndSavesIt()
    var
        Opener: Codeunit "LT NE Stamp Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/Stamped/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Stamped/V', ShownAfterValue);
        Check('rows', 'A/Stamped/V;', StampRows());
    end;

    [Test]
    [HandlerFunctions('StampHandler')]
    procedure NewLine_SaveRecordThenModifyThroughAnotherRecord_RowBefore_ShowsAndSavesIt()
    var
        Line: Record "LT NE Stamp Line";
        Opener: Codeunit "LT NE Stamp Opener";
    begin
        Initialize();
        Line."Group Code" := 'G1';
        Line.Name := 'B';
        Line.Insert();
        Commit();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/Stamped/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Stamped/V', ShownAfterValue);
        Check('rows', 'B//;A/Stamped/V;', StampRows());
    end;

    [Test]
    procedure NewLine_SaveRecordThenModifyThroughAnotherRecord_OpenEdit_ShowsAndSavesIt()
    var
        Lines: TestPage "LT NE Stamp Lines";
    begin
        Initialize();
        Lines.OpenEdit();
        Lines.Filter.SetFilter("Group Code", 'G1');
        Lines.New();
        Lines.Name.SetValue('A');
        Check('shown after the Name SetValue', 'A/Stamped/', Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value());
        Lines.Value.SetValue('V');
        Check('shown after the Value SetValue', 'A/Stamped/V', Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value());
        Lines.Close();
        Check('rows', 'A/Stamped/V;', StampRows());
    end;

    [ModalPageHandler]
    procedure StampHandler(var Lines: TestPage "LT NE Stamp Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('ResolveHandler')]
    procedure NewLine_FieldSetBeforeSaveRecord_ShowsAndSavesIt()
    var
        Opener: Codeunit "LT NE Resolve Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/Resolved/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Resolved/V', ShownAfterValue);
        Check('rows', 'A/Resolved/V;', ResolveRows());
    end;

    [Test]
    [HandlerFunctions('ResolveHandler')]
    procedure NewLine_FieldSetBeforeSaveRecord_RowBefore_ShowsAndSavesIt()
    var
        Line: Record "LT NE Resolve Line";
        Opener: Codeunit "LT NE Resolve Opener";
    begin
        Initialize();
        Line."Group Code" := 'G1';
        Line.Name := 'B';
        Line.Insert();
        Commit();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/Resolved/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Resolved/V', ShownAfterValue);
        Check('rows', 'B//;A/Resolved/V;', ResolveRows());
    end;

    [ModalPageHandler]
    procedure ResolveHandler(var Lines: TestPage "LT NE Resolve Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.OK().Invoke();
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ModifyThroughAnotherRecordBeforeSaveRecord_SaveWritesRecOverIt()
    begin
        Initialize();
        RunVar(1);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldSetsFieldBeforeSaveRecord_ShowsAndSavesIt()
    begin
        Initialize();
        RunVar(2);
        Check('shown after the Name SetValue', 'A/ByTable/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/ByTable/V', ShownAfterValue);
        Check('rows', 'A/ByTable/V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_FieldSetThenUpdateTrue_ShowsAndSavesIt()
    begin
        Initialize();
        RunVar(3);
        Check('shown after the Name SetValue', 'A/Resolved/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Resolved/V', ShownAfterValue);
        Check('rows', 'A/Resolved/V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordModifyThroughAnotherRecordThenUpdateTrue_SaveWritesRecOverIt()
    begin
        Initialize();
        RunVar(4);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_FieldSetAfterSaveRecord_UpdateFalseReadsTheStoredRow()
    begin
        Initialize();
        RunVar(5);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordThenWriteAnotherTable_ShowsAndSavesTheRow()
    var
        VarNote: Record "LT NE Var Note";
    begin
        Initialize();
        RunVar(6);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', VarRows());
        Check('notes', '1', Format(VarNote.Count()));
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TableFieldSetsFieldBeforeSaveRecord_RowBefore_ShowsAndSavesIt()
    begin
        Initialize();
        AddVarRow('B');
        RunVar(2);
        Check('shown after the Name SetValue', 'A/ByTable/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/ByTable/V', ShownAfterValue);
        Check('rows', 'B//;A/ByTable/V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_FieldSetAfterSaveRecord_RowBefore_UpdateFalseReadsTheStoredRow()
    begin
        Initialize();
        AddVarRow('B');
        RunVar(5);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B//;A//V;', VarRows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordThenWriteAnotherTable_RowBefore_ShowsAndSavesTheRow()
    var
        VarNote: Record "LT NE Var Note";
    begin
        Initialize();
        AddVarRow('B');
        RunVar(6);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B//;A//V;', VarRows());
        Check('notes', '1', Format(VarNote.Count()));
    end;

    [ModalPageHandler]
    procedure VarHandler(var Lines: TestPage "LT NE Var Lines")
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
        StampLine: Record "LT NE Stamp Line";
        ResolveLine: Record "LT NE Resolve Line";
        VarLine: Record "LT NE Var Line";
        VarNote: Record "LT NE Var Note";
        Mode: Codeunit "LT NE Mode";
    begin
        StampLine.DeleteAll();
        ResolveLine.DeleteAll();
        VarLine.DeleteAll();
        VarNote.DeleteAll();
        Mode.Set(0);
        Commit();
        ShownAfterName := '';
        ShownAfterValue := '';
    end;

    local procedure AddVarRow(Name: Text[30])
    var
        Line: Record "LT NE Var Line";
    begin
        Line."Group Code" := 'G1';
        Line.Name := Name;
        Line.Insert();
        Commit();
    end;

    local procedure RunVar(M: Integer)
    var
        Mode: Codeunit "LT NE Mode";
        Opener: Codeunit "LT NE Var Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
        Mode.Set(0);
    end;

    local procedure StampRows() Rows: Text
    var
        Line: Record "LT NE Stamp Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Rows += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure ResolveRows() Rows: Text
    var
        Line: Record "LT NE Resolve Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Rows += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure VarRows() Rows: Text
    var
        Line: Record "LT NE Var Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Rows += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
