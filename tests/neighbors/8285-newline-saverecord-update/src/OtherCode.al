// Code the probe does not run (review of al-runner PR 8305, round 3): copies of the probe's pages and table that differ from
// them in one body. BC has not been measured with any of them, so each is refused at its first SaveRecord or Update(true)
// (code_inserted_pinned.rs). The expected rows are what the code would leave if it ran; no test reaches them.
// SameCode_Quiet is the control: a copy of page 97473 under another name, whose code is the probe's line for line.

// Page 97472 whose OnModifyRecord returns false.
page 97487 "LT NS Lines Exit False"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        Log.Add('P.V(' + Log.Bool(Rec."Entry No." <> 0) + ')');
                        if Log.IsSaveThenUpdateFalse() then begin
                            CurrPage.SaveRecord();
                            Log.Add('P.SR');
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
                        Log.Add('P.V.end');
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('P.NEW');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('P.INS(' + Rec.Name + ')');
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('P.MOD(' + Rec.Name + '/' + Rec.Value + ')');
        exit(false);
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NS Log";
}

// Page 97472 whose OnAfterGetRecord assigns a field.
page 97488 "LT NS Lines Assigns"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        Log.Add('P.V(' + Log.Bool(Rec."Entry No." <> 0) + ')');
                        if Log.IsSaveThenUpdateFalse() then begin
                            CurrPage.SaveRecord();
                            Log.Add('P.SR');
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
                        Log.Add('P.V.end');
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('P.NEW');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('P.INS(' + Rec.Name + ')');
        exit(true);
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('P.MOD(' + Rec.Name + '/' + Rec.Value + ')');
        exit(true);
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + ')');
        Rec.Value := 'Z';
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NS Log";
}

// Page 97473 with other code before the SaveRecord in the control's OnValidate.
page 97489 "LT NS Quiet Code Before"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        if Log.IsSaveThenUpdateFalse() then begin
                            Rec.Value := 'P';
                            CurrPage.SaveRecord();
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "LT NS Log";
}

// Page 97474 with code after the Update(false).
page 97490 "LT NS Part Code After"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        CurrPage.SaveRecord();
                        CurrPage.Update(false);
                        Rec.Value := 'H';
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Peek; "LT NS Peek")
            {
                ApplicationArea = All;
                SubPageLink = "Group Code" = field("Group Code"), "Entry No." = field("Entry No.");
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('P.AGR(' + Rec.Name + '/' + Rec.Value + ')');
    end;

    var
        Log: Codeunit "LT NS Log";
}

// Table 97470 whose OnModify writes a field.
table 97491 "LT NS Line Writes"
{
    DataClassification = SystemMetadata;

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
            var
                Log: Codeunit "LT NS Log";
            begin
                Log.Add('T.V(' + Name + ')');
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

    trigger OnInsert()
    var
        Log: Codeunit "LT NS Log";
    begin
        Log.Add('T.I(' + Name + ')');
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT NS Log";
    begin
        Log.Add('T.M(' + Name + '/' + Value + ')');
        Value := Value + '!';
    end;
}

// Page 97473 on the table that writes.
page 97492 "LT NS Quiet Writes"
{
    PageType = List;
    SourceTable = "LT NS Line Writes";
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
                        if Log.IsSaveThenUpdateFalse() then begin
                            CurrPage.SaveRecord();
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "LT NS Log";
}

// Page 97473 under another name, its code unchanged.
page 97493 "LT NS Quiet Copy"
{
    PageType = List;
    SourceTable = "LT NS Line";
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
                        if Log.IsSaveThenUpdateFalse() then begin
                            CurrPage.SaveRecord();
                            CurrPage.Update(false);
                        end else
                            CurrPage.Update(true);
                    end;
                }
                field(Value; Rec.Value) { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "LT NS Log";
}

codeunit 97494 "LT NS Other Code Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT NS Log";

    local procedure Initialize(SaveThenUpdateFalse: Boolean)
    var
        Line: Record "LT NS Line";
        LineWrites: Record "LT NS Line Writes";
    begin
        Line.DeleteAll();
        LineWrites.DeleteAll();
        Commit();
        Log.Reset(SaveThenUpdateFalse);
    end;

    local procedure CheckRows(Expected: Text)
    var
        Line: Record "LT NS Line";
        Result: Text;
    begin
        if Line.FindSet() then
            repeat
                Result += Line.Name + '/' + Line.Value + ',';
            until Line.Next() = 0;
        if Result <> Expected then
            Error('Rows: expected %1, found %2.', Expected, Result);
    end;

    [Test]
    procedure ExitFalse_SaveRecord()
    var
        Lines: TestPage "LT NS Lines Exit False";
    begin
        Initialize(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('X/,');
    end;

    [Test]
    procedure ExitFalse_UpdateTrue()
    var
        Lines: TestPage "LT NS Lines Exit False";
    begin
        Initialize(false);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('X/,');
    end;

    [Test]
    procedure AssignsInAfterGetRecord_SaveRecord()
    var
        Lines: TestPage "LT NS Lines Assigns";
    begin
        Initialize(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('X/V,');
    end;

    [Test]
    procedure CodeBeforeSaveRecord_Quiet()
    var
        Lines: TestPage "LT NS Quiet Code Before";
    begin
        Initialize(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('X/V,');
    end;

    [Test]
    procedure CodeAfterUpdateFalse_PartHost()
    var
        Lines: TestPage "LT NS Part Code After";
    begin
        Initialize(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('X/V,');
    end;

    [Test]
    procedure TableOnModifyWrites_Quiet()
    var
        Lines: TestPage "LT NS Quiet Writes";
        LineWrites: Record "LT NS Line Writes";
    begin
        Initialize(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        LineWrites.FindFirst();
        if LineWrites.Value <> 'V!' then
            Error('Value: expected V!, found %1.', LineWrites.Value);
    end;

    [Test]
    procedure SameCode_Quiet()
    var
        Lines: TestPage "LT NS Quiet Copy";
    begin
        Initialize(true);
        Lines.OpenEdit();
        Lines.New();
        Lines.Name.SetValue('X');
        Lines.Value.SetValue('V');
        Lines.Close();
        CheckRows('X/V,');
    end;
}
