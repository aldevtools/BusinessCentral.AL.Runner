// Neighbors of ListNewLineExtraCode.al (al-runner issues 8712 and 8713): code just past what its cells measure, on a page like
// the probe's (no page triggers, AutoSplitKey, opened by Page.RunModal on one group). Each test holds Business Central's
// answer, measured on a local BC 29.1 container (3 runs of 3).
table 97400 "LT NN Line"
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
                Mode: Codeunit "LT NN Mode";
                Note: Record "LT NN Note";
            begin
                if "Entry No." = 0 then
                    Insert(true);
                if Mode.Get() = 3 then begin
                    Note.Init();
                    Note.Text := 'ByTable';
                    Note.Insert();
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
}

table 97401 "LT NN Note"
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

codeunit 97402 "LT NN Mode"
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

page 97403 "LT NN Lines"
{
    PageType = List;
    SourceTable = "LT NN Line";
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
                        Mode: Codeunit "LT NN Mode";
                    begin
                        case Mode.Get() of
                            1:
                                begin
                                    CurrPage.SaveRecord();
                                    StampRow(Rec."Group Code", Rec."Entry No.");
                                    Rec.Note := 'Late';
                                    CurrPage.Update(false);
                                end;
                            2:
                                begin
                                    CurrPage.SaveRecord();
                                    StampRow(Rec."Group Code", Rec."Entry No." - 1);
                                    CurrPage.Update(false);
                                end;
                            3:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            4:
                                begin
                                    AddNote();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            5:
                                begin
                                    CurrPage.SaveRecord();
                                    Rec.SetRange(Name, Rec.Name);
                                    CurrPage.Update(false);
                                end;
                            6:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.Update(true);
                                end;
                            7:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                    StampRow(Rec."Group Code", Rec."Entry No.");
                                end;
                            8:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                    Rec.Note := 'Late';
                                end;
                            9:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.Update(true);
                                    StampRow(Rec."Group Code", Rec."Entry No.");
                                end;
                            10:
                                begin
                                    CurrPage.SaveRecord();
                                    Rec.Note := 'X';
                                    Rec.Modify();
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

    local procedure StampRow(GroupCode: Code[10]; EntryNo: Integer)
    var
        Other: Record "LT NN Line";
    begin
        Other.Get(GroupCode, EntryNo);
        Other.Note := 'Stamped';
        Other.Modify();
    end;

    local procedure AddNote()
    var
        Note: Record "LT NN Note";
    begin
        Note.Init();
        Note.Text := 'ByPage';
        Note.Insert();
    end;
}

codeunit 97404 "LT NN Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NN Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NN Lines", Line);
    end;
}

// A page like the probe's but with OnAfterGetRecord: the effect gate covers the page without triggers only.
page 97405 "LT NN Lines Read"
{
    PageType = List;
    SourceTable = "LT NN Line";
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

    trigger OnAfterGetRecord()
    begin
        Reads += 1;
    end;

    var
        Reads: Integer;
}

codeunit 97406 "LT NN Read Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NN Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NN Lines Read", Line);
    end;
}

codeunit 97407 "LT NN Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        ShownAfterName: Text;
        ShownAfterValue: Text;

    // The row through another record and Rec both change between the save and Update(false): Update(false) reads the row.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_RowAndRecChangeAfterSaveRecord_UpdateFalseReadsTheRow()
    begin
        Initialize();
        RunMode(1);
        Check('shown after the Name SetValue', 'A/Stamped/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Stamped/V', ShownAfterValue);
        Check('rows', 'A/Stamped/V;', Rows());
    end;

    // Another row of the page's table changes between the save and Update(false).
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_AnotherRowChangesAfterSaveRecord_ShowsTheNewLine()
    begin
        Initialize();
        AddRow('B');
        RunMode(2);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B/Stamped/;A//V;', Rows());
    end;

    // The table field's OnValidate writes another table after its insert.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldWritesAnotherTable_ShowsAndSavesTheRow()
    var
        Note: Record "LT NN Note";
    begin
        Initialize();
        RunMode(3);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('notes', '1', Format(Note.Count()));
    end;

    // The control writes another table before the save.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_WriteAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    var
        Note: Record "LT NN Note";
    begin
        Initialize();
        RunMode(4);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('notes', '1', Format(Note.Count()));
    end;

    // The code sets a filter on Rec between the save and Update(false).
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_SetRangeAfterSaveRecord_ShowsTheRow()
    begin
        Initialize();
        RunMode(5);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
    end;

    // A field set, then Update(true), with a row before the new line.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_FieldSetThenUpdateTrue_RowBefore_ShowsAndSavesIt()
    begin
        Initialize();
        AddRow('B');
        RunMode(6);
        Check('shown after the Name SetValue', 'A/Resolved/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Resolved/V', ShownAfterValue);
        Check('rows', 'B//;A/Resolved/V;', Rows());
    end;

    // Code after the Update: SaveRecord, Update(false), then a modify of the row through another record.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ModifyThroughAnotherRecordAfterUpdateFalse_ShowsTheStoredRow()
    begin
        Initialize();
        RunMode(7);
        Check('shown and rows', 'A/Stamped/|A/Stamped/V|A/Stamped/V;', ShownAfterName + '|' + ShownAfterValue + '|' + Rows());
    end;

    // Code after the Update: SaveRecord, Update(false), then a field of Rec set.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_FieldSetAfterUpdateFalse_ReadsTheStoredRow()
    begin
        Initialize();
        RunMode(8);
        Check('shown and rows', 'A//|A//V|A//V;', ShownAfterName + '|' + ShownAfterValue + '|' + Rows());
    end;

    // Code after the Update: a field set, Update(true), then a modify of the row through another record.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ModifyThroughAnotherRecordAfterUpdateTrue_ShowsTheStoredRow()
    begin
        Initialize();
        RunMode(9);
        Check('shown and rows', 'A/Stamped/|A/Stamped/V|A/Stamped/V;', ShownAfterName + '|' + ShownAfterValue + '|' + Rows());
    end;

    // The row and Rec both changed between the save and Update(false), through Rec.Modify().
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_RecModifyAfterSaveRecord_ShowsTheModifiedRow()
    begin
        Initialize();
        RunMode(10);
        Check('shown and rows', 'A/X/|A/X/V|A/X/V;', ShownAfterName + '|' + ShownAfterValue + '|' + Rows());
    end;

    // The probe's field set before the save, on a page with OnAfterGetRecord.
    [Test]
    [HandlerFunctions('ReadHandler')]
    procedure NewLine_FieldSetBeforeSaveRecord_PageWithTrigger_ShowsAndSavesIt()
    var
        Opener: Codeunit "LT NN Read Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown after the Name SetValue', 'A/Resolved/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Resolved/V', ShownAfterValue);
        Check('rows', 'A/Resolved/V;', Rows());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NN Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure ReadHandler(var Lines: TestPage "LT NN Lines Read")
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
        Line: Record "LT NN Line";
        Note: Record "LT NN Note";
        Mode: Codeunit "LT NN Mode";
    begin
        Line.DeleteAll();
        Note.DeleteAll();
        Mode.Set(0);
        Commit();
        ShownAfterName := '';
        ShownAfterValue := '';
    end;

    local procedure AddRow(Name: Text[30])
    var
        Line: Record "LT NN Line";
    begin
        Line."Group Code" := 'G1';
        Line.Name := Name;
        Line.Insert();
        Commit();
    end;

    local procedure RunMode(M: Integer)
    var
        Mode: Codeunit "LT NN Mode";
        Opener: Codeunit "LT NN Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
        Mode.Set(0);
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NN Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
