// The corpus probe ListNewLineExtraCodeCodedPageSubscriber.al of corpus pull request 1541 (al-runner issue 8842), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/session/session-bindsubscription-method
// Scope: in-scope
// local object: a List page with page code and three CardPart FactBoxes whose third enqueues a page background task, with an
//   event subscriber to the page, whose table field's OnValidate inserts the page's new line, and the code around
//   CurrPage.SaveRecord() and CurrPage.Update in the control's OnValidate, are what is under test
// Fixtures used: own tables 96542 and 96543, pages 96544 to 96546, 96548 and 96551, codeunits 96547, 96549, 96550, 96552 and
//   96553, test codeunit 96554
// Probe (al-runner issue 8842): the page of ListNewLineExtraCodeCodedPage.al's repro, with a codeunit whose
//   EventSubscriberInstance is Manual subscribing to the page's OnModifyRecordEvent; the SetValue on Name runs the table field's
//   OnValidate, which inserts the row (Insert(true), an AutoIncrement key field) and two rows of another table, then the
//   control's OnValidate, which sets another field of Rec, saves, modifies the row through another record and calls
//   CurrPage.Update(false). The subscriber is not bound, bound for the open, or bound and unbound before it; a copy of the page
//   has an automatic subscriber. The page is opened with Page.RunModal on a record filtered to one empty group. Readings: what
//   the page and its first part show after the Name and the Value SetValue, the rows after OK, and what the subscriber logged.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96542 "LT NU4 Line"
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
        Detail: Record "LT NU4 Detail";
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

table 96543 "LT NU4 Detail"
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
page 96544 "LT NU4 Info"
{
    PageType = CardPart;
    SourceTable = "LT NU4 Line";

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
page 96545 "LT NU4 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NU4 Line";

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

// FactBox 3: counts the line's detail rows in a page background task.
page 96546 "LT NU4 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NU4 Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NU4 Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

codeunit 96547 "LT NU4 Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NU4 Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

// A List page with OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions and three FactBoxes, with a manual subscriber
// to its OnModifyRecordEvent (LT NU4 Spy): the Name control's OnValidate
// sets another field of Rec, saves the new line, modifies the row through another record and updates the page.
page 96548 "LT NU4 Lines"
{
    PageType = List;
    SourceTable = "LT NU4 Line";
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
            part(Info; "LT NU4 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NU4 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NU4 Usage")
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

    local procedure StampMark(Line: Record "LT NU4 Line")
    var
        Other: Record "LT NU4 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96549 "LT NU4 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NU4 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NU4 Lines", Line);
    end;
}

// A manual subscriber to the list page's OnModifyRecordEvent: it runs only while an instance is bound, and logs when it does.
codeunit 96550 "LT NU4 Spy"
{
    SingleInstance = true;
    EventSubscriberInstance = Manual;

    var
        Entries: Text;

    [EventSubscriber(ObjectType::Page, Page::"LT NU4 Lines", 'OnModifyRecordEvent', '', false, false)]
    local procedure OnModifyLine(var Rec: Record "LT NU4 Line"; var xRec: Record "LT NU4 Line"; var AllowModify: Boolean)
    begin
        Entries += 'MOD(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + '/' + Rec.Value + ');';
    end;

    procedure Log(): Text
    begin
        exit(Entries);
    end;

    procedure Reset()
    begin
        Entries := '';
    end;
}

// The repro's page, with an automatic subscriber to its OnModifyRecordEvent (LT NU4 Auto Spy).
page 96551 "LT NU4 Auto Lines"
{
    PageType = List;
    SourceTable = "LT NU4 Line";
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
            part(Info; "LT NU4 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NU4 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NU4 Usage")
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

    local procedure StampMark(Line: Record "LT NU4 Line")
    var
        Other: Record "LT NU4 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// An automatic subscriber to the copy's OnModifyRecordEvent: it runs at every modify of the page, and logs it.
codeunit 96552 "LT NU4 Auto Spy"
{
    SingleInstance = true;

    var
        Entries: Text;

    [EventSubscriber(ObjectType::Page, Page::"LT NU4 Auto Lines", 'OnModifyRecordEvent', '', false, false)]
    local procedure OnModifyLine(var Rec: Record "LT NU4 Line"; var xRec: Record "LT NU4 Line"; var AllowModify: Boolean)
    begin
        Entries += 'MOD(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + '/' + Rec.Value + ');';
    end;

    procedure Log(): Text
    begin
        exit(Entries);
    end;

    procedure Reset()
    begin
        Entries := '';
    end;
}

codeunit 96553 "LT NU4 Auto Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NU4 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NU4 Auto Lines", Line);
    end;
}

codeunit 96554 "LT NU4 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeAndUnboundPageSubscriber_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NU4 Opener";
        Spy: Codeunit "LT NU4 Spy";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details());
        Check('spy', '', Spy.Log());
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_PageSubscriberBoundAndUnboundBeforeTheOpen_RunsNothing()
    var
        Opener: Codeunit "LT NU4 Opener";
        Spy: Codeunit "LT NU4 Spy";
    begin
        Initialize();
        BindSubscription(Spy);
        UnbindSubscription(Spy);
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('spy', '', Spy.Log());
    end;

    [Test]
    [HandlerFunctions('AutoHandler')]
    procedure NewLine_AutomaticPageSubscriber_RunsAtEachModifyOfThePage()
    var
        Opener: Codeunit "LT NU4 Auto Opener";
        AutoSpy: Codeunit "LT NU4 Auto Spy";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('spy', 'MOD(A/Resolved//);MOD(A/Resolved/Stamped/V);', AutoSpy.Log());
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_BoundPageSubscriber_RunsAtEachModifyOfThePage()
    var
        Opener: Codeunit "LT NU4 Opener";
        Spy: Codeunit "LT NU4 Spy";
    begin
        Initialize();
        BindSubscription(Spy);
        Opener.Open('G1');
        UnbindSubscription(Spy);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('spy', 'MOD(A/Resolved//);MOD(A/Resolved/Stamped/V);', Spy.Log());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NU4 Lines")
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
    procedure AutoHandler(var Lines: TestPage "LT NU4 Auto Lines")
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

    local procedure Initialize()
    var
        Line: Record "LT NU4 Line";
        Detail: Record "LT NU4 Detail";
        Spy: Codeunit "LT NU4 Spy";
        AutoSpy: Codeunit "LT NU4 Auto Spy";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Spy.Reset();
        AutoSpy.Reset();
        Commit();
        Shown := '';
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NU4 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Line: Record "LT NU4 Line";
        Detail: Record "LT NU4 Detail";
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
