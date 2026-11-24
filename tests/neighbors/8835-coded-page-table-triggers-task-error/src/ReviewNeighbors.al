// Neighbors of ListNewLineExtraCodeCodedTableTriggers.al and ListNewLineExtraCodeCodedTaskError.al (review of al-runner
// PR 8840, 01:32Z and 07:30Z): a handler that catches the first SetValue's error, and a copy of the task FactBox whose
// worker fails on the blank record or on the parts' read of the new line, or whose error trigger calls CurrPage.Update.
// Each test holds
// Business Central's answer, measured on a local BC 29.1 container (3 runs of 3), in a codeunit of its own.

// The task error probe's variation FactBox, with modes 5 (fails on the blank record, too), 6 (its error
// trigger calls CurrPage.Update(false)) and 7 (fails at its second read of the blank record, the new line's).
page 96600 "LT NQ4 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NQ2 Line";

    layout
    {
        area(Content)
        {
            field(Usage; Usage) { ApplicationArea = All; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Mode: Codeunit "LT NQ2 Mode";
        Parameters: Dictionary of [Text, Text];
    begin
        Usage := '';
        if (Rec."Entry No." = 0) and (Mode.Get() = 7) and not BlankSeen then begin
            BlankSeen := true;
            exit;
        end;
        if (Rec."Entry No." = 0) and not (Mode.Get() in [5, 7]) then
            exit;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        Parameters.Add('Fail', Format(Mode.Get() in [2, 3, 5, 6, 7]));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NQ2 Var Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        if CompletedTaskId = TaskId then begin
            Usage := Results.Get('Count');
            Mode.Add('U.DONE(' + Usage + ')');
        end;
    end;

    trigger OnPageBackgroundTaskError(ErroredTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Mode.Add('U.ERR(' + ErrorText + ')');
        Usage := 'Failed';
        IsHandled := Mode.Get() in [2, 6];
        if Mode.Get() = 6 then
            CurrPage.Update(false);
    end;

    var
        BlankSeen: Boolean;
        TaskId: Integer;
        Usage: Text;
}

// The task error probe's variation page with that FactBox.
page 96601 "LT NQ4 Lines"
{
    PageType = List;
    SourceTable = "LT NQ2 Line";
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
                        StampMark(Rec);
                        CurrPage.Update(false);
                    end;
                }
                field(Note; Rec.Note) { ApplicationArea = All; }
                field(Mark; Rec.Mark) { ApplicationArea = All; }
                field(Value; Rec.Value) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            systempart(Notes; Notes)
            {
                ApplicationArea = All;
            }
            part(Info; "LT NQ2 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NQ2 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NQ4 Usage")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(MarkLine)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
            action(ClearNote)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Note := '';
                    CurrPage.Update(true);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opened := true;
    end;

    trigger OnAfterGetRecord()
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    local procedure StampMark(Line: Record "LT NQ2 Line")
    var
        Other: Record "LT NQ2 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// Opens the pages, sets up and reads back the rows.
codeunit 96602 "LT NX4 Lib"
{
    procedure Initialize(NeMode: Integer; NqMode: Integer)
    var
        NeLine: Record "LT NE4 Line";
        NeDetail: Record "LT NE2 Detail";
        NqLine: Record "LT NQ2 Line";
        NqDetail: Record "LT NQ2 Detail";
        Log: Codeunit "LT NE4 Log";
        Mode: Codeunit "LT NQ2 Mode";
    begin
        NeLine.DeleteAll();
        NeDetail.DeleteAll();
        NqLine.DeleteAll();
        NqDetail.DeleteAll();
        Log.Reset(NeMode);
        Mode.Set(NqMode);
        Commit();
        ClearLastError();
    end;

    procedure OpenNe()
    var
        Line: Record "LT NE4 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NE4 Lines", Line);
    end;

    procedure OpenNq(Probe: Boolean)
    var
        Line: Record "LT NQ2 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Probe then
            Page.RunModal(Page::"LT NQ2 Var Lines", Line)
        else
            Page.RunModal(Page::"LT NQ4 Lines", Line);
    end;

    procedure NeRows() Result: Text
    var
        Line: Record "LT NE4 Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    procedure NqRows() Result: Text
    var
        Line: Record "LT NQ2 Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    procedure NqLog(): Text
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        exit(Mode.Log());
    end;

    procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('all: expected <%1>, found <%2>.', Expected, Actual);
    end;
}


// The handler catches OnInsert's error and returns. Its own codeunit.
codeunit 96603 "LT NE4 Caught Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NX4 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_HandlerCatchesOnInsertsError_ThePageCloses()
    begin
        Lib.Initialize(5, 0);
        Shown := '';
        Lib.OpenNe();
        Lib.Check('caught=Validation error for Field: Name,  Message = ''Not allowed'' | err=Validation error for Field: Name,  Message = ''Not allowed''| code=TestValidation| rows=', Shown + '| err=' + GetLastErrorText() + '| code=' + GetLastErrorCode() + '| rows=' + Lib.NeRows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NE4 Lines")
    begin
        Lines.New();
        asserterror Lines.Name.SetValue('A');
        Shown += 'caught=' + GetLastErrorText() + ' ';
    end;
}


// The handler catches the unhandled task error and goes on to the Value SetValue. Its own codeunit.
codeunit 96604 "LT NQ4 Caught Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NX4 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_HandlerCatchesTheTaskErrorAndGoesOn_ThePageIsGone()
    begin
        Lib.Initialize(0, 3);
        Shown := '';
        asserterror Lib.OpenNq(true);
        Lib.Check('caught=Task failed | err=The TestPage is not open.| code=TestPageNotOpened| rows=| log=U.ERR(Task failed);|caught;|value;', Shown + '| err=' + GetLastErrorText() + '| code=' + GetLastErrorCode() + '| rows=' + Lib.NqRows() + '| log=' + Lib.NqLog());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NQ2 Var Lines")
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Lines.New();
        asserterror Lines.Name.SetValue('A');
        Mode.Add('|caught');
        Shown += 'caught=' + GetLastErrorText() + ' ';
        Lines.Value.SetValue('V');
        Mode.Add('|value');
        Shown += 'value=' + Lines.Value.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
        Mode.Add('|ok');
    end;
}


// The worker also runs and fails on the blank record at the open, unhandled. Its own codeunit.
codeunit 96605 "LT NQ4 Blank Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NX4 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_TaskFailsOnTheBlankRecord_TheErrorReachesTheTest()
    begin
        Lib.Initialize(0, 5);
        Shown := '';
        asserterror Lib.OpenNq(false);
        Lib.Check('| err=Task failed| code=ClientPageAbort:FormAbort| rows=| log=U.ERR(Task failed);', Shown + '| err=' + GetLastErrorText() + '| code=' + GetLastErrorCode() + '| rows=' + Lib.NqRows() + '| log=' + Lib.NqLog());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NQ4 Lines")
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Mode.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.Value.SetValue('V');
        Mode.Add('|value');
        Shown += 'value=' + Lines.Value.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
        Mode.Add('|ok');
    end;
}


