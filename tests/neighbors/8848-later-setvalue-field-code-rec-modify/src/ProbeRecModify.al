// The corpus probe ListNewLineExtraCodeCodedRecModify.al of corpus pull request 1542 (al-runner issue 8849), whole.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-modify-method
// Scope: in-scope
// local object: a List page with page code and three CardPart FactBoxes whose third enqueues a page background task, whose
//   table field's OnValidate inserts the page's new line, and a later SetValue whose control saves, then modifies the row
//   through Rec before it updates, are what is under test
// Fixtures used: own tables 96720 to 96722, pages 96725 to 96727, 96729 and 96732, codeunits 96723, 96724, 96728, 96730, 96731,
//   96733 and 96734, test codeunit 96735
// Probe (al-runner issue 8849): the page of ListNewLineExtraCodeCodedSecondSetValue.al's repro; after the new line lands, the
//   SetValue on Value runs the control's code: a table procedure that sets a field, CurrPage.SaveRecord(), Rec read again, a
//   codeunit that writes another table, a codeunit given Rec that sets Status and modifies it, a procedure of a FactBox and
//   CurrPage.Update(false). A variation page whose Value control saves, then does what the mode asks: Rec.Modify() after
//   setting a field, another record's write then Rec read again, a field of Rec set after its Modify(), a copy of Rec modified,
//   Rec modified then another record's write; then updates. The page is opened with Page.RunModal on a record filtered to one
//   empty group. Readings: what the page and its parts show after each SetValue, the rows after OK, and the other table.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96720 "LT NY6 Line"
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
        field(7; Shown; Text[30]) { }
        field(8; Status; Text[10]) { }
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
        Detail: Record "LT NY6 Detail";
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

table 96721 "LT NY6 Detail"
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

table 96722 "LT NY6 Stamp"
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

// Marks a line as applied, through the record it is given.
codeunit 96723 "LT NY6 Applier"
{
    procedure Apply(var Line: Record "LT NY6 Line")
    begin
        Line.Status := 'Applied';
        Line.Modify();
    end;
}

// Writes a stamp row for the line into another table.
codeunit 96724 "LT NY6 Stamper"
{
    procedure Stamp(Line: Record "LT NY6 Line")
    var
        Stamp: Record "LT NY6 Stamp";
    begin
        Stamp.Init();
        Stamp."Line Entry No." := Line."Entry No.";
        Stamp.Text := Line.Shown;
        Stamp.Insert();
    end;
}

// FactBox 1: shows the line's fields and a variable its OnAfterGetRecord sets.
page 96725 "LT NY6 Info"
{
    PageType = CardPart;
    SourceTable = "LT NY6 Line";

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
page 96726 "LT NY6 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NY6 Line";

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
page 96727 "LT NY6 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NY6 Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NY6 Usage Task", Parameters);
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

codeunit 96728 "LT NY6 Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NY6 Detail";
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
// control's OnValidate sets a field through a table procedure, saves the row, reads it again, writes another table, passes Rec to a
// codeunit that sets Status and modifies it, calls a procedure of a FactBox and updates the page.
page 96729 "LT NY6 Lines"
{
    PageType = List;
    SourceTable = "LT NY6 Line";
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
                        Stamper: Codeunit "LT NY6 Stamper";
                        Applier: Codeunit "LT NY6 Applier";
                    begin
                        Rec.ApplyValue();
                        CurrPage.SaveRecord();
                        Rec.GetBySystemId(Rec.SystemId);
                        Stamper.Stamp(Rec);
                        Applier.Apply(Rec);
                        CurrPage.Usage.Page.ClearUsage();
                        CurrPage.Update(false);
                    end;
                }
                field(Shown; Rec.Shown) { ApplicationArea = All; }
                field(Status; Rec.Status) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NY6 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NY6 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NY6 Usage")
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

    local procedure StampMark(Line: Record "LT NY6 Line")
    var
        Other: Record "LT NY6 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96730 "LT NY6 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NY6 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NY6 Lines", Line);
    end;
}

