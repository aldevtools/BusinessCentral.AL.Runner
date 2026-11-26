// The corpus probe ListNewLineExtraCodeCodedFieldOnValidate.al of corpus pull request 1542 (al-runner issue 8848), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/triggers-auto/field/devenv-onvalidate-field-trigger
// Scope: in-scope
// local object: a List page with page code and three CardPart FactBoxes whose third enqueues a page background task, whose
//   table field's OnValidate inserts the page's new line, and a later SetValue on a field with its own OnValidate whose control
//   saves and updates, are what is under test
// Fixtures used: own tables 96696 to 96699 and 96708, pages 96702 to 96704, 96706, 96711 to 96713 and 96715, codeunits 96700,
//   96701, 96705, 96707, 96709, 96710, 96714, 96716 and 96717, test codeunit 96718
// Probe (al-runner issue 8848): the page of ListNewLineExtraCodeCodedSecondSetValue.al's repro, whose Value field has an
//   OnValidate of its own in the table: when the value changes, it sets Changed and calls a codeunit that logs the change into
//   a fourth table. After the new line lands, the SetValue on Value runs that OnValidate, then the control's: a table
//   procedure that sets a field, CurrPage.SaveRecord(), Rec read again, a codeunit that writes another table, a procedure of a
//   FactBox and CurrPage.Update(false). A copy of the objects whose field OnValidate's codeunit does what the mode asks: nothing,
//   insert another row of the page's table, modify the row through another record, or raise an error. The page is opened with
//   Page.RunModal on a record filtered to one empty group. Readings: what the page and its parts show after each SetValue, the
//   rows after OK, and the rows of the other tables.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96696 "LT NT6 Line"
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
        field(6; Value; Text[30])
        {
            trigger OnValidate()
            var
                Logger: Codeunit "LT NT6 Logger";
            begin
                if Value <> xRec.Value then begin
                    Changed := 'Yes';
                    Logger.LogChange("Entry No.", xRec.Value, Value);
                end;
            end;
        }
        field(7; Shown; Text[30]) { }
        field(8; Changed; Text[10]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    // Sets the field that shows the value, as the page's Value control asks.
    procedure ApplyValue()
    begin
        Shown := 'Value ' + Value;
    end;

    // Writes the default detail rows of the line into another table.
    local procedure CopyDefaults()
    var
        Detail: Record "LT NT6 Detail";
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

table 96697 "LT NT6 Detail"
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

table 96698 "LT NT6 Stamp"
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

table 96699 "LT NT6 Log"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "Line Entry No."; Integer) { }
        field(3; Text; Text[80]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

// Logs a change of a line's value into another table.
codeunit 96700 "LT NT6 Logger"
{
    procedure LogChange(LineEntryNo: Integer; OldValue: Text; NewValue: Text)
    var
        Log: Record "LT NT6 Log";
    begin
        Log.Init();
        Log."Line Entry No." := LineEntryNo;
        Log.Text := OldValue + '>' + NewValue;
        Log.Insert();
    end;
}

// Writes a stamp row for the line into another table.
codeunit 96701 "LT NT6 Stamper"
{
    procedure Stamp(Line: Record "LT NT6 Line")
    var
        Stamp: Record "LT NT6 Stamp";
    begin
        Stamp.Init();
        Stamp."Line Entry No." := Line."Entry No.";
        Stamp.Text := Line.Shown;
        Stamp.Insert();
    end;
}

// FactBox 1: shows the line's fields and a variable its OnAfterGetRecord sets.
page 96702 "LT NT6 Info"
{
    PageType = CardPart;
    SourceTable = "LT NT6 Line";

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
page 96703 "LT NT6 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NT6 Line";

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
page 96704 "LT NT6 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NT6 Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NT6 Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    // Clears the count, which the next refresh recomputes.
    procedure ClearUsage()
    begin
        Usage := '';
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

codeunit 96705 "LT NT6 Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NT6 Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

// A List page with OnOpenPage, OnAfterGetRecord and OnDeleteRecord, actions and three FactBoxes: the Name control's OnValidate
// sets another field of Rec, saves the new line, modifies the row through another record and updates the page; the Value
// control's OnValidate sets a field through a table procedure, saves the row, reads it again, writes another table, calls a
// procedure of a FactBox and updates the page. The Value field's own OnValidate sets Changed and logs the change into a
// fourth table.
page 96706 "LT NT6 Lines"
{
    PageType = List;
    SourceTable = "LT NT6 Line";
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
                field(Value; Rec.Value)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Stamper: Codeunit "LT NT6 Stamper";
                    begin
                        Rec.ApplyValue();
                        CurrPage.SaveRecord();
                        Rec.GetBySystemId(Rec.SystemId);
                        Stamper.Stamp(Rec);
                        CurrPage.Usage.Page.ClearUsage();
                        CurrPage.Update(false);
                    end;
                }
                field(Shown; Rec.Shown) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NT6 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NT6 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NT6 Usage")
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

    local procedure StampMark(Line: Record "LT NT6 Line")
    var
        Other: Record "LT NT6 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96707 "LT NT6 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NT6 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NT6 Lines", Line);
    end;
}

// The variation copy of the repro's objects (all but the tables it shares): its Value field's OnValidate calls the variation
// logger, which does what the mode asks.
table 96708 "LT NT6 Var Line"
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
        field(6; Value; Text[30])
        {
            trigger OnValidate()
            var
                Logger: Codeunit "LT NT6 Var Logger";
            begin
                if Value <> xRec.Value then begin
                    Changed := 'Yes';
                    Logger.LogChange("Entry No.", xRec.Value, Value);
                end;
            end;
        }
        field(7; Shown; Text[30]) { }
        field(8; Changed; Text[10]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }

    // Sets the field that shows the value, as the page's Value control asks.
    procedure ApplyValue()
    begin
        Shown := 'Value ' + Value;
    end;

    // Writes the default detail rows of the line into another table.
    local procedure CopyDefaults()
    var
        Detail: Record "LT NT6 Detail";
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

// The variation logger: what the Value field's OnValidate does through it, by the mode of LT NT6 Mode.
codeunit 96709 "LT NT6 Var Logger"
{
    procedure LogChange(LineEntryNo: Integer; OldValue: Text; NewValue: Text)
    var
        Line: Record "LT NT6 Var Line";
        Mode: Codeunit "LT NT6 Mode";
    begin
        case Mode.Get() of
            1:
                exit;
            2:
                begin
                    Line.Init();
                    Line."Group Code" := 'G2';
                    Line.Name := 'Other';
                    Line.Insert();
                end;
            3:
                begin
                    Line.SetRange("Entry No.", LineEntryNo);
                    Line.FindFirst();
                    Line.Mark := 'Field';
                    Line.Modify();
                end;
            4:
                Error('Not allowed');
        end;
    end;
}

codeunit 96710 "LT NT6 Var Stamper"
{
    procedure Stamp(Line: Record "LT NT6 Var Line")
    var
        Stamp: Record "LT NT6 Stamp";
    begin
        Stamp.Init();
        Stamp."Line Entry No." := Line."Entry No.";
        Stamp.Text := Line.Shown;
        Stamp.Insert();
    end;
}

page 96711 "LT NT6 Var Info"
{
    PageType = CardPart;
    SourceTable = "LT NT6 Var Line";

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

page 96712 "LT NT6 Var Picture"
{
    PageType = CardPart;
    SourceTable = "LT NT6 Var Line";

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

page 96713 "LT NT6 Var Usage"
{
    PageType = CardPart;
    SourceTable = "LT NT6 Var Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NT6 Var Usage Task", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        if CompletedTaskId = TaskId then
            Usage := Results.Get('Count');
    end;

    // Clears the count, which the next refresh recomputes.
    procedure ClearUsage()
    begin
        Usage := '';
    end;

    var
        TaskId: Integer;
        Usage: Text;
}

codeunit 96714 "LT NT6 Var Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NT6 Detail";
        EntryNo: Integer;
        Results: Dictionary of [Text, Text];
    begin
        Evaluate(EntryNo, Page.GetBackgroundParameters().Get('Entry'));
        Detail.SetRange("Line Entry No.", EntryNo);
        Results.Add('Count', Format(Detail.Count()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

page 96715 "LT NT6 Var Lines"
{
    PageType = List;
    SourceTable = "LT NT6 Var Line";
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
                field(Value; Rec.Value)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Stamper: Codeunit "LT NT6 Var Stamper";
                    begin
                        Rec.ApplyValue();
                        CurrPage.SaveRecord();
                        Rec.GetBySystemId(Rec.SystemId);
                        Stamper.Stamp(Rec);
                        CurrPage.Usage.Page.ClearUsage();
                        CurrPage.Update(false);
                    end;
                }
                field(Shown; Rec.Shown) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NT6 Var Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NT6 Var Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NT6 Var Usage")
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

    local procedure StampMark(Line: Record "LT NT6 Var Line")
    var
        Other: Record "LT NT6 Var Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96716 "LT NT6 Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NT6 Var Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NT6 Var Lines", Line);
    end;
}

codeunit 96717 "LT NT6 Mode"
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

codeunit 96718 "LT NT6 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_SecondSetValueOnAFieldWithItsOwnOnValidate_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NT6 Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'G1:A/Resolved/Stamped/V/Value V/Yes;', Rows());
        Check('others', 'stamps=Value V; log=>V;', Others());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueFieldOnValidateSetsAFieldOnly_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(1);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'G1:A/Resolved/Stamped/V/Value V/Yes;', VarRows());
        Check('others', 'stamps=Value V; log=', Others());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueFieldOnValidateInsertsARowOfThePageTable_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(2);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'G1:A/Resolved/Stamped/V/Value V/Yes;G2:Other/////;', VarRows());
        Check('others', 'stamps=Value V; log=', Others());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueFieldOnValidateModifiesTheRowThroughAnotherRecord_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(3);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'G1:A/Resolved/Stamped/V/Value V/Yes;', VarRows());
        Check('others', 'stamps=Value V; log=', Others());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueFieldOnValidateRaisesAnError_TheSetValueFails()
    begin
        Initialize();
        asserterror RunVar(4);
        Check('error', 'Validation error for Field: Value,  Message = ''Not allowed''', GetLastErrorText());
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A ', Shown);
        Check('rows', '', VarRows());
        Check('others', 'stamps= log=', Others());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NT6 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure VarHandler(var Lines: TestPage "LT NT6 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NT6 Line";
        VarLine: Record "LT NT6 Var Line";
        Detail: Record "LT NT6 Detail";
        Stamp: Record "LT NT6 Stamp";
        Log: Record "LT NT6 Log";
        Mode: Codeunit "LT NT6 Mode";
    begin
        Line.DeleteAll();
        VarLine.DeleteAll();
        Detail.DeleteAll();
        Stamp.DeleteAll();
        Log.DeleteAll();
        Mode.Set(0);
        Commit();
        Shown := '';
    end;

    local procedure RunVar(M: Integer)
    var
        Mode: Codeunit "LT NT6 Mode";
        Opener: Codeunit "LT NT6 Var Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NT6 Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + '/' + Line.Changed + ';';
            until Line.Next() = 0;
    end;

    local procedure VarRows() Result: Text
    var
        Line: Record "LT NT6 Var Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + '/' + Line.Changed + ';';
            until Line.Next() = 0;
    end;

    local procedure Others() Result: Text
    var
        Stamp: Record "LT NT6 Stamp";
        Log: Record "LT NT6 Log";
    begin
        Result := 'stamps=';
        if Stamp.FindSet() then
            repeat
                Result += Stamp.Text + ';';
            until Stamp.Next() = 0;
        Result += ' log=';
        if Log.FindSet() then
            repeat
                Result += Log.Text + ';';
            until Log.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
