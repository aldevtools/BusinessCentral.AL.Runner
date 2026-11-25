// The corpus probe ListNewLineExtraCodeCodedSecondSetValue.al of corpus pull request 1541 (al-runner issue 8843), whole.
// Scope: in-scope
// local object: a List page with page code and three CardPart FactBoxes whose third enqueues a page background task, whose
//   table field's OnValidate inserts the page's new line, and the code around CurrPage.SaveRecord() and CurrPage.Update in
//   the controls' OnValidate, on the new line and on the row it became, are what is under test
// Fixtures used: own tables 96560 to 96562 and 96570, pages 96564 to 96566, 96568, 96572 and 96574, codeunits 96563, 96567,
//   96569, 96571 and 96573, test codeunit 96575
// Probe (al-runner issue 8843): the page of ListNewLineExtraCodeCodedPage.al's repro; the SetValue on Name runs the table
//   field's OnValidate, which inserts the row (Insert(true), an AutoIncrement key field) and two rows of another table, then
//   the control's OnValidate, which sets another field of Rec, saves, modifies the row through another record and calls
//   CurrPage.Update(false). After it lands, a SetValue on Value runs the control's own code: a table procedure that sets a
//   field, CurrPage.SaveRecord(), Rec read again, a codeunit that writes another table, a procedure of a FactBox and
//   CurrPage.Update(false). Variations of that code: only the save and the update, another table written before the save, the
//   row modified through another record after it, a filter set before the update, Update(true) after the save or without it,
//   no update, another table written after the update, Rec.Modify() before the update; and the Value control with a
//   TableRelation. The page is opened with Page.RunModal on a record filtered to one empty group. Readings: what the page and
//   its parts show after each SetValue, the rows after OK, and the rows of the other table.
//   Measured on a local BC 29.1 OnPrem w1 container.
// BC versions: 27, 28, 29
table 96560 "LT NV3 Line"
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
        Detail: Record "LT NV3 Detail";
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

table 96561 "LT NV3 Detail"
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

table 96562 "LT NV3 Stamp"
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

// Writes a stamp row for the line into another table.
codeunit 96563 "LT NV3 Stamper"
{
    procedure Stamp(Line: Record "LT NV3 Line")
    var
        Stamp: Record "LT NV3 Stamp";
    begin
        Stamp.Init();
        Stamp."Line Entry No." := Line."Entry No.";
        Stamp.Text := Line.Shown;
        Stamp.Insert();
    end;
}

// FactBox 1: shows the line's fields and a variable its OnAfterGetRecord sets.
page 96564 "LT NV3 Info"
{
    PageType = CardPart;
    SourceTable = "LT NV3 Line";

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
page 96565 "LT NV3 Picture"
{
    PageType = CardPart;
    SourceTable = "LT NV3 Line";

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
page 96566 "LT NV3 Usage"
{
    PageType = CardPart;
    SourceTable = "LT NV3 Line";

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
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"LT NV3 Usage Task", Parameters);
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

codeunit 96567 "LT NV3 Usage Task"
{
    trigger OnRun()
    var
        Detail: Record "LT NV3 Detail";
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
// procedure of a FactBox and updates the page.
page 96568 "LT NV3 Lines"
{
    PageType = List;
    SourceTable = "LT NV3 Line";
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
                        Stamper: Codeunit "LT NV3 Stamper";
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
            part(Info; "LT NV3 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NV3 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NV3 Usage")
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

    local procedure StampMark(Line: Record "LT NV3 Line")
    var
        Other: Record "LT NV3 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96569 "LT NV3 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NV3 Lines", Line);
    end;
}

table 96570 "LT NV3 Choice"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; Code; Text[30]) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 96571 "LT NV3 Mode"
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

// The variation page: the repro's page whose Value control runs the code of the mode (LT NV3 Mode).
page 96572 "LT NV3 Var Lines"
{
    PageType = List;
    SourceTable = "LT NV3 Line";
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
                        Stamper: Codeunit "LT NV3 Stamper";
                        Mode: Codeunit "LT NV3 Mode";
                    begin
                        case Mode.Get() of
                            1:
                                begin
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            2:
                                begin
                                    Rec.ApplyValue();
                                    Stamper.Stamp(Rec);
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            3:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    MarkLate(Rec);
                                    CurrPage.Update(false);
                                end;
                            4:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    Rec.SetRange(Name, 'A');
                                    CurrPage.Update(false);
                                end;
                            5:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(true);
                                end;
                            6:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.Update(true);
                                end;
                            7:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                end;
                            8:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                    Stamper.Stamp(Rec);
                                end;
                            9:
                                begin
                                    Rec.ApplyValue();
                                    Rec.Modify();
                                    CurrPage.Update(false);
                                end;
                        end;
                    end;
                }
                field(Shown; Rec.Shown) { ApplicationArea = All; }
                field(Kind; Kind) { ApplicationArea = All; Editable = false; }
            }
        }
        area(FactBoxes)
        {
            part(Info; "LT NV3 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NV3 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NV3 Usage")
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

    local procedure MarkLate(Line: Record "LT NV3 Line")
    var
        Other: Record "LT NV3 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Late';
        Other.Modify();
    end;

    local procedure StampMark(Line: Record "LT NV3 Line")
    var
        Other: Record "LT NV3 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96573 "LT NV3 Var Opener"
{
    procedure Open(GroupCode: Code[10]; Rel: Boolean)
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        if Rel then
            Page.RunModal(Page::"LT NV3 Rel Lines", Line)
        else
            Page.RunModal(Page::"LT NV3 Var Lines", Line);
    end;
}

// The repro's page whose Value control declares a TableRelation to the choices.
page 96574 "LT NV3 Rel Lines"
{
    PageType = List;
    SourceTable = "LT NV3 Line";
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
                    TableRelation = "LT NV3 Choice".Code;

                    trigger OnValidate()
                    var
                        Stamper: Codeunit "LT NV3 Stamper";
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
            part(Info; "LT NV3 Info")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Picture; "LT NV3 Picture")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
            part(Usage; "LT NV3 Usage")
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

    local procedure StampMark(Line: Record "LT NV3 Line")
    var
        Other: Record "LT NV3 Line";
    begin
        Other.Get(Line."Group Code", Line."Entry No.");
        Other.Mark := 'Stamped';
        Other.Modify();
    end;

    var
        Opened: Boolean;
        Kind: Text;
}