// The error trigger marks the error handled and calls CurrPage.Update(false). Its own codeunit.
codeunit 96606 "LT NQ4 Update Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NX4 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_TaskErrorTriggerCallsUpdate_ShowsTheFailure()
    begin
        Lib.Initialize(0, 6);
        Shown := '';
        Lib.OpenNq(false);
        Lib.Check('name=A usage=Failed value=V usage=Failed | err=| code=| rows=A/Resolved/Stamped/V;| log=U.ERR(Task failed);|name;|value;|ok;', Shown + '| err=' + GetLastErrorText() + '| code=' + GetLastErrorCode() + '| rows=' + Lib.NqRows() + '| log=' + Lib.NqLog());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NQ4 Lines")
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Mode.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.Value.SetValue('V');
        Mode.Add('|value');
        Shown += 'value=' + Lines.Value.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
        Mode.Add('|ok');
    end;
}


// The worker fails only at the parts' read of the new line at the start of the first SetValue. Its own codeunit.
codeunit 96607 "LT NQ4 Line Read Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NX4 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_TaskFailsAtThePartsReadOfTheNewLine_TheErrorReachesTheTest()
    begin
        Lib.Initialize(0, 7);
        Shown := '';
        asserterror Lib.OpenNq(false);
        Lib.Check('| err=Task failed| code=ClientPageAbort:FormAbort| rows=| log=U.ERR(Task failed);', Shown + '| err=' + GetLastErrorText() + '| code=' + GetLastErrorCode() + '| rows=' + Lib.NqRows() + '| log=' + Lib.NqLog());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NQ4 Lines")
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Mode.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.Value.SetValue('V');
        Mode.Add('|value');
        Shown += 'value=' + Lines.Value.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
        Mode.Add('|ok');
    end;
}
