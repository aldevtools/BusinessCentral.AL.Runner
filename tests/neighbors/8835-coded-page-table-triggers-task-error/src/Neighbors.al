// Neighbors of ListNewLineExtraCodeCodedTaskError.al (al-runner issue 8836): the background-task FactBox's
// OnPageBackgroundTaskError does more than the cells' (it writes another table, or calls CurrPage.Update(false)), on the probe's
// page shape over its table. Each test holds Business Central's answer, measured on a local BC 29.1 container (3 runs of 3).

// The task fails; the error trigger writes another table and marks the error handled.
page 96535 "LT NQ3 Writer Usage"
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
        Parameters.Add('Fail', Format(true));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NQ2 Var Task", Parameters);
    end;

    trigger OnPageBackgroundTaskError(ErroredTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    var
        Detail: Record "LT NQ2 Detail";
    begin
        Detail.Init();
        Detail."Line Entry No." := -1;
        Detail.Text := 'ByError';
        Detail.Insert();
        Usage := 'Failed';
        IsHandled := true;
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

// The task fails; the error trigger calls CurrPage.Update(false) and marks the error handled.
page 96536 "LT NQ3 Update Usage"
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
        Parameters.Add('Fail', Format(true));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NQ2 Var Task", Parameters);
    end;

    trigger OnPageBackgroundTaskError(ErroredTaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean)
    begin
        Usage := 'Failed';
        IsHandled := true;
        CurrPage.Update(false);
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

page 96537 "LT NQ3 Writer Lines"
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
                    var
                        Other: Record "LT NQ2 Line";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Other.Get(Rec."Group Code", Rec."Entry No.");
                        Other.Mark := 'Stamped';
                        Other.Modify();
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
            part(Usage; "LT NQ3 Writer Usage")
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

page 96538 "LT NQ3 Update Lines"
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
                    var
                        Other: Record "LT NQ2 Line";
                    begin
                        Rec.Note := 'Resolved';
                        CurrPage.SaveRecord();
                        Other.Get(Rec."Group Code", Rec."Entry No.");
                        Other.Mark := 'Stamped';
                        Other.Modify();
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
            part(Usage; "LT NQ3 Update Usage")
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

codeunit 96539 "LT NQ3 Opener"
{
    procedure Open(PageId: Integer)
    var
        Line: Record "LT NQ2 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(PageId, Line);
    end;
}

codeunit 96540 "LT NQ3 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    // The error trigger writes another table.
    [Test]
    [HandlerFunctions('WriterHandler')]
    procedure NewLine_TaskErrorTriggerWritesAnotherTable_ShowsTheFailure()
    var
        Opener: Codeunit "LT NQ3 Opener";
    begin
        Initialize();
        Opener.Open(Page::"LT NQ3 Writer Lines");
        Check('shown', 'name=A/Resolved/Stamped/ usage=Failed value=A/Resolved/Stamped/V usage=Failed ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('written by the error trigger', '1', Format(Written()));
    end;

    // The error trigger calls CurrPage.Update(false).
    [Test]
    [HandlerFunctions('UpdateHandler')]
    procedure NewLine_TaskErrorTriggerUpdatesThePart_ShowsTheFailure()
    var
        Opener: Codeunit "LT NQ3 Opener";
    begin
        Initialize();
        Opener.Open(Page::"LT NQ3 Update Lines");
        Check('shown', 'name=A/Resolved/Stamped/ usage=Failed value=A/Resolved/Stamped/V usage=Failed ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
    end;

    [ModalPageHandler]
    procedure WriterHandler(var Lines: TestPage "LT NQ3 Writer Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure UpdateHandler(var Lines: TestPage "LT NQ3 Update Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NQ2 Line";
        Detail: Record "LT NQ2 Detail";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Commit();
        Shown := '';
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

    local procedure Written(): Integer
    var
        Detail: Record "LT NQ2 Detail";
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
