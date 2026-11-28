// Neighbors of ListNewLineWritesOtherTable.al (al-runner issue 8735): writes of another table before the save just past what its
// cells measure, on a page like the probe's (no page triggers, AutoSplitKey). Each test holds Business Central's answer,
// measured on a local BC 29.1 container (3 runs of 3).
table 97661 "LT NX Line"
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
                Mode: Codeunit "LT NX Mode";
                Extra: Record "LT NX Line";
            begin
                // Copilot review of the train for runner pull request 8878: an unchanged Modify of another table, then a Commit,
                // before the insert.
                if Mode.Get() = 13 then begin
                    TouchDetails('Old');
                    Commit();
                end;
                if Mode.Get() = 18 then
                    Commit();
                if "Entry No." = 0 then
                    Insert(true);
                case Mode.Get() of
                    1, 2:
                        AddDetail(Name);
                    4:
                        begin
                            AddDetail(Name);
                            Extra."Group Code" := "Group Code";
                            Extra.Name := 'Extra';
                            Extra.Insert();
                        end;
                    5, 7, 8:
                        AddDetail(Name);
                    12:
                        TouchDetails('Old');
                    16:
                        AddTempDetail();
                    17:
                        Commit();
                    6:
                        begin
                            AddDetail(Name);
                            Note := 'Twice';
                            Modify();
                        end;
                end;
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Value; Text[30]) { }
        // A FlowField over the table the table field writes (review of the runner pull request 8806).
        field(6; "Detail Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("LT NX Detail");
        }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    local procedure AddDetail(NewText: Text[30])
    var
        Detail: Record "LT NX Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
    end;

    local procedure AddTempDetail()
    var
        TempDetail: Record "LT NX Detail" temporary;
    begin
        TempDetail."Entry No." := 1;
        TempDetail.Text := 'Temp';
        TempDetail.Insert();
    end;

    local procedure TouchDetails(NewText: Text[30])
    var
        Detail: Record "LT NX Detail";
    begin
        if Detail.FindSet() then
            repeat
                Detail.Text := NewText;
                Detail.Modify();
            until Detail.Next() = 0;
    end;
}

table 97662 "LT NX Detail"
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

codeunit 97663 "LT NX Mode"
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

page 97664 "LT NX Lines"
{
    PageType = List;
    SourceTable = "LT NX Line";
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
                        Mode: Codeunit "LT NX Mode";
                    begin
                        case Mode.Get() of
                            2:
                                CurrPage.Update(true);
                            5:
                                begin
                                    StampRow(Rec."Group Code", Rec."Entry No." - 1, 'Stamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            8:
                                begin
                                    AddDetail('ByPage');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            9:
                                begin
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    AddDetail('ByPage');
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Restamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            10:
                                begin
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Restamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            11:
                                begin
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    TouchDetails();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            12:
                                begin
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            16, 17, 18:
                                begin
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            13:
                                begin
                                    AddDetail('ByPage');
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            14:
                                begin
                                    AddTempDetails(2);
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            15:
                                begin
                                    AddTempDetails(1);
                                    StampRow(Rec."Group Code", Rec."Entry No.", 'Stamped');
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
                field("Detail Count"; Rec."Detail Count") { ApplicationArea = All; }
            }
        }
    }

    local procedure AddDetail(NewText: Text[30])
    var
        Detail: Record "LT NX Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
    end;

    // Inserts rows into a temporary record of the detail table (no SQLTimestamp field): no database write.
    local procedure AddTempDetails(Count: Integer)
    var
        TempDetail: Record "LT NX Detail" temporary;
        Index: Integer;
    begin
        for Index := 1 to Count do begin
            TempDetail.Init();
            TempDetail."Entry No." := Index;
            TempDetail.Text := 'Temp';
            TempDetail.Insert();
        end;
    end;

    local procedure StampRow(GroupCode: Code[10]; EntryNo: Integer; NewNote: Text[30])
    var
        Other: Record "LT NX Line";
    begin
        Other.Get(GroupCode, EntryNo);
        Other.Note := NewNote;
        Other.Modify();
    end;

    local procedure TouchDetails()
    var
        Detail: Record "LT NX Detail";
    begin
        if Detail.FindSet() then
            repeat
                Detail.Text := 'ByPage';
                Detail.Modify();
            until Detail.Next() = 0;
    end;
}

codeunit 97665 "LT NX Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NX Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NX Lines", Line);
    end;
}