codeunit 96575 "LT NV3 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure NewLine_SecondSetValueWithCodeOnPageWithCodeAndFactBoxes_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NV3 Opener";
    begin
        Initialize();
        Opener.Open('G1');
        Check('shown', 'name=A/Resolved/Stamped//Noted part=Resolved/Stamped/A value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('details', 'A1;A2;', Details());
        Check('stamps', 'Value V;', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueSavesAndUpdates_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(1, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V//Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueWritesAnotherTableBeforeTheSave_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(2, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', 'Value V;', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueModifiesTheRowThroughAnotherRecordAfterTheSave_ShowsTheOtherWrite()
    begin
        Initialize();
        RunVar(3, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Late/V/Value V/Noted part=Resolved/Late/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Late/V/Value V;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueSetsAFilterBeforeTheUpdate_ShowsTheRow()
    begin
        Initialize();
        RunVar(4, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueSavesThenUpdateTrue_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(5, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueUpdateTrueWithoutSave_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(6, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueSavesWithoutUpdate_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(7, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueWritesAnotherTableAfterTheUpdate_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(8, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', 'Value V;', Stamps());
    end;

    [Test]
    [HandlerFunctions('VarHandler')]
    procedure NewLine_SecondSetValueModifiesRecThenUpdates_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(9, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', '', Stamps());
    end;

    [Test]
    [HandlerFunctions('RelHandler')]
    procedure NewLine_SecondSetValueOnAControlWithATableRelation_ShowsAndSavesTheRow()
    begin
        Initialize();
        RunVar(0, true);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V/Value V/Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/Value V;', Rows());
        Check('stamps', 'Value V;', Stamps());
    end;

    // The test writes the row between the two SetValues (review of al-runner PR 8847, 01:00Z): the page's save writes its Rec over it.
    [Test]
    [HandlerFunctions('OutsideHandler')]
    procedure NewLine_SecondSetValueAfterTheTestWroteTheRow_SavesThePageRecord()
    begin
        Initialize();
        RunVar(1, false);
        Check('shown', 'name=A/Resolved/Stamped//Noted value=A/Resolved/Stamped/V//Noted part=Resolved/Stamped/A usage=2 ', Shown);
        Check('rows', 'A/Resolved/Stamped/V/;', Rows());
        Check('stamps', '', Stamps());
    end;

    [ModalPageHandler]
    procedure LinesHandler(var Lines: TestPage "LT NV3 Lines")
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
    procedure VarHandler(var Lines: TestPage "LT NV3 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure RelHandler(var Lines: TestPage "LT NV3 Rel Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value() + ' ';
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure OutsideHandler(var Lines: TestPage "LT NV3 Var Lines")
    var
        Line: Record "LT NV3 Line";
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Kind.Value() + ' ';
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Line.Mark := 'Outside';
        Line.Modify();
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Kind.Value();
        Shown += ' part=' + Lines.Info.Note.Value() + '/' + Lines.Info.Mark.Value() + '/' + Lines.Info.Info.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NV3 Line";
        Detail: Record "LT NV3 Detail";
        Stamp: Record "LT NV3 Stamp";
        Choice: Record "LT NV3 Choice";
        Mode: Codeunit "LT NV3 Mode";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Stamp.DeleteAll();
        Choice.DeleteAll();
        Choice.Code := 'V';
        Choice.Insert();
        Mode.Set(0);
        Commit();
        Shown := '';
    end;

    local procedure RunVar(M: Integer; Rel: Boolean)
    var
        Mode: Codeunit "LT NV3 Mode";
        Opener: Codeunit "LT NV3 Var Opener";
    begin
        Mode.Set(M);
        Opener.Open('G1', Rel);
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + ';';
            until Line.Next() = 0;
    end;

    local procedure Details() Result: Text
    var
        Line: Record "LT NV3 Line";
        Detail: Record "LT NV3 Detail";
    begin
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Detail.SetRange("Line Entry No.", Line."Entry No.");
        if Detail.FindSet() then
            repeat
                Result += Detail.Text + ';';
            until Detail.Next() = 0;
    end;

    local procedure Stamps() Result: Text
    var
        Stamp: Record "LT NV3 Stamp";
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
