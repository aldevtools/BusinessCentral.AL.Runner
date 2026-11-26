// The corpus probe ListNewLineExtraCodeCodedPage.al of corpus pull request 1537 (al-runner issue 8826), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: a List page with page code and three CardPart FactBoxes (one enqueues a page background task) whose table
//   field's OnValidate inserts the page's new line, and the code around CurrPage.SaveRecord() and CurrPage.Update in the
//   control's OnValidate, are what is under test
// Fixtures used: own tables 96843, 96844 and 96845, pages 96850 to 96859 and 96861, codeunits 96846 to 96849, test codeunit
//   96860
// Probe (al-runner issue 8826): a List page with AutoSplitKey, OnOpenPage, OnAfterGetRecord (it sets a variable a column
//   shows), OnDeleteRecord and two actions, with three CardPart FactBoxes linked to the line's key (one sets a variable in
//   OnAfterGetRecord, one has a field and an action, one enqueues a page background task in OnAfterGetCurrRecord that counts
//   the line's detail rows), stands on its new line. The SetValue on Name runs the table field's OnValidate, which inserts the
//   row (Insert(true), an AutoIncrement key field) and two rows of another table, then the control's OnValidate. In the repro
//   of 8826 it sets another field of Rec, saves, modifies the row through another record and calls CurrPage.Update(false). A
//   second page logs its triggers and its parts' and varies the code one step at a time (only the save, the modify through
//   another record after the save, the field set before the save; the repro's code also with one row before the new line).
//   A third page is the repro's over a table that declares OnInsert and OnModify. The page is opened with Page.RunModal on
//   a record filtered to one group. Readings: what the page and its parts show after the Name and the Value SetValue, the
//   rows of both tables after OK, and the triggers' log with a mark at each step of the test.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96843 "LT NZ Line"
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
        Detail: Record "LT NZ Detail";
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

table 96844 "LT NZ Detail"
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

// The repro's table with OnInsert and OnModify, which log.
table 96845 "LT NZ Trig Line"
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

    trigger OnInsert()
    var
        Log: Codeunit "LT NZ Log";
    begin
        Log.Add('T.INS(' + Name + ')');
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT NZ Log";
    begin
        Log.Add('T.MOD(' + Name + '/' + Note + '/' + Mark + ')');
    end;
}

codeunit 96846 "LT NZ Log"
{
    SingleInstance = true;

    var
        Entries: Text;
        Mode: Integer;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;

    procedure Reset(NewMode: Integer)
    begin
        Entries := '';
        Mode := NewMode;
    end;

    procedure GetMode(): Integer
    begin
        exit(Mode);
    end;
}

codeunit 96847 "LT NZ Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NZ Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 96848 "LT NZ Opener"
{
    procedure Open(GroupCode: Code[10]; PageId: Integer)
    var
        Line: Record "LT NZ Line";
        TrigLine: Record "LT NZ Trig Line";
    begin
        if PageId = Page::"LT NZ Trig Lines" then begin
            TrigLine.SetRange("Group Code", GroupCode);
            Page.RunModal(PageId, TrigLine);
        end else begin
            Line.SetRange("Group Code", GroupCode);
            Page.RunModal(PageId, Line);
        end;
    end;
}

codeunit 96849 "LT NZ Stamp"
{
    procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    procedure StampTrigMark(Line: Record "LT NZ Trig Line")
    var
        Other: Record "LT NZ Trig Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;
}

// FactBox 1 of the repro: shows the line's fields and a variable its OnAfterGetRecord sets.
page 96850 "LT NZ Info"
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
    begin
        Info := Rec.Name;
    end;

    var
        Info: Text;
}

// FactBox 2: a field and an action.
page 96851 "LT NZ Picture"
{
    PageType = CardPart;
    SourceTable = "LT NZ Line";

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

// FactBox 3 of the repro: counts the line's detail rows in a page background task.
page 96852 "LT NZ Usage"
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

    var
        TaskId: Integer;
        Usage: Text;
}

// The repro of 8826.
page 96853 "LT NZ Lines"
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

    local procedure StampMark(Line: Record "LT NZ Line")
    var
        Other: Record "LT NZ Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The logging page's FactBox 1.
page 96854 "LT NZ Var Info"
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
        Log: Codeunit "LT NZ Log";
    begin
        Info := Rec.Name;
        Log.Add('I.AGR(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + ')');
    end;

    var
        Info: Text;
}

