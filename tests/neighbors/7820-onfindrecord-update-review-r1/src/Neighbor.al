table 97300 "R7820 Review Header"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 97301 "R7820 Review Line"
{
    fields
    {
        field(1; "Order No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Qty; Integer) { }
    }
    keys
    {
        key(PK; "Order No.", "Line No.") { Clustered = true; }
    }
}

codeunit 97302 "R7820 Review Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Clear()
    begin
        Entries := '';
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;
}

page 97300 "PG R7820 Doc"
{
    PageType = Document;
    SourceTable = "R7820 Review Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "PG R7820 Part")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 97301 "PG R7820 Part"
{
    PageType = ListPart;
    SourceTable = "R7820 Review Line";
    ApplicationArea = All;
    AutoSplitKey = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; Editable = false; }
                field(Qty; Rec.Qty)
                {
                    ApplicationArea = All;
                    trigger OnValidate()
                    begin
                        CurrPage.Update(true);
                    end;
                }
            }
        }
    }

    trigger OnFindRecord(Which: Text): Boolean
    var Log: Codeunit "R7820 Review Log";
    begin
        Log.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var Log: Codeunit "R7820 Review Log";
    begin
        Log.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var Log: Codeunit "R7820 Review Log";
    begin
        Log.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

page 97302 "Mixed Properties Doc"
{
    PageType = Document;
    SourceTable = "R7820 Review Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Mixed Properties Part")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 97303 "Mixed Properties Part"
{
    PageType = ListPart;
    SourceTable = "R7820 Review Line";
    AutoSplitKey = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; Editable = false; }
                field(Qty; Rec.Qty)
                {
                    ApplicationArea = All;
                    trigger OnValidate()
                    begin
                        CurrPage.Update(true);
                    end;
                }
            }
        }
    }

    trigger OnFindRecord(Which: Text): Boolean
    var Log: Codeunit "R7820 Review Log";
    begin
        Log.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var Log: Codeunit "R7820 Review Log";
    begin
        Log.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var Log: Codeunit "R7820 Review Log";
    begin
        Log.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

codeunit 97310 "Neighbor 7820 Review R1 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure MergedProbe_OneRow_IsAssumed()
    begin
        CheckMergedProbe(1, 0, 'F=>;R1;F=;R1;C1;', 10000);
    end;

    [Test]
    procedure MergedProbe_TwoRows_IsAssumed()
    begin
        CheckMergedProbe(2, 0, 'F=>;R1;R2;R1;F=;R1;C1;', 10000);
    end;

    [Test]
    procedure MergedProbe_FiveRows_IsAssumed()
    begin
        CheckMergedProbe(5, 0, 'F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;', 10000);
    end;

    [Test]
    procedure MergedProbe_ThreeRows_Second_IsAssumed()
    begin
        CheckMergedProbe(3, 1, 'F=>;R2;R3;R2;R1;R2;F=;R2;C2;', 20000);
    end;

    [Test]
    procedure MixedProperties_KeepMainValues()
    var
        Header: Record "R7820 Review Header";
        OrderCard: TestPage "Mixed Properties Doc";
    begin
        SeedRows(1);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        OrderCard.Lines.Qty.SetValue(100);
        OrderCard.Lines."Line No.".AssertEquals(10000);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure CheckMergedProbe(Rows: Integer; Moves: Integer; Expected: Text; ExpectedLine: Integer)
    var
        Header: Record "R7820 Review Header";
        Log: Codeunit "R7820 Review Log";
        OrderCard: TestPage "PG R7820 Doc";
        i: Integer;
    begin
        SeedRows(Rows);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        for i := 1 to Moves do
            OrderCard.Lines.Next();
        Log.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        AssertLog(Expected);
        OrderCard.Lines."Line No.".AssertEquals(ExpectedLine);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure SeedRows(Rows: Integer)
    var
        Header: Record "R7820 Review Header";
        Line: Record "R7820 Review Line";
        i: Integer;
    begin
        Header.DeleteAll();
        Line.DeleteAll();
        Header."No." := 'O1';
        Header.Insert();
        for i := 1 to Rows do begin
            Line."Order No." := 'O1';
            Line."Line No." := i * 10000;
            Line.Qty := i;
            Line.Insert();
        end;
    end;

    local procedure AssertLog(Expected: Text)
    var
        Log: Codeunit "R7820 Review Log";
    begin
        if Log.Get() <> Expected then
            Error('The part read %1, expected %2.', Log.Get(), Expected);
    end;
}
