// The corpus probe ListNewLineExtraCodeCodedTaskError.al of corpus pull request 1540 (al-runner issue 8836), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/triggers-auto/page/page-onpagebackgroundtaskerror-trigger
// Scope: in-scope
// local object: a List page with page code, a Notes part and three CardPart FactBoxes whose third enqueues a page background
//   task and declares OnPageBackgroundTaskError, whose table field's OnValidate inserts the page's new line, and the code
//   around CurrPage.SaveRecord() and CurrPage.Update in the control's OnValidate, are what is under test
// Fixtures used: own tables 96520 and 96521, pages 96523, 96524, 96525, 96527, 96529 and 96530, codeunits 96526, 96528 and
//   96531 to 96533, test codeunit 96534
// Probe (al-runner issue 8836): the page of ListNewLineExtraCodeCodedPage.al's repro, with a Notes system part, whose
//   background-task FactBox also declares OnPageBackgroundTaskError; the SetValue on Name runs the table field's OnValidate,
//   which inserts the row (Insert(true), an AutoIncrement key field) and two rows of another table, then the control's
//   OnValidate, which sets another field of Rec, saves, modifies the row through another record and calls CurrPage.Update(false).
//   The same page with a variation part: its task does not fail, or fails and the error trigger marks it handled, or fails and
//   the trigger leaves it unhandled. The page is opened with Page.RunModal on a record filtered to one empty group. Readings:
//   what the page and its parts show after the Name and the Value SetValue, the parts' log, and the rows after OK.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96520 "LT NQ2 Line"
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
        field(4; Note; Text[30]) { }
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    // Writes the default detail rows of the line into another table.
    local procedure CopyDefaults()
    var
        Detail: Record "LT NQ2 Detail";
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

table 96521 "LT NQ2 Detail"
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

// FactBox 1: shows the line's fields and a variable its OnAfterGetRecord sets.
page 96523 "LT NQ2 Info"
{
    PageType = CardPart;
    SourceTable = "LT NQ2 Line";

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
    begin
        Info := Rec.Name;
    end;

    var
        Info: Text;
}

// FactBox 2: a field and an action.
page 96524 "LT NQ2 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NQ2 Line";

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ClearValue)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Rec.Value := '';
                    Rec.Modify();
                end;
            }
        }
    }
}

// FactBox 3: counts the line's detail rows in a page background task, and handles an error of it.
page 96525 "LT NQ2 Usage"
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
        Parameters: Dictionary of [Text, Text];
    begin
        Usage := '';
        if Rec."Entry No." = 0 then
            exit;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NQ2 Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    trigger OnPageBackgroundTaskError(ErroredTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        Usage := 'Failed';
        IsHandled := true;
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

codeunit 96526 "LT NQ2 Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NQ2 Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

// A List page with OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions, a Notes part and three FactBoxes (the third declares
// OnPageBackgroundTaskError): the Name control's OnValidate
// sets another field of Rec, saves the new line, modifies the row through another record and updates the page.
page 96527 "LT NQ2 Lines"
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
            part(Usage; "LT NQ2 Usage")
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

codeunit 96528 "LT NQ2 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NQ2 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NQ2 Lines", Line);
    end;
}

// The variation part: its background task fails when the mode asks it to (2 and 3); its OnPageBackgroundTaskError logs, shows
// the failure and marks the error handled (2) or not (3).
page 96529 "LT NQ2 Var Usage"
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
        if Rec."Entry No." = 0 then
            exit;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        Parameters.Add('Fail', Format(Mode.Get() in [2, 3]));
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
        IsHandled := Mode.Get() = 2;
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

codeunit 96531 "LT NQ2 Var Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NQ2 Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        if Page.GetBackgroundParameters().Get('Fail') = Format(true) then
            Error('Task failed');
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 96532 "LT NQ2 Mode"
{
    SingleInstance = true;

    var
        Current: Integer;
        Entries: Text;

    procedure Set(NewMode: Integer)
    begin
        Current := NewMode;
        Entries := '';
    end;

    procedure Get(): Integer
    begin
        exit(Current);
    end;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Log(): Text
    begin
        exit(Entries);
    end;
}

// The variation page: the repro's page whose third FactBox is the variation part.
page 96530 "LT NQ2 Var Lines"
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
            part(Usage; "LT NQ2 Var Usage")
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

codeunit 96533 "LT NQ2 Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NQ2 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NQ2 Var Lines", Line);
    end;
}

codeunit 96534 "LT NQ2 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeAndTaskErrorTrigger_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NQ2 Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TaskErrorTriggerTheTaskDoesNotRaise_ShowsTheCount()
    begin
        Initialize();
        RunVar(1);
        Check('shown', 'name=A/Resolved/Stamped/ usage=2 value=A/Resolved/Stamped/V usage=2 ', Shown);
        Check('log', 'U.DONE(2);|name;|value;|ok;', Log());
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TaskRaisesAnErrorTheTriggerHandles_ShowsTheFailure()
    begin
        Initialize();
        RunVar(2);
        Check('shown', 'name=A/Resolved/Stamped/ usage=Failed value=A/Resolved/Stamped/V usage=Failed ', Shown);
        Check('log', 'U.ERR(Task failed);|name;|value;|ok;', Log());
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_TaskRaisesAnErrorTheTriggerDoesNotHandle_TheErrorReachesTheTest()
    begin
        Initialize();
        asserterror RunVar(3);
        Check('error', 'Task failed', GetLastErrorText());
        Check('shown', '', Shown);
        Check('log', 'U.ERR(Task failed);', Log());
        Check('rows', '', Rows());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NQ2 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure VarHandler(var Lines: TestPage "LT NQ2 Var Lines")
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Mode.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.Value.SetValue('V');
        Mode.Add('|value');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
        Mode.Add('|ok');
    end;

    local procedure Initialize()
    var
        Line: Record "LT NQ2 Line";
        Detail: Record "LT NQ2 Detail";
        Mode: Codeunit "LT NQ2 Mode";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Mode.Set(0);
        Commit();
        Shown := '';
    end;

    local procedure RunVar(M: Integer)
    var
        Mode: Codeunit "LT NQ2 Mode";
        Opener: Codeunit "LT NQ2 Var Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
    end;

    local procedure Log(): Text
    var
        Mode: Codeunit "LT NQ2 Mode";
    begin
        exit(Mode.Log());
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NQ2 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Line: Record "LT NQ2 Line";
        Detail: Record "LT NQ2 Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Detail.SetRange("Line Entry No.", Line."Entry No.");
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