// The logging page's FactBox 3.
page 96855 "LT NZ Var Usage"
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
        Log: Codeunit "LT NZ Log";
        Parameters: Dictionary of [Text, Text];
    begin
        Log.Add('U.AGCR(' + Rec.Name + ')');
        Usage := '';
        if Rec."Entry No." = 0 then
            exit;
        Parameters.Add('Entry', Format(Rec."Entry No."));
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NZ Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    var
        Log: Codeunit "LT NZ Log";
    begin
        if CompletedTaskId = TaskId then begin
            Usage := Results.Get('Count');
            Log.Add('U.DONE(' + Usage + ')');
        end;
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

// The logging page: its triggers and its parts' log what they see; the mode of the log decides the control's code.
page 96856 "LT NZ Var Lines"
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
                        Log: Codeunit "LT NZ Log";
                        Stamp: Codeunit "LT NZ Stamp";
                    begin
                        case Log.GetMode() of
                            1:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.SaveRecord();
                                    Stamp.StampMark(Rec);
                                    CurrPage.Update(false);
                                end;
                            2:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            3:
                                begin
                                    CurrPage.SaveRecord();
                                    Stamp.StampMark(Rec);
                                    CurrPage.Update(false);
                                end;
                            4:
                                begin
                                    Rec.Note := 'Resolved';
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                        end;
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
            part(Info; "LT NZ Var Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Var Usage")
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
                var
                    Stamp: Codeunit "LT NZ Stamp";
                begin
                    Stamp.StampMark(Rec);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    var
        Log: Codeunit "LT NZ Log";
    begin
        Opened := true;
        Log.Add('OPEN');
    end;

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "LT NZ Log";
    begin
        Kind := '';
        if Opened and (Rec.Note <> '') then
            Kind := 'Noted';
        Log.Add('AGR(' + Rec.Name + '/' + Rec.Note + '/' + Rec.Mark + ')');
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