codeunit 97666 "LT NX Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        ShownAfterName: Text;
        ShownAfterValue: Text;

    // The table field writes another table, on a page opened with TestPage.OpenEdit and a filter on the group.
    [Test]
    procedure NewLine_TableFieldWritesAnotherTable_OpenEdit_ShowsAndSavesTheRow()
    var
        Mode: Codeunit "LT NX Mode";
        Lines: TestPage "LT NX Lines";
    begin
        Initialize();
        Mode.Set(1);
        Lines.OpenEdit();
        Lines.Filter.SetFilter("Group Code", 'G1');
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value();
        Lines.Close();
        Mode.Set(0);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'A;', Details());
    end;

    // The table field writes another table, then Update(true), with a row before the new line.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldWritesAnotherTableThenUpdateTrue_RowBefore_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddRow('B');
        RunMode(2);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B//;A//V;', Rows());
        Check('details', 'A;', Details());
    end;

    // The table field writes another table and inserts a second row of the page's table.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldAlsoInsertsAnotherLine_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(4);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;Extra//;', Rows());
        Check('details', 'A;', Details());
    end;

    // The table field writes another table and the control modifies the row before the line, before the save.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ControlStampsTheRowBeforeBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddRow('B');
        RunMode(5);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'B/Stamped/;A//V;', Rows());
        Check('details', 'A;', Details());
    end;

    // The table field writes another table, then sets a field of the line and modifies it through Rec.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldWritesAnotherTableThenModifiesTheLine_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(6);
        Check('shown after the Name SetValue', 'A/Twice/', ShownAfterName);
        Check('shown after the Value SetValue', 'A/Twice/V', ShownAfterValue);
        Check('rows', 'A/Twice/V;', Rows());
        Check('details', 'A;', Details());
    end;

    // The table field and the control both write another table before the save (review of the runner pull request 8806).
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldAndControlWriteAnotherTable_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(8);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'A;ByPage;', Details());
    end;

    // A FlowField of the page's table counts the rows the table field writes; the page shows it (review of the runner pull
    // request 8806).
    [Test]
    [HandlerFunctions('CountHandler')]
    procedure NewLine_FlowFieldOverTheWrittenTable_ShowsTheCount()
    begin
        Initialize();
        RunMode(7);
        Check('shown after the Name SetValue', 'A//1', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V/1', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'A;', Details());
    end;

    // After the insert alone, the control writes the line through another record, inserts into another table, then writes the
    // line again, before the save (follow-up of the review of the runner pull request 8806: :630 and :654 measure one rewrite).
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ControlRewritesTheLineTwiceAroundAnotherTablesWrite_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(9);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'ByPage;', Details());
    end;

    // After the insert alone, the control writes the line twice through another record and no other table.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ControlRewritesTheLineTwiceWithoutAnotherTablesWrite_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(10);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', '', Details());
    end;

    // One rewrite next to a modify of another table is not the insert measured at :630 and :654.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ControlRewritesTheLineAndModifiesAnotherTable_SaveWritesRecOverTheLine()
    begin
        Initialize();
        AddDetail('Old');
        RunMode(11);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'ByPage;', Details());
    end;

    // The table field calls Modify on one existing other-table row (the value stays Old), then the control rewrites the line
    // once. The rewritten cells measure an Insert of another table there, not a Modify.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldModifiesAnotherTableThenControlRewritesTheLine_SaveWritesRecOverTheLine()
    begin
        Initialize();
        AddDetail('Old');
        RunMode(12);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'Old;', Details());
    end;

    // Copilot review of the train for runner pull request 8878, thread 4726039255: the table field calls an unchanged Modify on
    // another table and commits before it inserts the line, then the control inserts into another table and rewrites the line.
    // The Commit empties the write log both boundaries index; the cells measure no Modify of another table and no Commit there.
    // Not measured on BC: both binaries refuse it, so the expected values are only the measured cells' pattern.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldModifiesAnotherTableAndCommitsBeforeInsert_ControlInsertsAndRewrites()
    begin
        Initialize();
        AddDetail('Old');
        RunMode(13);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', 'Old;ByPage;', Details());
    end;

    // Thread 4726039269: the control inserts two rows into a temporary record (no SQLTimestamp field), then rewrites the line.
    // The temporary inserts take row versions but are no noted database writes; no cell measures them. Not measured on BC.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ControlInsertsTwoTemporaryRowsThenRewritesTheLine_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(14);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', '', Details());
    end;

    // The same with one temporary insert: the write mark then moves by two, as with the measured other-table Insert.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ControlInsertsOneTemporaryRowThenRewritesTheLine_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(15);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', '', Details());
    end;

    // Next to thread 4726039269: the table field inserts one temporary row after the line, then the control rewrites the line
    // once. The new write-mark check reads the control's writes only; this shape keeps main's result.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldInsertsATemporaryRowThenControlRewritesTheLine_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(16);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', '', Details());
    end;

    // Next to thread 4726039255: the table field inserts the line and commits, then the control rewrites the line once. Not
    // measured on BC.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldInsertsAndCommitsThenControlRewritesTheLine_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(17);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', '', Details());
    end;

    // Next to thread 4726039255: the table field commits, then inserts the line; the control rewrites the line once. The Commit
    // empties the write log after New(), so the row-version rule alone decides, as on main. Not measured on BC.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_TableFieldCommitsThenInsertsThenControlRewritesTheLine_SaveWritesRecOverTheLine()
    begin
        Initialize();
        RunMode(18);
        Check('shown after the Name SetValue', 'A//', ShownAfterName);
        Check('shown after the Value SetValue', 'A//V', ShownAfterValue);
        Check('rows', 'A//V;', Rows());
        Check('details', '', Details());
    end;

    [ModalPageHandler]
    procedure CountHandler(var Lines: TestPage "LT NX Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        ShownAfterName := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines."Detail Count".Value();
        Lines.Value.SetValue('V');
        ShownAfterValue := Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Value.Value() + '/' + Lines."Detail Count".Value();
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NX Lines")
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
        Line: Record "LT NX Line";
        Detail: Record "LT NX Detail";
        Mode: Codeunit "LT NX Mode";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Mode.Set(0);
        Commit();
        ShownAfterName := '';
        ShownAfterValue := '';
    end;

    local procedure AddRow(Name: Text[30])
    var
        Line: Record "LT NX Line";
    begin
        Line."Group Code" := 'G1';
        Line.Name := Name;
        Line.Insert();
        Commit();
    end;

    local procedure AddDetail(NewText: Text[30])
    var
        Detail: Record "LT NX Detail";
    begin
        Detail.Init();
        Detail.Text := NewText;
        Detail.Insert();
        Commit();
    end;

    local procedure RunMode(M: Integer)
    var
        Mode: Codeunit "LT NX Mode";
        Opener: Codeunit "LT NX Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
        Mode.Set(0);
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NX Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Detail: Record "LT NX Detail";
    begin
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
