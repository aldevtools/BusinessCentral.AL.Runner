// Neighbors of ListNewLineExtraCodeCodedSecondSetValue.al for the reviews of al-runner PR 8847 (01:00Z, 02:55Z): a
// TableRelation on a control without code; the value typed into the Value control with a TableRelation stored, missing, or
// stored in another case, and a relation to a Code key; a field of Rec set, the row modified through another record, or a row
// of another table modified or deleted, between the later save and its Update; a filter set on Rec before the save; Rec moved
// off the landed row before the save; the row written between the two SetValues. Each test is in its own codeunit and holds
// Business Central's answer, measured on a local BC 29.1 container (3 runs of 3).

// The probe's page whose Value control runs the code of the mode (LT NV3 Mode).
page 96580 "LT NV5 Var Lines"
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
                        Other: Record "LT NV3 Line";
                        Stamp: Record "LT NV3 Stamp";
                        Mode: Codeunit "LT NV3 Mode";
                    begin
                        case Mode.Get() of
                            1:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    Rec.Mark := 'Late';
                                    CurrPage.Update(false);
                                end;
                            2:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    MarkLate(Rec);
                                    CurrPage.Update(true);
                                end;
                            3:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    Rec.Mark := 'Late';
                                    CurrPage.Update(true);
                                end;
                            4:
                                begin
                                    Rec.ApplyValue();
                                    Rec.SetRange(Name, 'A');
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            5:
                                begin
                                    Rec.ApplyValue();
                                    Other.SetRange("Group Code", 'G2');
                                    Other.FindFirst();
                                    Rec := Other;
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            6:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    CurrPage.Update(false);
                                end;
                            7:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    Stamp.FindFirst();
                                    Stamp.Delete();
                                    CurrPage.Update(false);
                                end;
                            8:
                                begin
                                    Rec.ApplyValue();
                                    CurrPage.SaveRecord();
                                    Stamp.FindFirst();
                                    Stamp.Text := 'Changed';
                                    Stamp.Modify();
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

// The probe's page whose Value control has a TableRelation to the choices and no code of its own.
page 96581 "LT NV5 Plain Rel Lines"
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

// The probe's Rel page whose Value control relates to a table with a Code key.
page 96593 "LT NV5 Code Rel Lines"
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
                    TableRelation = "LT NV5 Code Choice".Code;

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

table 96594 "LT NV5 Code Choice"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; Code; Code[30]) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

// Opens the pages above, and the probe's Rel page, on group G1; sets up the rows and reads them back.
codeunit 96582 "LT NV5 Lib"
{
    procedure Initialize(M: Integer; OtherGroupRow: Boolean)
    var
        Line: Record "LT NV3 Line";
        Detail: Record "LT NV3 Detail";
        Stamp: Record "LT NV3 Stamp";
        Choice: Record "LT NV3 Choice";
        CodeChoice: Record "LT NV5 Code Choice";
        Mode: Codeunit "LT NV3 Mode";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Stamp.DeleteAll();
        Choice.DeleteAll();
        Choice.Code := 'V';
        Choice.Insert();
        CodeChoice.DeleteAll();
        CodeChoice.Code := 'V';
        CodeChoice.Insert();
        if OtherGroupRow then begin
            Line.Init();
            Line."Group Code" := 'G2';
            Line.Name := 'Other';
            Line.Insert();
        end;
        if M in [7, 8] then begin
            Stamp.Init();
            Stamp.Text := 'Before';
            Stamp.Insert();
        end;
        Mode.Set(M);
        Commit();
    end;

    procedure OpenVar()
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NV5 Var Lines", Line);
    end;

    procedure OpenPlainRel()
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NV5 Plain Rel Lines", Line);
    end;

    procedure OpenRel()
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NV3 Rel Lines", Line);
    end;

    procedure OpenCodeRel()
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", 'G1');
        Page.RunModal(Page::"LT NV5 Code Rel Lines", Line);
    end;

    procedure Rows() Result: Text
    var
        Line: Record "LT NV3 Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + ';';
            until Line.Next() = 0;
    end;

    procedure Stamps() Result: Text
    var
        Stamp: Record "LT NV3 Stamp";
    begin
        if Stamp.FindSet() then
            repeat
                Result += Stamp.Text + ';';
            until Stamp.Next() = 0;
    end;

    procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}

