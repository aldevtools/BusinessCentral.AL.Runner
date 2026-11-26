// Neighbors of ListNewLineExtraCodeCodedSecondSetValue.al (al-runner issue 8843): the later SetValue's control inserts another
// row of the page's table before its save (on a copy of the probe's page), the landing control's SetValue again, and two
// SetValues with code (on the probe's variation page, mode 1: the save and the update). Each test holds Business Central's
// answer, measured on a local BC 29.1 container (3 runs of 3).

// The probe's page, whose Value control inserts a row of another group before its save.
page 96576 "LT NV4 Lines"
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
                    begin
                        Rec.ApplyValue();
                        Other.Init();
                        Other."Group Code" := 'G2';
                        Other.Name := 'Other';
                        Other.Insert();
                        CurrPage.SaveRecord();
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

codeunit 96577 "LT NV4 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NV3 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NV4 Lines", Line);
    end;
}

codeunit 96578 "LT NV4 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('InsertHandler')]
    procedure NewLine_SecondSetValueInsertsAnotherRowOfThePageTable_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NV4 Opener";
    begin
        Initialize(0);
        Opener.Open('G1');
        Check('shown', 'value=A/Resolved/Stamped/V/Value V ', Shown);
        Check('rows', 'G1:A/Resolved/Stamped/V/Value V;G2:Other////;', Rows());
        Check('stamps', ' details=2', Stamps());
    end;

    [Test]
    [HandlerFunctions('NameTwiceHandler')]
    procedure NewLine_SecondSetValueOnTheLandingControl_RunsItsCodeAgain()
    var
        Opener: Codeunit "LT NV3 Var Opener";
    begin
        Initialize(1);
        Opener.Open('G1', false);
        Check('shown', 'name=B/Resolved/Stamped// ', Shown);
        Check('rows', 'G1:B/Resolved/Stamped//;', Rows());
        Check('stamps', ' details=4', Stamps());
    end;

    [Test]
    [HandlerFunctions('ValueTwiceHandler')]
    procedure NewLine_TwoSetValuesWithCode_ShowAndSaveTheRow()
    var
        Opener: Codeunit "LT NV3 Var Opener";
    begin
        Initialize(1);
        Opener.Open('G1', false);
        Check('shown', 'value=A/Resolved/Stamped/W/ usage=2 ', Shown);
        Check('rows', 'G1:A/Resolved/Stamped/W/;', Rows());
        Check('stamps', ' details=2', Stamps());
    end;

    [ModalPageHandler]
    procedure InsertHandler(var Lines: TestPage "LT NV4 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure NameTwiceHandler(var Lines: TestPage "LT NV3 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Name.SetValue('B');
        Shown += 'name=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure ValueTwiceHandler(var Lines: TestPage "LT NV3 Var Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Lines.Value.SetValue('W');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' usage=' + Lines.Usage.Usage.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize(M: Integer)
    var
        Line: Record "LT NV3 Line";
        Detail: Record "LT NV3 Detail";
        Stamp: Record "LT NV3 Stamp";
        Mode: Codeunit "LT NV3 Mode";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Stamp.DeleteAll();
        Mode.Set(M);
        Commit();
        Shown := '';
    end;

    local procedure Rows() Result: Text
    var
        Line: Record "LT NV3 Line";
    begin
        if Line.FindSet() then
            repeat
                Result += Line."Group Code" + ':' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + ';';
            until Line.Next() = 0;
    end;

    local procedure Stamps() Result: Text
    var
        Stamp: Record "LT NV3 Stamp";
        Detail: Record "LT NV3 Detail";
    begin
        if Stamp.FindSet() then
            repeat
                Result += Stamp.Text + ';';
            until Stamp.Next() = 0;
        Result += ' details=' + Format(Detail.Count());
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
