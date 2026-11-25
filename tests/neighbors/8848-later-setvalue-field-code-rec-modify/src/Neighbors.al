// Neighbors of ListNewLineExtraCodeCodedFieldOnValidate.al and ListNewLineExtraCodeCodedRecModify.al (al-runner issues 8848 and
// 8849): a later SetValue on a control without code over the field with its own OnValidate, and a second CurrPage.SaveRecord()
// after the codeunit modified Rec, each on a copy of its probe's page over its table and parts. Each test holds Business
// Central's answer, measured on a local BC 29.1 container (3 runs of 3).

// The page of the field probe, whose Value control has no code of its own.
page 96736 "LT NT7 Lines"
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
                field(Value; Rec.Value) { ApplicationArea = All; }
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

// The page of the Rec probe, whose Value control saves a second time after the codeunit modified Rec.
page 96737 "LT NY7 Lines"
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
                        CurrPage.SaveRecord();
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

codeunit 96738 "LT NT7 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NT6 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NT7 Lines", Line);
    end;
}

codeunit 96739 "LT NY7 Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        Line: Record "LT NY6 Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"LT NY7 Lines", Line);
    end;
}

codeunit 96740 "LT NT7 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Shown: Text;

    [Test]
    [HandlerFunctions('FieldHandler')]
    procedure NewLine_LaterSetValueWithoutControlCodeOnAFieldWithItsOwnOnValidate_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NT7 Opener";
        Line: Record "LT NT6 Line";
        Log: Record "LT NT6 Log";
    begin
        Initialize();
        Opener.Open('G1');
        if Line.FindFirst() then
            Shown += 'rows=' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + '/' + Line.Changed;
        if Log.FindFirst() then
            Shown += ' log=' + Log.Text;
        Check('all', 'value=A/Resolved/Stamped/V/ rows=A/Resolved/Stamped/V//Yes log=>V', Shown);
    end;

    [Test]
    [HandlerFunctions('RecHandler')]
    procedure NewLine_LaterSetValueSavesAgainAfterTheCodeunitModifiedRec_ShowsAndSavesTheRow()
    var
        Opener: Codeunit "LT NY7 Opener";
        Line: Record "LT NY6 Line";
    begin
        Initialize();
        Opener.Open('G1');
        if Line.FindFirst() then
            Shown += 'rows=' + Line.Name + '/' + Line.Note + '/' + Line.Mark + '/' + Line.Value + '/' + Line.Shown + '/' + Line.Status;
        Check('all', 'value=A/Resolved/Stamped/V/Value V/Applied rows=A/Resolved/Stamped/V/Value V/Applied', Shown);
    end;

    [ModalPageHandler]
    procedure FieldHandler(var Lines: TestPage "LT NT7 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + ' ';
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure RecHandler(var Lines: TestPage "LT NY7 Lines")
    begin
        Lines.New();
        Lines.Name.SetValue('A');
        Lines.Value.SetValue('V');
        Shown += 'value=' + Lines.Name.Value() + '/' + Lines.Note.Value() + '/' + Lines.Mark.Value() + '/' + Lines.Value.Value() + '/' + Lines.Shown.Value() + '/' + Lines.Status.Value() + ' ';
        Lines.OK().Invoke();
    end;

    local procedure Initialize()
    var
        Line: Record "LT NT6 Line";
        Detail: Record "LT NT6 Detail";
        Stamp: Record "LT NT6 Stamp";
        Log: Record "LT NT6 Log";
        Line2: Record "LT NY6 Line";
        Detail2: Record "LT NY6 Detail";
        Stamp2: Record "LT NY6 Stamp";
    begin
        Line.DeleteAll();
        Detail.DeleteAll();
        Stamp.DeleteAll();
        Log.DeleteAll();
        Line2.DeleteAll();
        Detail2.DeleteAll();
        Stamp2.DeleteAll();
        Commit();
        Shown := '';
    end;

    local procedure Check(What: Text; Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected <%2>, found <%3>.', What, Expected, Actual);
    end;
}