// The repro's FactBoxes over the table with OnInsert and OnModify.
page 96857 "LT NZ Trig Info"
{
    PageType = CardPart;
    SourceTable = "LT NZ Trig Line";

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

page 96858 "LT NZ Trig Picture"
{
    PageType = CardPart;
    SourceTable = "LT NZ Trig Line";

    layout
    {
        area(Content)
        {
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }
}

page 96859 "LT NZ Trig Usage"
{
    PageType = CardPart;
    SourceTable = "LT NZ Trig Line";

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

    var
        TaskId: Integer;
        Usage: Text;
}

// The repro's page over the table with OnInsert and OnModify.
page 96861 "LT NZ Trig Lines"
{
    PageType = List;
    SourceTable = "LT NZ Trig Line";
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
                        Stamp.StampTrigMark(Rec);
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
            part(Info; "LT NZ Trig Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NZ Trig Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NZ Trig Usage")
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

    trigger OnAfterGetRecord()
    begin
        Opened := true;
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    var
        Opened: Boolean;
}

codeunit 96860 "LT NZ Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeAndFactBoxes_ShowsAndSavesTheRow()
    begin
        Initialize();
        Open(0, Page::"LT NZ Lines");
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeAndFactBoxes_LogsTheTriggers()
    begin
        Initialize();
        Open(1, Page::"LT NZ Var Lines");
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A/2 value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A/2 ', Shown);
        Check('log', 'OPEN;U.AGCR();|open;|new;U.AGCR();AGR(A/Resolved/Stamped);I.AGR(A/Resolved/Stamped);U.AGCR(A);U.DONE(2);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordOnPageWithCodeAndFactBoxes_LogsTheTriggers()
    begin
        Initialize();
        Open(2, Page::"LT NZ Var Lines");
        Check('shown', 'name=A//// part=//A/2 value=A///V/ part=//A/2 ', Shown);
        Check('log', 'OPEN;U.AGCR();|open;|new;U.AGCR();AGR(A//);I.AGR(A//);U.AGCR(A);U.DONE(2);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A///V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SaveRecordThenModifyThroughAnotherRecordOnPageWithCodeAndFactBoxes_LogsTheTriggers()
    begin
        Initialize();
        Open(3, Page::"LT NZ Var Lines");
        Check('shown', 'name=A//Stamped// part=/Stamped/A/2 value=A//Stamped/V/ part=/Stamped/A/2 ', Shown);
        Check('log', 'OPEN;U.AGCR();|open;|new;U.AGCR();AGR(A//Stamped);I.AGR(A//Stamped);U.AGCR(A);U.DONE(2);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A//Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_FieldSetBeforeSaveRecordOnPageWithCodeAndFactBoxes_LogsTheTriggers()
    begin
        Initialize();
        Open(4, Page::"LT NZ Var Lines");
        Check('shown', 'name=A/Resolved///Noted part=Resolved//A/2 value=A/Resolved//V/Noted part=Resolved//A/2 ', Shown);
        Check('log', 'OPEN;U.AGCR();|open;|new;U.AGCR();AGR(A/Resolved/);I.AGR(A/Resolved/);U.AGCR(A);U.DONE(2);|name;|read;|value;|read;|ok;', Log());
        Check('rows', 'A/Resolved//V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeAndFactBoxes_RowBefore_LogsTheTriggers()
    var
        Line: Record "LT NZ Line";
    begin
        Initialize();
        Line."Group Code" := 'G1';
        Line.Name := 'B';
        Line.Insert();
        Commit();
        Open(1, Page::"LT NZ Var Lines");
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A/2 value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A/2 ', Shown);
        Check('log', 'OPEN;AGR(B//);I.AGR(B//);U.AGCR(B);U.DONE(0);|open;|new;U.AGCR();AGR(A/Resolved/Stamped);AGR(B//);I.AGR(A/Resolved/Stamped);U.AGCR(A);U.DONE(2);|name;|read;|value;|read;AGR(A/Resolved/Stamped);|ok;', Log());
        Check('rows', 'B///;A/Resolved/Stamped/V;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('TrigHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeAndFactBoxes_TableWithTriggers_LogsThem()
    begin
        Initialize();
        Open(0, Page::"LT NZ Trig Lines");
        Check('shown', 'name=A/Resolved/Stamped/ part=Resolved/Stamped/A value=A/Resolved/Stamped/V part=Resolved/Stamped/A ', Shown);
        Check('log', '|open;|new;T.INS(A);T.MOD(A/Resolved/);|name;|value;T.MOD(A/Resolved/Stamped);|ok;', Log());
        Check('rows', 'A/Resolved/Stamped/V;', TrigRows());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NZ Lines")
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
    procedure VarHandler(var Lines: TestPage "LT NZ Var Lines")
    var
        LogCu: Codeunit "LT NZ Log";
    begin
        LogCu.Add('|open');
        Lines.New();
        LogCu.Add('|new');
        Lines.Name.SetValue('A');
        LogCu.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + '/' + Lines.Usage.Usage.Value() + ' ';
        LogCu.Add('|read');
        Lines.Value.SetValue('V');
        LogCu.Add('|value');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + '/' + Lines.Usage.Usage.Value() + ' ';
        LogCu.Add('|read');
        Lines.OK().Invoke();
        LogCu.Add('|ok');
    end;

    [ModalPageHandler]
    procedure TrigHandler(var Lines: TestPage "LT NZ Trig Lines")
    var
        LogCu: Codeunit "LT NZ Log";
    begin
        LogCu.Add('|open');
        Lines.New();
        LogCu.Add('|new');
        Lines.Name.SetValue('A');
        LogCu.Add('|name');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        LogCu.Add('|value');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.OK().Invoke();
        LogCu.Add('|ok');
    end;

    local procedure Initialize()
    var
        Line: Record "LT NZ Line";
        TrigLine: Record "LT NZ Trig Line";
        Detail: Record "LT NZ Detail";
        LogCu: Codeunit "LT NZ Log";
    begin
        Line.DeleteAll();
        TrigLine.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
    end;

    local procedure Open(M: Integer; PageId: Integer)
    var
        LogCu: Codeunit "LT NZ Log";
        Opener: Codeunit "LT NZ Opener";
    begin
        LogCu.Reset(M);
        Opener.Open('G1', PageId);
    end;

    local procedure Log(): Text
    var
        LogCu: Codeunit "LT NZ Log";
    begin
        exit(LogCu.Get());
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

    local procedure TrigRows() Result: Text
    var
        Line: Record "LT NZ Trig Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Line: Record "LT NZ Line";
        Detail: Record "LT NZ Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.SetRange(Name, 'A');
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
