// al-runner issue 8285: the reporter's repro (green on real BC 29.1, 3/3), app and test app in one file.
table 50100 "Repro Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Group Code"; Code[10]) { }
        field(2; "Entry No."; Integer)
        {
            AutoIncrement = true;
        }
        field(3; Name; Text[30])
        {
            trigger OnValidate()
            begin
                if "Entry No." = 0 then
                    Insert(true);
            end;
        }
        field(4; Value; Text[30]) { }
    }

    keys
    {
        key(PK; "Group Code", "Entry No.") { Clustered = true; }
    }
}

page 50100 "Repro Line FactBox"
{
    PageType = CardPart;
    SourceTable = "Repro Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { }
        }
    }
}

page 50101 "Repro Lines With FactBox"
{
    PageType = List;
    SourceTable = "Repro Line";
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
                    trigger OnValidate()
                    begin
                        CurrPage.SaveRecord();
                        CurrPage.Update(false);
                    end;
                }
                field(Value; Rec.Value) { }
            }
        }
        area(FactBoxes)
        {
            part(LineFactBox; "Repro Line FactBox")
            {
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        RowsRead += 1;
    end;

    var
        RowsRead: Integer;
}

page 50102 "Repro Lines Save Update"
{
    PageType = List;
    SourceTable = "Repro Line";
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
                    trigger OnValidate()
                    begin
                        CurrPage.SaveRecord();
                        CurrPage.Update(false);
                    end;
                }
                field(Value; Rec.Value) { }
            }
        }
    }
}

page 50103 "Repro Lines Update True"
{
    PageType = List;
    SourceTable = "Repro Line";
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
                    trigger OnValidate()
                    begin
                        CurrPage.Update(true);
                    end;
                }
                field(Value; Rec.Value) { }
            }
        }
    }
}

codeunit 50101 "Repro Line Opener"
{
    procedure Open(PageId: Integer; GroupCode: Code[10])
    var
        ReproLine: Record "Repro Line";
    begin
        ReproLine.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, ReproLine);
    end;
}

codeunit 50120 "Repro New Line Update Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        NameShownAfterUpdate: Text;

    [Test]
    [HandlerFunctions('WithFactBoxHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure SaveRecordThenUpdateFalse_ListWithFactBox()
    begin
        RunWithOneRow(Page::"Repro Lines With FactBox", 'G1');
    end;

    [Test]
    [HandlerFunctions('SaveUpdateHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure SaveRecordThenUpdateFalse_ListWithoutParts()
    begin
        RunWithOneRow(Page::"Repro Lines Save Update", 'G2');
    end;

    [Test]
    [HandlerFunctions('UpdateTrueHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure UpdateTrue_ListWithoutParts()
    begin
        RunWithOneRow(Page::"Repro Lines Update True", 'G3');
    end;

    local procedure RunWithOneRow(PageId: Integer; GroupCode: Code[10])
    var
        ReproLine: Record "Repro Line";
        Opener: Codeunit "Repro Line Opener";
    begin
        ReproLine.DeleteAll();
        ReproLine.Init();
        ReproLine."Group Code" := GroupCode;
        ReproLine.Name := 'A';
        ReproLine.Insert(true);
        Commit();
        NameShownAfterUpdate := '';

        Opener.Open(PageId, GroupCode);

        ReproLine.Reset();
        ReproLine.SetRange("Group Code", GroupCode);
        if ReproLine.Count() <> 2 then
            Error('Expected 2 rows in %1, found %2.', GroupCode, ReproLine.Count());
        ReproLine.SetRange(Name, 'X');
        if not ReproLine.FindFirst() then
            Error('Expected a row with Name X in %1.', GroupCode);
        if ReproLine.Value <> 'V' then
            Error('Expected Value V on the X row, found %1.', ReproLine.Value);
        ReproLine.SetRange(Name, 'A');
        ReproLine.FindFirst();
        if ReproLine.Value <> '' then
            Error('Expected the A row to keep an empty Value, found %1.', ReproLine.Value);
        if NameShownAfterUpdate <> 'X' then
            Error('Expected the page to show Name X after CurrPage.Update, found %1.', NameShownAfterUpdate);
    end;

    [ModalPageHandler]
    procedure WithFactBoxHandler(var Lines: TestPage "Repro Lines With FactBox")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        NameShownAfterUpdate := Lines.Name.Value();
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure SaveUpdateHandler(var Lines: TestPage "Repro Lines Save Update")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        NameShownAfterUpdate := Lines.Name.Value();
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure UpdateTrueHandler(var Lines: TestPage "Repro Lines Update True")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        NameShownAfterUpdate := Lines.Name.Value();
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;
}
