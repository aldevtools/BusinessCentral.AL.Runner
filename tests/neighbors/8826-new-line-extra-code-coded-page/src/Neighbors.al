// Neighbors of ListNewLineExtraCodeCodedPage.al (al-runner issue 8826): pages of the probe's shape whose code goes just past
// what its cells measure, over the probe's table and with its parts where they fit. Each test holds Business Central's answer,
// measured on a local BC 29.1 container (3 runs of 3).

// A part like the probe's first FactBox whose OnAfterGetRecord also writes another table.
page 96862 "LT NG2 Writer Info"
{
    PageType = CardPart;
    SourceTable = "LT NZ Line";

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
        Detail: Record "LT NZ Detail";
    begin
        Info := Rec.Name;
        if Rec.Name = '' then
            exit;
        Detail.Init();
        Detail."Line Entry No." := -1;
        Detail.Text := 'ByPart';
        Detail.Insert();
    end;

    var
        Info: Text;
}

// A part like the probe's third FactBox that also declares OnPageBackgroundTaskError.
page 96863 "LT NG2 Error Usage"
{
    PageType = CardPart;
    SourceTable = "LT NZ Line";

    layout
    {
        area(Content)
        {
            field(Usage; Usage) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
    begin
        Usage := '';
        if Rec."Entry No." = 0 then
            exit;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NZ Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    trigger OnPageBackgroundTaskError(ErroredTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        IsHandled := true;
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

// The probe's repro page with OnAfterGetCurrRecord as well.
page 96864 "LT NG2 Curr Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
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
                        Stamp: Codeunit "LT NZ Stamp";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Stamp.StampMark(Rec);
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
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrentName := Rec.Name;
    end;

    var
        Opened: Boolean;
        CurrentName: Text;
}

// The probe's repro page whose first FactBox writes another table in its OnAfterGetRecord.
page 96865 "LT NG2 Writer Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
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
                        Stamp: Codeunit "LT NZ Stamp";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Stamp.StampMark(Rec);
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
            part(Info; "LT NG2 Writer Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    var
        Opened: Boolean;
}

// The probe's repro page whose control also writes another table before the save.
page 96866 "LT NG2 Control Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
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
                        Detail: Record "LT NZ Detail";
                        Stamp: Codeunit "LT NZ Stamp";
                    begin
                        Detail.Init();
                        Detail."Line Entry No." := -1;
                        Detail.Text := 'ByPage';
                        Detail.Insert();
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Stamp.StampMark(Rec);
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
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    var
        Opened: Boolean;
}

// The probe's repro page whose third FactBox declares OnPageBackgroundTaskError.
page 96867 "LT NG2 Error Lines"
{
    PageType = List;
    SourceTable = "LT NZ Line";
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
                        Stamp: Codeunit "LT NZ Stamp";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Stamp.StampMark(Rec);
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
            part(Info; "LT NZ Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NG2 Error Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    var
        Opened: Boolean;
}

codeunit 96868 "LT NG2 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    // The page declares OnAfterGetCurrRecord too.
    [Test]
    [HandlerFunctions('CurrHandler')]
    procedure NewLine_PageWithOnAfterGetCurrRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        Open(Page::"LT NG2 Curr Lines");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details(0));
    end;

    // The first FactBox's OnAfterGetRecord writes another table.
    [Test]
    [HandlerFunctions('WriterHandler')]
    procedure NewLine_PartTriggerWritesAnotherTable_ShowsAndSavesTheRow()
    begin
        Initialize();
        Open(Page::"LT NG2 Writer Lines");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details(0));
        Check('written by the part', '1', Format(Written()));
    end;

    // The control writes another table before the save.
    [Test]
    [HandlerFunctions('ControlHandler')]
    procedure NewLine_ControlWritesAnotherTableBeforeSaveRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        Open(Page::"LT NG2 Control Lines");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details(0));
        Check('written by the control', '1', Format(Written()));
    end;

    // The third FactBox declares OnPageBackgroundTaskError.
    [Test]
    [HandlerFunctions('ErrorHandler')]
    procedure NewLine_PartWithBackgroundTaskError_ShowsAndSavesTheRow()
    begin
        Initialize();
        Open(Page::"LT NG2 Error Lines");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details(0));
    end;

    [ModalPageHandler]
    procedure CurrHandler(var Lines: TestPage "LT NG2 Curr Lines")
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
    procedure WriterHandler(var Lines: TestPage "LT NG2 Writer Lines")
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
    procedure ControlHandler(var Lines: TestPage "LT NG2 Control Lines")
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
    procedure ErrorHandler(var Lines: TestPage "LT NG2 Error Lines")
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
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Commit();
        Shown := '';
    end;

    local procedure Open(PageId: Integer)
    var
        Opener: Codeunit "LT NZ Opener";
    begin
        Opener.Open('G1', PageId);
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NZ Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details(Skip: Integer) Result: Text
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Detail.SetRange("Line Entry No.", Line."Entry No." + Skip);
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Written(): Integer
    var
        Detail: Record "LT NZ Detail";
    begin
        Detail.SetRange("Line Entry No.", -1);
        exit(Detail.Count());
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
