// The reporter's repro of al-runner issue 8285 under other names and ids, line for line (code_inserted_pinned.rs REPRO_*), and
// the opens assumption list-new-line-save-record-repro does not measure: TestPage.OpenEdit, and Page.RunModal on an empty group
// (review of #8305, round 4).
table 97477 "LT NR Line"
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

page 97478 "LT NR Line FactBox"
{
    PageType = CardPart;
    SourceTable = "LT NR Line";
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

page 97496 "LT NR Lines With FactBox"
{
    PageType = List;
    SourceTable = "LT NR Line";
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
            part(LineFactBox; "LT NR Line FactBox")
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

page 97479 "LT NR Lines Save Update"
{
    PageType = List;
    SourceTable = "LT NR Line";
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

page 97495 "LT NR Lines Update True"
{
    PageType = List;
    SourceTable = "LT NR Line";
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

codeunit 97497 "LT NR Opener"
{
    procedure Open(PageId: Integer; GroupCode: Code[10])
    var
        Line: Record "LT NR Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, Line);
    end;
}

codeunit 97498 "LT NR Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    // The measured open: Page.RunModal on one group with one row before the new line.
    [Test]
    [HandlerFunctions('SaveUpdateHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure Repro_RunModal_OneGroup_RowBefore()
    begin
        AddRowA('G1');
        Commit();
        OpenGroup(Page::"LT NR Lines Save Update", 'G1');
        CheckLanded('G1');
    end;

    [Test]
    procedure Repro_OpenEdit_RowBefore_SaveUpdate()
    var
        Lines: TestPage "LT NR Lines Save Update";
    begin
        AddRowA('');
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckLanded('');
    end;

    [Test]
    procedure Repro_OpenEdit_RowBefore_UpdateTrue()
    var
        Lines: TestPage "LT NR Lines Update True";
    begin
        AddRowA('');
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckLanded('');
    end;

    [Test]
    procedure Repro_OpenEdit_Empty_SaveUpdate()
    var
        Line: Record "LT NR Line";
        Lines: TestPage "LT NR Lines Save Update";
    begin
        Line.DeleteAll();
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        Assert.AreEqual(1, Line.Count(), 'rows');
    end;

    [Test]
    [HandlerFunctions('SaveUpdateHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure Repro_RunModal_EmptyGroup_SaveUpdate()
    var
        Line: Record "LT NR Line";
    begin
        Line.DeleteAll();
        Commit();
        OpenGroup(Page::"LT NR Lines Save Update", 'G2');
        Line.SetRange("Group Code", 'G2');
        Assert.AreEqual(1, Line.Count(), 'rows');
    end;

    [Test]
    [HandlerFunctions('WithFactBoxHandler')]
    [TransactionModel(TransactionModel::AutoCommit)]
    procedure Repro_RunModal_EmptyGroup_WithFactBox()
    var
        Line: Record "LT NR Line";
    begin
        Line.DeleteAll();
        Commit();
        OpenGroup(Page::"LT NR Lines With FactBox", 'G3');
        Line.SetRange("Group Code", 'G3');
        Assert.AreEqual(1, Line.Count(), 'rows');
    end;

    local procedure AddRowA(GroupCode: Code[10])
    var
        Line: Record "LT NR Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Group Code" := GroupCode;
        Line.Name := 'A';
        Line.Insert(true);
    end;

    local procedure OpenGroup(PageId: Integer; GroupCode: Code[10])
    var
        Opener: Codeunit "LT NR Opener";
    begin
        Opener.Open(PageId, GroupCode);
    end;

    local procedure CheckLanded(GroupCode: Code[10])
    var
        Line: Record "LT NR Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Assert.AreEqual(2, Line.Count(), 'rows');
        Line.SetRange(Name, 'X');
        Line.FindFirst();
        Assert.AreEqual('V', Line.Value, 'the value on the X row');
    end;

    [ModalPageHandler]
    procedure SaveUpdateHandler(var Lines: TestPage "LT NR Lines Save Update")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;

    [ModalPageHandler]
    procedure WithFactBoxHandler(var Lines: TestPage "LT NR Lines With FactBox")
    begin
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.OK().Invoke();
    end;
}