codeunit 96583 "LT NV5 Plain Rel Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueOnAControlWithoutCodeWithATableRelation_ShowsAndSavesTheRow()
    begin
        Lib.Initialize(0, false);
        Shown := '';
        Lib.OpenPlainRel();
        Lib.Check('shown', 'value=A/Resolved/Stamped/V/ ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/V/;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Plain Rel Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96584 "LT NV5 Rel Stored Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueOfAStoredRelatedValue_ShowsAndSavesTheRow()
    begin
        Lib.Initialize(0, false);
        Shown := '';
        Lib.OpenRel();
        Lib.Check('shown', 'value=A/Resolved/Stamped/V/Value V ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/V/Value V;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV3 Rel Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96585 "LT NV5 Rel Missing Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueOfAMissingRelatedValue_SavesItAsTyped()
    begin
        Lib.Initialize(0, false);
        Shown := '';
        Lib.OpenRel();
        Lib.Check('shown', 'value=A/Resolved/Stamped/W/Value W ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/W/Value W;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV3 Rel Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('W');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96586 "LT NV5 Rel Case Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueOfARelatedValueInAnotherCase_SavesItAsTyped()
    begin
        Lib.Initialize(0, false);
        Shown := '';
        Lib.OpenRel();
        Lib.Check('shown', 'value=A/Resolved/Stamped/v/Value v ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/v/Value v;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV3 Rel Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('v');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96587 "LT NV5 Field After Save Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueSetsAFieldOfRecAfterTheSave_ShowsTheRow()
    begin
        Lib.Initialize(1, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('shown', 'value=A/Resolved/Stamped/V/Value V ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/V/Value V;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96588 "LT NV5 Modify Upd True Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueModifiesTheRowThenUpdateTrue_ShowsTheRow()
    begin
        Lib.Initialize(2, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('shown', 'value=A/Resolved/Stamped/V/Value V ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/V/Value V;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96589 "LT NV5 Field Upd True Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueSetsAFieldOfRecThenUpdateTrue_ShowsTheRow()
    begin
        Lib.Initialize(3, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('shown', 'value=A/Resolved/Late/V/Value V ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Late/V/Value V;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96590 "LT NV5 Filter Save Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueSetsAFilterBeforeTheSave_ShowsTheRow()
    begin
        Lib.Initialize(4, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('shown', 'value=A/Resolved/Stamped/V/Value V ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/V/Value V;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96591 "LT NV5 Other Row Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueMovesRecToAnotherRowBeforeTheSave_ShowsTheRow()
    begin
        Lib.Initialize(5, true);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('shown', 'value=A/Resolved/Stamped// ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped//;G2:Other////;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

codeunit 96592 "LT NV5 Between Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_RowWrittenBetweenTheTwoSetValues_ShowsTheRow()
    begin
        Lib.Initialize(6, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('shown', 'value=A/Resolved/Stamped/V/Value V ', Shown);
        Lib.Check('rows', 'G1:A/Resolved/Stamped/V/Value V;', Lib.Rows());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    var
        Line: Record "LT NV3 Line";
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Line.SetRange("Group Code", 'G1');
        Line.FindFirst();
        Line.Mark := 'Outside';
        Line.Modify();
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

// Review of al-runner PR 8847, 02:55Z: a row of another table deleted between the save and the Update.
codeunit 96595 "LT NV5 Delete Other Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueDeletesAnotherTablesRowAfterTheSave_ShowsTheRow()
    begin
        Lib.Initialize(7, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('all', 'value=A/Resolved/Stamped/V/Value V | rows=G1:A/Resolved/Stamped/V/Value V;| stamps=', Shown + '| rows=' + Lib.Rows() + '| stamps=' + Lib.Stamps());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

// Review of al-runner PR 8847, 02:55Z: a row of another table modified between the save and the Update.
codeunit 96596 "LT NV5 Modify Other Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueModifiesAnotherTablesRowAfterTheSave_ShowsTheRow()
    begin
        Lib.Initialize(8, false);
        Shown := '';
        Lib.OpenVar();
        Lib.Check('all', 'value=A/Resolved/Stamped/V/Value V | rows=G1:A/Resolved/Stamped/V/Value V;| stamps=Changed;', Shown + '| rows=' + Lib.Rows() + '| stamps=' + Lib.Stamps());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}

// Review of al-runner PR 8847, 02:55Z: the Value control relates to a table with a Code key.
codeunit 96597 "LT NV5 Code Rel Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "LT NV5 Lib";
        Shown: Text;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NewLine_LaterSetValueIntoARelationWithACodeKey_ShowsAndSavesTheRow()
    begin
        Lib.Initialize(0, false);
        Shown := '';
        Lib.OpenCodeRel();
        Lib.Check('all', 'value=A/Resolved/Stamped/V/Value V | rows=G1:A/Resolved/Stamped/V/Value V;| stamps=Value V;', Shown + '| rows=' + Lib.Rows() + '| stamps=' + Lib.Stamps());
    end;

    [ModalPageHandler]
    procedure Handler(var Lines: TestPage "LT NV5 Code Rel Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;
}
