// Neighbors of ListNewLineExtraCodeFactbox.al (al-runner issue 8734): a List page with page triggers and a CardPart FactBox,
// whose code goes just past what its cells measure. Each test holds Business Central's answer, measured on a local BC 29.1
// container (3 runs of 3).
table 97701 "LT NG Line"
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
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

table 97702 "LT NG Other"
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

codeunit 97703 "LT NG Mode"
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

    procedure AddOther(NewText: Text[30])
    var
        Other: Record "LT NG Other";
    begin
        Other.Init();
        Other.Text := NewText;
        Other.Insert();
    end;

    procedure ChangeOther(NewText: Text[30])
    var
        Other: Record "LT NG Other";
    begin
        Other.FindFirst();
        Other.Text := NewText;
        Other.Modify();
    end;

    procedure StampMark(Line: Record "LT NG Line")
    var
        Other: Record "LT NG Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;
}

page 97704 "LT NG Info"
{
    PageType = CardPart;
    SourceTable = "LT NG Line";

    layout
    {
        area(Content)
        {
            field(Note; Rec.Note) { ApplicationArea = All; }
            field(Mark; Rec.Mark) { ApplicationArea = All; }
            field(Info; Info) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetRecord()
    var
        Mode: Codeunit "LT NG Mode";
    begin
        Info := Rec.Name;
        if (Mode.Get() = 2) and (Rec.Name <> '') then
            Mode.AddOther('ByPart');
    end;

    var
        Info: Text;
}

page 97705 "LT NG Lines"
{
    PageType = List;
    SourceTable = "LT NG Line";
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
                        Mode: Codeunit "LT NG Mode";
                    begin
                        case Mode.Get() of
                            3:
                                Mode.StampMark(Rec);
                            4:
                                Mode.AddOther('ByPage');
                        end;
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        if Mode.Get() in [0, 1, 2] then
                            Mode.StampMark(Rec);
                        case Mode.Get() of
                            10, 12:
                                Rec."Entry No." += 100;
                            11:
                                Rec.Next(-1);
                            13:
                                Rec.Find('=');
                        end;
                        if Mode.Get() = 12 then
                            Mode.AddOther('ByPage');
                        if Mode.Get() = 14 then
                            Mode.ChangeOther('ByPage');
                        if not (Mode.Get() in [8, 10, 11, 12, 13, 14]) then
                            CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Mode: Codeunit "LT NG Mode";
    begin
        if (Mode.Get() = 1) and (Rec.Name <> '') then
            Rec.Value := 'Got';
    end;

    trigger OnAfterGetCurrRecord()
    var
        Mode: Codeunit "LT NG Mode";
    begin
        CurrentName := Rec.Name;
        if (Mode.Get() = 9) and (Rec.Name = 'A') then
            Rec.SetRange(Name, 'A');
    end;

    var
        CurrentName: Text;
}

// The same page without OnAfterGetCurrRecord.
page 97706 "LT NG Lines Read"
{
    PageType = List;
    SourceTable = "LT NG Line";
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
                        Mode: Codeunit "LT NG Mode";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Mode.StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
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

// The same page whose Name control calls CurrPage.Update(true) or Update() without a save, or Update(true) after it.
page 97709 "LT NG Upd Lines"
{
    PageType = List;
    SourceTable = "LT NG Line";
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
                        Mode: Codeunit "LT NG Mode";
                    begin
                        Rec.Note := 'Resolved';
                        case Mode.Get() of
                            5:
                                CurrPage.Update(true);
                            6:
                                CurrPage.Update();
                            7:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(true);
                                end;
                        end;
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Mode: Codeunit "LT NG Mode";
    begin
        if (Mode.Get() = 1) and (Rec.Name <> '') then
            Rec.Value := 'Got';
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrentName := Rec.Name;
    end;

    var
        CurrentName: Text;
}

// The same page whose OnAfterGetCurrRecord sets a field of Rec on the new line.
page 97710 "LT NG Curr Lines"
{
    PageType = List;
    SourceTable = "LT NG Line";
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
                        Mode: Codeunit "LT NG Mode";
                    begin
                        case Mode.Get() of
                            3:
                                Mode.StampMark(Rec);
                            4:
                                Mode.AddOther('ByPage');
                        end;
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        if Mode.Get() in [0, 1, 2] then
                            Mode.StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NG Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    var
        Mode: Codeunit "LT NG Mode";
    begin
        if (Mode.Get() = 1) and (Rec.Name <> '') then
            Rec.Value := 'Got';
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrentName := Rec.Name;
        if Rec."Entry No." = 0 then
            Rec.Value := 'Init';
    end;

    var
        CurrentName: Text;
}

codeunit 97707 "LT NG Opener"
{
    procedure Open(GroupCode: Code[10]; PageId: Integer)
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, Line);
    end;
}

codeunit 97708 "LT NG Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    // The page's OnAfterGetRecord sets a field of Rec.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PageTriggerSetsAField_ShowsIt()
    begin
        Initialize();
        RunMode(1, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved/Stamped/Got part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    // The part's OnAfterGetRecord writes another table.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PartTriggerWritesAnotherTable_ShowsAndSavesTheRow()
    var
        Other: Record "LT NG Other";
    begin
        Initialize();
        RunMode(2, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('others', '1', Format(Other.Count()));
    end;

    // The control modifies the row through another record before the save.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ModifyThroughAnotherRecordBeforeSaveRecord_SaveWritesRecOverIt()
    begin
        Initialize();
        RunMode(3, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
    end;

    // The control writes another table before the save.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_WriteAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    var
        Other: Record "LT NG Other";
    begin
        Initialize();
        RunMode(4, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
        Check('others', '1', Format(Other.Count()));
    end;

    // The page declares OnAfterGetRecord only, with the same part.
    [Test]
    [HandlerFunctions('ReadHandler')]
    procedure NewLine_PageWithoutOnAfterGetCurrRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(0, Page::"LT NG Lines Read");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    // Update(true) without a save, Update() without a save, and Update(true) after the save.
    [Test]
    [HandlerFunctions('UpdHandler')]
    procedure NewLine_UpdateTrueWithoutSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(5, Page::"LT NG Upd Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
    end;

    [Test]
    [HandlerFunctions('UpdHandler')]
    procedure NewLine_UpdateWithoutSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(6, Page::"LT NG Upd Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
    end;

    [Test]
    [HandlerFunctions('UpdHandler')]
    procedure NewLine_SaveRecordThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(7, Page::"LT NG Upd Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
    end;

    [ModalPageHandler]
    procedure UpdHandler(var Lines: TestPage "LT NG Upd Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
    end;

    // The control sets a field of Rec and saves, with no Update after it, with a row before the new line.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_SaveRecordWithoutUpdate_RowBefore_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddRow('B');
        RunMode(8, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'B///;A/Resolved//V;', Rows());
    end;

    // The page's OnAfterGetCurrRecord sets a filter on Rec in the refresh of the Update, with a row before.
    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_CurrRecordTriggerFiltersRec_RowBefore_ShowsAndSavesTheRow()
    begin
        Initialize();
        AddRow('B');
        RunMode(9, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved// part=Resolved//A value=A/Resolved//V part=Resolved//A ', Shown);
        Check('rows', 'B///;A/Resolved//V;', Rows());
    end;

    // After the landing: Previous() or Next(), and after a SetValue Previous() or Close(), instead of OK.
    [Test]
    [HandlerFunctions('PreviousHandler')]
    procedure NewLine_PreviousAfterTheLanding_StaysOnTheRow()
    begin
        Initialize();
        RunMode(0, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved/Stamped/ previous=No then=A/Resolved/Stamped/ ', Shown);
        Check('rows', 'A/Resolved/Stamped/;', Rows());
    end;

    [Test]
    [HandlerFunctions('PreviousTypedHandler')]
    procedure NewLine_PreviousAfterASetValue_SavesTheRow()
    begin
        Initialize();
        RunMode(0, Page::"LT NG Lines");
        Check('shown', 'value=A/Resolved/Stamped/V previous=No then=A/Resolved/Stamped/V ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('NextHandler')]
    procedure NewLine_NextAfterTheLanding_MovesToTheNewLine()
    begin
        Initialize();
        RunMode(0, Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved/Stamped/ next=Yes then=/// ', Shown);
        Check('rows', 'A/Resolved/Stamped/;', Rows());
    end;

    [Test]
    [HandlerFunctions('CloseHandler')]
    procedure NewLine_CloseAfterASetValue_RunModalFails()
    begin
        Initialize();
        asserterror RunMode(0, Page::"LT NG Lines");
        Check('error', 'The RunModal procedure could not close the page 97705 as it has already been closed.', GetLastErrorText());
    end;

    // The page's OnAfterGetCurrRecord sets a field of Rec on the new line at New().
    [Test]
    [HandlerFunctions('CurrHandler')]
    procedure NewLine_CurrRecordTriggerSetsAFieldOfTheNewLine_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunMode(0, Page::"LT NG Curr Lines");
        Check('shown', 'new=/Init name=A/Resolved/Stamped/Init part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/Init;', Rows());
    end;

    [ModalPageHandler]
    procedure PreviousHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Shown += 'previous=' + Format(Lines.Previous()) + ' ';
        Shown += 'then=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure PreviousTypedHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Shown += 'previous=' + Format(Lines.Previous()) + ' ';
        Shown += 'then=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure NextHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Shown += 'next=' + Format(Lines.Next()) + ' ';
        Shown += 'then=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure CloseHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Close();
    end;

    [ModalPageHandler]
    procedure CurrHandler(var Lines: TestPage "LT NG Curr Lines")
    begin
        Lines.New();
        Shown += 'new=' + Lines.Name.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure ReadHandler(var Lines: TestPage "LT NG Lines Read")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NG Line";
        Other: Record "LT NG Other";
        Mode: Codeunit "LT NG Mode";
    begin
        Line.DeleteAll();
        Other.DeleteAll();
        Mode.Set(0);
        Commit();
        Shown := '';
    end;

    local procedure AddRow(Name: Text[30])
    var
        Line: Record "LT NG Line";
    begin
        Line."Group Code" := 'G1';
        Line.Name := Name;
        Line.Insert();
        Commit();
    end;

    local procedure RunMode(M: Integer; PageId: Integer)
    var
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1', PageId);
        Mode.Set(0);
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

// Previous(), then Next(), after a SetValue on the row the new line became. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97711 "LT NG Prev Next Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('PrevNextHandler')]
    procedure NewLine_PreviousThenNextAfterASetValue_SavesTheRow()
    var
        Line: Record "LT NG Line";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Mode.Set(0);
        Commit();
        Shown := '';
        Opener.Open('G1', Page::"LT NG Lines");
        Check('shown', 'value=A/Resolved/Stamped/V previous=No next=Yes then=/// ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [ModalPageHandler]
    procedure PrevNextHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Shown += 'previous=' + Format(Lines.Previous()) + ' ';
        Shown += 'next=' + Format(Lines.Next()) + ' ';
        Shown += 'then=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

// Close() right after the landing, with no SetValue. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97712 "LT NG Close Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('CloseAtLandingHandler')]
    procedure NewLine_CloseAfterTheLanding_RunModalFails()
    var
        Line: Record "LT NG Line";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Mode.Set(0);
        Commit();
        Shown := '';
        asserterror Opener.Open('G1', Page::"LT NG Lines");
        Check('shown', 'name=A/Resolved/Stamped/ ', Shown);
        Check('error', 'The RunModal procedure could not close the page 97705 as it has already been closed.', GetLastErrorText());
        Check('rows', '', Rows());
    end;

    [ModalPageHandler]
    procedure CloseAtLandingHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Close();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

// The save, then Rec read again by its key (Rec.Find('=')), with no Update: the saved key kept. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97713 "LT NG Key Kept Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure NewLine_RecFoundAgainAfterSaveRecordWithoutUpdate_KeepsTheRow()
    var
        Line: Record "LT NG Line";
        Other: Record "LT NG Other";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Other.DeleteAll();
        Mode.Set(13);
        Commit();
        Shown := '';
        Opener.Open('G1', Page::"LT NG Lines");
        Mode.Set(0);
        Check('shown', 'name=A/Resolved// value=A/Resolved//V ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
        Check('others', '', Others());
    end;

    [ModalPageHandler]
    procedure KeyHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Others() Result: Text
    var
        Other: Record "LT NG Other";
    begin
        if Other.FindSet() then
            repeat
                Result += Other.Text + ';';
            until Other.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

// The save, then a primary-key field of Rec set, with no Update. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97714 "LT NG Key Set Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure NewLine_KeyFieldSetAfterSaveRecordWithoutUpdate_ChangesTheKey()
    var
        Line: Record "LT NG Line";
        Other: Record "LT NG Other";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Other.DeleteAll();
        Mode.Set(10);
        Commit();
        Shown := '';
        Opener.Open('G1', Page::"LT NG Lines");
        Mode.Set(0);
        Check('shown', 'name=A/Resolved// value=A/Resolved//V ', Shown);
        Check('rows', 'A/Resolved//;', Rows());
        Check('others', '', Others());
    end;

    [ModalPageHandler]
    procedure KeyHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Others() Result: Text
    var
        Other: Record "LT NG Other";
    begin
        if Other.FindSet() then
            repeat
                Result += Other.Text + ';';
            until Other.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

// The save, then Rec moved to the row before (Rec.Next(-1)), with no Update. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97715 "LT NG Key Moved Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure NewLine_RecMovedAfterSaveRecordWithoutUpdate_RowBefore_ChangesTheRow()
    var
        Line: Record "LT NG Line";
        Other: Record "LT NG Other";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Other.DeleteAll();
        Line."Group Code" := 'G1';
        Line.Name := 'B';
        Line.Insert();
        Mode.Set(11);
        Commit();
        Shown := '';
        Opener.Open('G1', Page::"LT NG Lines");
        Mode.Set(0);
        Check('shown', 'name=A/// value=A///V ', Shown);
        Check('rows', 'B///;A/Resolved//;A///V;', Rows());
        Check('others', '', Others());
    end;

    [ModalPageHandler]
    procedure KeyHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Others() Result: Text
    var
        Other: Record "LT NG Other";
    begin
        if Other.FindSet() then
            repeat
                Result += Other.Text + ';';
            until Other.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

// The save, then a primary-key field of Rec set and a row of another table inserted, with no Update. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97716 "LT NG Key Other Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure NewLine_KeyFieldSetAndAnotherTableWrittenAfterSaveRecordWithoutUpdate_ChangesTheKey()
    var
        Line: Record "LT NG Line";
        Other: Record "LT NG Other";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Other.DeleteAll();
        Mode.Set(12);
        Commit();
        Shown := '';
        Opener.Open('G1', Page::"LT NG Lines");
        Mode.Set(0);
        Check('shown', 'name=A/Resolved// value=A/Resolved//V ', Shown);
        Check('rows', 'A/Resolved//;', Rows());
        Check('others', 'ByPage;', Others());
    end;

    [ModalPageHandler]
    procedure KeyHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Others() Result: Text
    var
        Other: Record "LT NG Other";
    begin
        if Other.FindSet() then
            repeat
                Result += Other.Text + ';';
            until Other.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
// The save, then a row of another table modified, with no Update. Its own codeunit, so that no earlier refused test hides its result.
codeunit 97717 "LT NG Other Modified Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('KeyHandler')]
    procedure NewLine_AnotherTableModifiedAfterSaveRecordWithoutUpdate_ShowsAndSavesTheRow()
    var
        Line: Record "LT NG Line";
        Other: Record "LT NG Other";
        Mode: Codeunit "LT NG Mode";
        Opener: Codeunit "LT NG Opener";
    begin
        Line.DeleteAll();
        Other.DeleteAll();
        Mode.AddOther('Before');
        Mode.Set(14);
        Commit();
        Shown := '';
        Opener.Open('G1', Page::"LT NG Lines");
        Mode.Set(0);
        Check('shown', 'name=A/Resolved// value=A/Resolved//V ', Shown);
        Check('rows', 'A/Resolved//V;', Rows());
        Check('others', 'ByPage;', Others());
    end;

    [ModalPageHandler]
    procedure KeyHandler(var Lines: TestPage "LT NG Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NG Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Others() Result: Text
    var
        Other: Record "LT NG Other";
    begin
        if Other.FindSet() then
            repeat
                Result += Other.Text + ';';
            until Other.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
