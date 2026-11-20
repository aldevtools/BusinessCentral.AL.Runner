// al-runner issue 8186: the reporter's repro (green on real BC 29.1, 2/2), app and test app in one file.
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

page 50100 "Repro Lines"
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
                        CurrPage.Update();
                    end;
                }
                field(Value; Rec.Value) { }
            }
        }
    }
}

codeunit 50101 "Repro Line Opener"
{
    procedure Open(GroupCode: Code[10])
    var
        ReproLine: Record "Repro Line";
    begin
        ReproLine.SetRange("Group Code", GroupCode);
        Page.RunModal(Page::"Repro Lines", ReproLine);
    end;
}

codeunit 50120 "Repro Update New Line Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        NameShownAfterUpdate: Text;

    [Test]
    [HandlerFunctions('EnterLineHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure NewLineInsertedByFieldValidate_EmptyView()
    var
        ReproLine: Record "Repro Line";
        Opener: Codeunit "Repro Line Opener";
    begin
        ReproLine.DeleteAll();
        Commit();

        Opener.Open('G1');

        ReproLine.SetRange("Group Code", 'G1');
        if ReproLine.Count() <> 1 then
            Error('Expected 1 row in G1, found %1.', ReproLine.Count());
        ReproLine.FindFirst();
        if ReproLine.Name <> 'X' then
            Error('Expected Name X, found %1.', ReproLine.Name);
        if ReproLine.Value <> 'V' then
            Error('Expected Value V, found %1.', ReproLine.Value);
        if NameShownAfterUpdate <> 'X' then
            Error('Expected the page to show Name X after CurrPage.Update, found %1.', NameShownAfterUpdate);
    end;

    [Test]
    [HandlerFunctions('EnterLineHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure NewLineInsertedByFieldValidate_ViewHasARow()
    var
        ReproLine: Record "Repro Line";
        Opener: Codeunit "Repro Line Opener";
    begin
        ReproLine.DeleteAll();
        ReproLine.Init();
        ReproLine."Group Code" := 'G2';
        ReproLine.Name := 'A';
        ReproLine.Insert(true);
        Commit();

        Opener.Open('G2');

        ReproLine.Reset();
        ReproLine.SetRange("Group Code", 'G2');
        if ReproLine.Count() <> 2 then
            Error('Expected 2 rows in G2, found %1.', ReproLine.Count());
        ReproLine.SetRange(Name, 'X');
        if not ReproLine.FindFirst() then
            Error('Expected a row with Name X in G2.');
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
    procedure EnterLineHandler(var ReproLines: TestPage "Repro Lines")
    begin
        ReproLines.New();
        ReproLines.Name.SetValue('X');
        NameShownAfterUpdate := ReproLines.Name.Value();
        ReproLines.Value.SetValue('V');
        ReproLines.OK().Invoke();
    end;
}
