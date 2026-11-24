// The corpus probe ListNewLineExtraCodeCodedTableTriggers.al of corpus pull request 1540 (al-runner issue 8835), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/triggers-auto/table/table-oninsert-trigger
// Scope: in-scope
// local object: a List page with page code, a Notes part and three CardPart FactBoxes (one enqueues a page background task)
//   over a table that declares OnInsert and OnDelete, whose table field's OnValidate inserts the page's new line, and the code
//   around CurrPage.SaveRecord() and CurrPage.Update in the control's OnValidate, are what is under test
// Fixtures used: own tables 96502, 96503 and 96504, pages 96510 to 96517, codeunits 96505 to 96509, test codeunit 96518
// Probe (al-runner issue 8835): the page of ListNewLineExtraCodeCodedPage.al's repro, with a Notes system part, over a table
//   whose OnInsert sets a field and whose OnDelete deletes the line's detail rows; the SetValue on Name runs the table field's
//   OnValidate, which inserts the row (Insert(true), an AutoIncrement key field, which runs OnInsert) and two rows of another
//   table, then the control's OnValidate, which sets another field of Rec, saves, modifies the row through another record and
//   calls CurrPage.Update(false). The same page over a second table varies its OnInsert one step at a time: it sets the
//   field, reads the table for a sorting number, writes another table, calls a codeunit that sets the field, or raises an
//   error; its OnDelete writes a log no cell reaches. The page is opened with Page.RunModal on a record filtered to one empty
//   group. Readings: what the page and its first part show after the Name and the Value SetValue, and the rows of both tables
//   after OK.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96502 "LT NE2 Line"
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
        field(7; Created; Text[10]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        Created := 'Yes';
    end;

    trigger OnDelete()
    var
        Detail: Record "LT NE2 Detail";
    begin
        Detail.SetRange("Line Entry No.", "Entry No.");
        Detail.DeleteAll();
    end;

    // Writes the default detail rows of the line into another table.
    local procedure CopyDefaults()
    var
        Detail: Record "LT NE2 Detail";
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

table 96503 "LT NE2 Detail"
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
page 96510 "LT NE2 Info"
{
    PageType = CardPart;
    SourceTable = "LT NE2 Line";

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
page 96511 "LT NE2 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NE2 Line";

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
page 96512 "LT NE2 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NE2 Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NE2 Usage Task", Parameters);
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

codeunit 96506 "LT NE2 Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NE2 Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

// A List page with OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions, a Notes part and three FactBoxes, over a table with
// OnInsert and OnDelete: the Name control's OnValidate
// sets another field of Rec, saves the new line, modifies the row through another record and updates the page.
page 96513 "LT NE2 Lines"
{
    PageType = List;
    SourceTable = "LT NE2 Line";
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
            part(Info; "LT NE2 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NE2 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NE2 Usage")
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

    local procedure StampMark(Line: Record "LT NE2 Line")
    var
        Other: Record "LT NE2 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96507 "LT NE2 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NE2 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NE2 Lines", Line);
    end;
}

// The table whose OnInsert the mode decides: it sets the field (1), reads the table and sets a field from what it found, as a
// sorting number does (2), writes another table (3), calls a codeunit that sets the field (4), or raises an error (5); its
// OnDelete writes the log, which no cell reaches.
table 96504 "LT NE2 Var Line"
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
        field(7; Created; Text[10]) { }
        field(8; Sorting; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT NE2 Log";
        Defaults: Codeunit "LT NE2 Defaults";
        Last: Record "LT NE2 Var Line";
        Detail: Record "LT NE2 Detail";
    begin
        case Log.GetMode() of
            1:
                Created := 'Yes';
            2:
                begin
                    Last.SetRange("Group Code", "Group Code");
                    Last.SetLoadFields(Sorting);
                    if Last.FindLast() then
                        Sorting := Last.Sorting + 10
                    else
                        Sorting := 10;
                end;
            3:
                begin
                    Detail.Init();
                    Detail."Line Entry No." := -1;
                    Detail.Text := 'ByInsert';
                    Detail.Insert();
                end;
            4:
                Defaults.Apply(Rec);
            5:
                Error('Not allowed');
        end;
    end;

    trigger OnDelete()
    var
        Log: Codeunit "LT NE2 Log";
    begin
        Log.Add('T.DEL(' + Name + ')');
    end;

    local procedure CopyDefaults()
    var
        Detail: Record "LT NE2 Detail";
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

codeunit 96505 "LT NE2 Log"
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

codeunit 96509 "LT NE2 Defaults"
{
    procedure Apply(var Line: Record "LT NE2 Var Line")
    begin
        Line.Created := 'Called';
    end;
}

// The variation set: the repro's page, parts and opener over the table whose OnInsert the mode decides.
// FactBox 1: shows the line's fields and a variable its OnAfterGetRecord sets.
page 96514 "LT NE2 Var Info"
{
    PageType = CardPart;
    SourceTable = "LT NE2 Var Line";

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
page 96515 "LT NE2 Var Picture"
{
    PageType = CardPart;
    SourceTable = "LT NE2 Var Line";

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
page 96516 "LT NE2 Var Usage"
{
    PageType = CardPart;
    SourceTable = "LT NE2 Var Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NE2 Usage Task", Parameters);
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


// A List page with OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions, a Notes part and three FactBoxes, over a table with
// OnInsert and OnDelete: the Name control's OnValidate
// sets another field of Rec, saves the new line, modifies the row through another record and updates the page.
page 96517 "LT NE2 Var Lines"
{
    PageType = List;
    SourceTable = "LT NE2 Var Line";
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
            part(Info; "LT NE2 Var Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NE2 Var Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NE2 Var Usage")
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

    local procedure StampMark(Line: Record "LT NE2 Var Line")
    var
        Other: Record "LT NE2 Var Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96508 "LT NE2 Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NE2 Var Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NE2 Var Lines", Line);
    end;
}

codeunit 96518 "LT NE2 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_ExtraCodeOnPageWithCodeOverTableWithTriggers_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NE2 Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Yes;', Rows());
        Check('details', 'A1;A2;', Details());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_OnInsertSetsAField_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(1);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Yes/0;', VarRows());
        Check('details', 'A1;A2;', VarDetails());
        Check('log', '', Log());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_OnInsertReadsTheTableForASortingNumber_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(2);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V//10;', VarRows());
        Check('details', 'A1;A2;', VarDetails());
        Check('log', '', Log());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_OnInsertWritesAnotherTable_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(3);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V//0;', VarRows());
        Check('details', 'A1;A2;', VarDetails());
        Check('written by OnInsert', '1', Format(Written()));
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_OnInsertCallsACodeunit_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(4);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Called/0;', VarRows());
        Check('details', 'A1;A2;', VarDetails());
    end;

    [Test]
    [HandlerFunctions('VarErrorHandler')]
    procedure NewLine_OnInsertRaisesAnError_TheSetValueFails()
    var
        LogCu: Codeunit "LT NE2 Log";
        Opener: Codeunit "LT NE2 Var Opener";
    begin
        Initialize();
        LogCu.Reset(5);
        asserterror Opener.Open('G1');
        Check('error', 'Validation error for Field: Name,  Message = ''Not allowed''', GetLastErrorText());
        Check('rows', '', VarRows());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NE2 Lines")
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
    procedure VarHandler(var Lines: TestPage "LT NE2 Var Lines")
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
    procedure VarErrorHandler(var Lines: TestPage "LT NE2 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NE2 Line";
        VarLine: Record "LT NE2 Var Line";
        Detail: Record "LT NE2 Detail";
        LogCu: Codeunit "LT NE2 Log";
    begin
        Line.DeleteAll();
        VarLine.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(0);
        Commit();
        Shown := '';
    end;

    local procedure RunVar(M: Integer)
    var
        LogCu: Codeunit "LT NE2 Log";
        Opener: Codeunit "LT NE2 Var Opener";
    begin
        LogCu.Reset(M);
        Opener.Open('G1');
    end;

    local procedure Log(): Text
    var
        LogCu: Codeunit "LT NE2 Log";
    begin
        exit(LogCu.Get());
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NE2 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Created + ';';
            until Line.Next() = 0;
    end;

    local procedure VarRows() Result: Text
    var
        Line: Record "LT NE2 Var Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Created + '/' + Format(Line.Sorting) + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Line: Record "LT NE2 Line";
        Detail: Record "LT NE2 Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Detail.SetRange("Line Entry No.", Line."Entry No.");
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure VarDetails() Result: Text
    var
        Line: Record "LT NE2 Var Line";
        Detail: Record "LT NE2 Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Detail.SetRange("Line Entry No.", Line."Entry No.");
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Written(): Integer
    var
        Detail: Record "LT NE2 Detail";
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

// The cells of errors of the first SetValue other than OnInsert's (review of al-runner PR 8840, 01:32Z): a copy of the
// variation table whose Name OnValidate raises before the insert (mode 11, TestField) or after it (mode 12).
table 96430 "LT NE4 Line"
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
                Log: Codeunit "LT NE2 Log";
            begin
                if Log.GetMode() = 11 then
                    TestField(Note);
                if "Entry No." = 0 then
                    Insert(true);
                if Log.GetMode() = 12 then
                    Error('Failed after the insert');
                CopyDefaults();
            end;
        }
        field(4; Note; Text[30]) { }
        field(5; Mark; Text[30]) { }
        field(6; Value; Text[30]) { }
        field(7; Created; Text[10]) { }
        field(8; Sorting; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT NE2 Log";
        Defaults: Codeunit "LT NE2 Defaults";
        Last: Record "LT NE4 Line";
        Detail: Record "LT NE2 Detail";
    begin
        case Log.GetMode() of
            1:
                Created := 'Yes';
            2:
                begin
                    Last.SetRange("Group Code", "Group Code");
                    Last.SetLoadFields(Sorting);
                    if Last.FindLast() then
                        Sorting := Last.Sorting + 10
                    else
                        Sorting := 10;
                end;
            3:
                begin
                    Detail.Init();
                    Detail."Line Entry No." := -1;
                    Detail.Text := 'ByInsert';
                    Detail.Insert();
                end;
            4:
                Created := 'Called';
            5:
                Error('Not allowed');
        end;
    end;

    trigger OnDelete()
    var
        Log: Codeunit "LT NE2 Log";
    begin
        Log.Add('T.DEL(' + Name + ')');
    end;

    local procedure CopyDefaults()
    var
        Detail: Record "LT NE2 Detail";
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

// The variation FactBoxes over that table.
page 96431 "LT NE4 Info"
{
    PageType = CardPart;
    SourceTable = "LT NE4 Line";

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

page 96432 "LT NE4 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NE4 Line";

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

page 96433 "LT NE4 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NE4 Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NE2 Usage Task", Parameters);
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


// A List page with OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions, a Notes part and three FactBoxes, over a table with
// OnInsert and OnDelete: the Name control's OnValidate
// sets another field of Rec, saves the new line, modifies the row through another record and updates the page.

// The variation page over that table; its Name control raises before its save (mode 13).
page 96434 "LT NE4 Lines"
{
    PageType = List;
    SourceTable = "LT NE4 Line";
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
                        Log: Codeunit "LT NE2 Log";
                    begin
                        if Log.GetMode() = 13 then
                            Error('Control failed');
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
            part(Info; "LT NE4 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NE4 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NE4 Usage")
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

    local procedure StampMark(Line: Record "LT NE4 Line")
    var
        Other: Record "LT NE4 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96435 "LT NE4 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NE4 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NE4 Lines", Line);
    end;
}

codeunit 96436 "LT NE4 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('ErrorHandler')]
    procedure NewLine_TableFieldTestFieldFailsBeforeTheInsert_TheSetValueFails()
    begin
        Run(11);
        Check('error', 'Validation error for Field: Name,  Message = ''Note must have a value in LT NE4 Line: Group Code=G1, Entry No.=0. It cannot be zero or empty.''', GetLastErrorText());
        Check('code', 'TestValidation', GetLastErrorCode());
        Check('rows', '', Rows());
    end;

    [Test]
    [HandlerFunctions('ErrorHandler')]
    procedure NewLine_TableFieldFailsAfterTheInsert_TheSetValueFails()
    begin
        Run(12);
        Check('error', 'Validation error for Field: Name,  Message = ''Failed after the insert''', GetLastErrorText());
        Check('code', 'TestValidation', GetLastErrorCode());
        Check('rows', '', Rows());
    end;

    [Test]
    [HandlerFunctions('ErrorHandler')]
    procedure NewLine_ControlFailsBeforeItsSave_TheSetValueFails()
    begin
        Run(13);
        Check('error', 'Validation error for Field: Name,  Message = ''Control failed''', GetLastErrorText());
        Check('code', 'TestValidation', GetLastErrorCode());
        Check('rows', '', Rows());
    end;

    [ModalPageHandler]
    procedure ErrorHandler(var Lines: TestPage "LT NE4 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.OK().Invoke();
    end;

    local procedure Run(M: Integer)
    var
        Line: Record "LT NE4 Line";
        Detail: Record "LT NE2 Detail";
        LogCu: Codeunit "LT NE2 Log";
        Opener: Codeunit "LT NE4 Opener";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        LogCu.Reset(M);
        Commit();
        asserterror Opener.Open('G1');
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NE4 Line";
    begin
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