codeunit 96731 "LT NY6 Mode"
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

// The variation page: the repro's page whose Value control saves, then does what the mode asks, then updates.
page 96732 "LT NY6 Var Lines"
{
    PageType = List;
    SourceTable = "LT NY6 Line";
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
                        Applier: Codeunit "LT NY6 Applier";
                        CopyApplier: Codeunit "LT NY6 Copy Applier";
                        Mode: Codeunit "LT NY6 Mode";
                    begin
                        Rec.ApplyValue();
                        CurrPage.SaveRecord();
                        case Mode.Get() of
                            1:
                                begin
                                    Rec.Status := 'Applied';
                                    Rec.Modify();
                                end;
                            2:
                                begin
                                    MarkLate(Rec);
                                    Rec.Get(Rec."Group Code", Rec."Entry No.");
                                end;
                            3:
                                begin
                                    Rec.Status := 'Applied';
                                    Rec.Modify();
                                    Rec.Note := 'Late';
                                end;
                            4:
                                CopyApplier.Apply(Rec);
                            5:
                                begin
                                    Applier.Apply(Rec);
                                    MarkLate(Rec);
                                end;
                        end;
                        CurrPage.Update(false);
                    end;
                }
                field(Shown; Rec.Shown) { ApplicationArea = All; }
                field(Status; Rec.Status) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NY6 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NY6 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NY6 Usage")
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

    local procedure MarkLate(Line: Record "LT NY6 Line")
    var
        Other: Record "LT NY6 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Late';
        Other.Modify();
    end;

    local procedure StampMark(Line: Record "LT NY6 Line")
    var
        Other: Record "LT NY6 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96733 "LT NY6 Var Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NY6 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NY6 Var Lines", Line);
    end;
}

// Marks a line as applied through a copy of the record it is given: the row changes, the caller's record does not.
codeunit 96734 "LT NY6 Copy Applier"
{
    procedure Apply(Line: Record "LT NY6 Line")
    begin
        Line.Status := 'Applied';
        Line.Modify();
    end;
}

codeunit 96735 "LT NY6 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_SecondSetValueCodeunitModifiesRecAfterTheSave_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NY6 Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Applied/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V/Applied;', Rows());
        Check('stamps', 'Value V;', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueModifiesRecAfterTheSave_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(1);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Applied/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V/Applied;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueReadsRecAgainAfterAnotherRecordsWrite_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(2);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Late/V/Value V//Noted part=Resolved/Late/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Late/V/Value V/;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueChangesRecAfterItsModify_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(3);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Applied/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V/Applied;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueCopyOfRecModifiesTheRow_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(4);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Applied/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V/Applied;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_LaterSetValueModifiesRecThenAnotherRecordWrites_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(5);
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Late/V/Value V/Applied/Noted part=Resolved/Late/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Late/V/Value V/Applied;', Rows());
        Check('stamps', '', Stamps());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NY6 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Status.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure VarHandler(var Lines: TestPage "LT NY6 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Status.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NY6 Line";
        Detail: Record "LT NY6 Detail";
        Stamp: Record "LT NY6 Stamp";
        Mode: Codeunit "LT NY6 Mode";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Stamp.DeleteAll();
        Mode.Set(0);
        Commit();
        Shown := '';
    end;

    local procedure RunVar(M: Integer)
    var
        Mode: Codeunit "LT NY6 Mode";
        Opener: Codeunit "LT NY6 Var Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1');
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NY6 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + '/' + Line.Status + ';';
            until Line.Next() = 0;
    end;

    local procedure Stamps() Result: Text
    var
        Stamp: Record "LT NY6 Stamp";
    begin
        if Stamp.FindSet() then
            repeat
                Result += Stamp.Text + ';';
            until Stamp.Next() = 0;
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
