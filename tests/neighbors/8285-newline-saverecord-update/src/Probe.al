// The objects of the corpus probe ListNewLineSaveRecordUpdate.al (al-runner issue 8285), without its test codeunit 97477:
// the neighbors in the other files use its table, log, pages and part. Its header follows.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-saverecord-method
// Scope: in-scope
// local object: the table field's OnValidate that inserts the page's new line, and the page triggers that log, are what is under test
// Fixtures used: shared Assert (60021); own table 97470, log codeunit 97471, pages 97472 to 97475, opener codeunit 97476, test
//   codeunit 97477
// Probe (al-runner issue 8285, the follow-up of 8186 measured in ListNewLineCodeInsertUpdate.al): a List page stands on its new
//   line (New()). SetValue on Name runs the table field's OnValidate, which inserts Rec itself (Insert(true), with an
//   AutoIncrement key field), and then the control's OnValidate, which calls CurrPage.SaveRecord() and then
//   CurrPage.Update(false), or CurrPage.Update(true). Readings: the page and table triggers each step runs, the row the page
//   shows after the refresh, the rows the table holds, where a later SetValue lands, and Previous()/Next() from there. Cells: a
//   page with the five row triggers and AutoSplitKey, and a page without page triggers, each opened with OpenEdit on an empty
//   view and on a view with one row before, and with Page.RunModal on a record filtered to one group (a row of another group
//   outside the view); a page with OnAfterGetRecord only and one CardPart FactBox without triggers linked to the line's key
//   (SaveRecord then Update(false) only), opened the same three ways; then Close() or OK() after the Value SetValue (Close() in a
//   modal handler raises on BC: not under RunModal). Measured on a local BC 29.1 OnPrem w1 container; green in CI on 27, 28, 29.
// BC versions: 27, 28, 29

table 97470 "LT NS Line"
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
    end;
}

codeunit 97471 "LT NS Log"
{
    SingleInstance = true;

    var
        Buffer: Text;
        SaveThenUpdateFalse: Boolean;

    procedure Reset(NewSaveThenUpdateFalse: Boolean)
    begin
        Buffer := '';
        SaveThenUpdateFalse := NewSaveThenUpdateFalse;
    end;

    procedure Add(Entry: Text)
    begin
        Buffer += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Buffer;
        Buffer := '';
        exit(Result);
    end;

    procedure IsSaveThenUpdateFalse(): Boolean
    begin
        exit(SaveThenUpdateFalse);
    end;

    procedure Bool(Value: Boolean): Text
    begin
        if Value then
            exit('T');
        exit('F');
    end;
}

// The five row triggers and AutoSplitKey, as page 97032 of ListNewLineCodeInsertUpdate.al.
page 97472 "LT NS Lines"
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
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('P.AGCR(' + Rec.Name + ')');
    end;

    var
        Log: Codeunit "LT NS Log";
}

// No page trigger, as the repro of al-runner issue 8285 and page 97039 of ListNewLineCodeInsertUpdate.al.
page 97473 "LT NS Lines Quiet"
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

// The host's shape in al-runner issue 8285: only OnAfterGetRecord, and one CardPart FactBox without triggers linked to the
// line's key.
page 97474 "LT NS Lines Part"
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

page 97475 "LT NS Peek"
{
    PageType = CardPart;
    SourceTable = "LT NS Line";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            field(Name; Rec.Name) { ApplicationArea = All; }
            field(Value; Rec.Value) { ApplicationArea = All; }
        }
    }
}

codeunit 97476 "LT NS Opener"
{
    procedure Open(PageId: Integer; GroupCode: Code[10])
    var
        Line: Record "LT NS Line";
    begin
        Line.SetRange("Group Code", GroupCode);
        Page.RunModal(PageId, Line);
    end;
}
