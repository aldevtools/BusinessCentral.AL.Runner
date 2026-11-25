table 97200 "Repro Order Header"
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

table 97201 "Repro Order Line"
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

codeunit 97202 "Repro Read Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Clear()
    begin
        Entries := '';
    end;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Joined(): Text
    begin
        exit(Entries);
    end;
}

page 97200 "Standalone Order Card"
{
    PageType = Document;
    SourceTable = "Repro Order Header";

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Standalone Order Subform")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 97201 "Standalone Order Subform"
{
    PageType = ListPart;
    SourceTable = "Repro Order Line";
    AutoSplitKey = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.")
                {
                    ApplicationArea = All;
                    Editable = false;
                }
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
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

page 97202 "Repro Order Card"
{
    PageType = Document;
    SourceTable = "Repro Order Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Repro Order Subform")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 97203 "Repro Order Subform"
{
    PageType = ListPart;
    SourceTable = "Repro Order Line";
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
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

page 97204 "Other Trigger Card"
{
    PageType = Document;
    SourceTable = "Repro Order Header";

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Other Trigger Subform")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 97205 "Other Trigger Subform"
{
    PageType = ListPart;
    SourceTable = "Repro Order Line";
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
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('X' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

page 97206 "PG R7820 Doc"
{
    PageType = Document;
    SourceTable = "Repro Order Header";
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

page 97207 "PG R7820 Part"
{
    PageType = ListPart;
    SourceTable = "Repro Order Line";
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
    var
        Log: Codeunit "Repro Read Log";
    begin
        Log.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "Repro Read Log";
    begin
        Log.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "Repro Read Log";
    begin
        Log.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

page 97208 "Mixed Properties Card"
{
    PageType = Document;
    SourceTable = "Repro Order Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
            part(Lines; "Mixed Properties Subform")
            {
                ApplicationArea = All;
                SubPageLink = "Order No." = field("No.");
            }
        }
    }
}

page 97209 "Mixed Properties Subform"
{
    PageType = ListPart;
    SourceTable = "Repro Order Line";
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
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('F' + Which);
        exit(Rec.Find(Which));
    end;

    trigger OnAfterGetRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('R' + Format(Rec."Line No." div 10000));
    end;

    trigger OnAfterGetCurrRecord()
    var
        ReadLog: Codeunit "Repro Read Log";
    begin
        ReadLog.Add('C' + Format(Rec."Line No." div 10000));
    end;
}

codeunit 97210 "Neighbor 7820 Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Standalone_OneRow_IsAssumed()
    begin
        CheckStandalone(1, 'F=>;R1;F=;R1;C1;');
    end;

    [Test]
    procedure Standalone_FiveRows_IsAssumed()
    begin
        CheckStandalone(5, 'F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;');
    end;

    [Test]
    procedure OriginalWithPageProperties_OneRow()
    begin
        CheckOriginal(1, 'F=>;R1;F=;R1;C1;');
    end;

    [Test]
    procedure OriginalWithPageProperties_FiveRows()
    begin
        CheckOriginal(5, 'F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;');
    end;

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
    procedure ThreeRowsFirst_KeepsMainValues()
    begin
        CheckStandaloneValues(3);
    end;

    [Test]
    procedure Standalone_TwoRows_KeepsMainValues()
    begin
        CheckStandaloneValues(2);
    end;

    [Test]
    procedure MixedProperties_KeepsMainValues()
    var
        Header: Record "Repro Order Header";
        OrderCard: TestPage "Mixed Properties Card";
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

    [Test]
    procedure OtherTriggerBody_KeepsMainOpenRead()
    var
        Header: Record "Repro Order Header";
        ReadLog: Codeunit "Repro Read Log";
        OrderCard: TestPage "Other Trigger Card";
    begin
        SeedRows(1);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        ReadLog.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        AssertLog('X=><;R1;X=;R1;C1;');
        OrderCard.Close();
    end;

    [Test]
    procedure LaterCursor_KeepsMainValue()
    var
        Header: Record "Repro Order Header";
        ReadLog: Codeunit "Repro Read Log";
        OrderCard: TestPage "Standalone Order Card";
    begin
        SeedRows(5);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        OrderCard.Lines.Next();
        ReadLog.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure CheckStandalone(Rows: Integer; Expected: Text)
    var
        Header: Record "Repro Order Header";
        ReadLog: Codeunit "Repro Read Log";
        OrderCard: TestPage "Standalone Order Card";
    begin
        SeedRows(Rows);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        ReadLog.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        AssertLog(Expected);
        OrderCard.Lines."Line No.".AssertEquals(10000);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure CheckOriginal(Rows: Integer; Expected: Text)
    var
        Header: Record "Repro Order Header";
        ReadLog: Codeunit "Repro Read Log";
        OrderCard: TestPage "Repro Order Card";
    begin
        SeedRows(Rows);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        ReadLog.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        AssertLog(Expected);
        OrderCard.Lines."Line No.".AssertEquals(10000);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure CheckMergedProbe(Rows: Integer; Moves: Integer; Expected: Text; ExpectedLine: Integer)
    var
        Header: Record "Repro Order Header";
        ReadLog: Codeunit "Repro Read Log";
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
        ReadLog.Clear();
        OrderCard.Lines.Qty.SetValue(100);
        AssertLog(Expected);
        OrderCard.Lines."Line No.".AssertEquals(ExpectedLine);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure CheckStandaloneValues(Rows: Integer)
    var
        Header: Record "Repro Order Header";
        OrderCard: TestPage "Standalone Order Card";
    begin
        SeedRows(Rows);
        Header.FindFirst();
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        OrderCard.Lines.Qty.SetValue(100);
        OrderCard.Lines."Line No.".AssertEquals(10000);
        OrderCard.Lines.Qty.AssertEquals(100);
        OrderCard.Close();
    end;

    local procedure SeedRows(Rows: Integer)
    var
        Header: Record "Repro Order Header";
        Line: Record "Repro Order Line";
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
        ReadLog: Codeunit "Repro Read Log";
    begin
        if ReadLog.Joined() <> Expected then
            Error('The part read %1, expected %2.', ReadLog.Joined(), Expected);
    end;
}

codeunit 97211 "Neighbor 7820 Refusal Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Standalone_SecondSetValue_IsRefused()
    var
        Header: Record "Repro Order Header";
        Line: Record "Repro Order Line";
        OrderCard: TestPage "Standalone Order Card";
        i: Integer;
    begin
        Header.DeleteAll();
        Line.DeleteAll();
        Header."No." := 'O1';
        Header.Insert();
        for i := 1 to 5 do begin
            Line."Order No." := 'O1';
            Line."Line No." := i * 10000;
            Line.Qty := i;
            Line.Insert();
        end;
        OrderCard.OpenEdit();
        OrderCard.GoToRecord(Header);
        OrderCard.Lines.First();
        OrderCard.Lines.Qty.SetValue(100);
        OrderCard.Lines.Qty.SetValue(101);
    end;
}
