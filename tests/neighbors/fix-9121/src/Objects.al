// The issue's repro (a field outside the primary key) and the same shapes on a field in the key.
table 97500 "N9121 Line"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Kind; Option) { OptionMembers = A,B,C,D; }
    }
    keys { key(PK; "Entry No.") { } }
}

table 97501 "N9121 Keyed"
{
    fields
    {
        field(1; "Item No."; Code[20]) { }
        field(2; Kind; Option) { OptionMembers = A,B,C,D; }
        field(3; "Entry No."; Integer) { }
    }
    keys { key(PK; "Item No.", Kind, "Entry No.") { } }
}

page 97500 "N9121 Lines"
{
    PageType = List;
    SourceTable = "N9121 Line";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(KindFilter; KindFilter)
            {
                trigger OnValidate()
                begin
                    Rec.SetRange(Kind, KindFilter);
                    CurrPage.Update(false);
                end;
            }
            repeater(Lines)
            {
                field("Entry No."; Rec."Entry No.") { }
                field(Kind; Rec.Kind) { }
            }
        }
    }
    var
        KindFilter: Option A,B,C,D;
}

// The shape of the Microsoft pages: OnOpenPage filters the key field, the filter control saves, filters and updates.
page 97501 "N9121 Keyed Save"
{
    PageType = List;
    SourceTable = "N9121 Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(KindFilter; KindFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    Rec.SetRange(Kind, KindFilter);
                    CurrPage.Update(false);
                end;
            }
            repeater(Lines)
            {
                field("Item No."; Rec."Item No.") { }
                field(Kind; Rec.Kind) { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.SetRange(Kind, KindFilter);
    end;

    var
        KindFilter: Option A,B,C,D;
}

// No filter at the open, no SaveRecord in the OnValidate.
page 97502 "N9121 Keyed Plain"
{
    PageType = List;
    SourceTable = "N9121 Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(KindFilter; KindFilter)
            {
                trigger OnValidate()
                begin
                    Rec.SetRange(Kind, KindFilter);
                    CurrPage.Update(false);
                end;
            }
            field(RangeFilter; RangeFilter)
            {
                trigger OnValidate()
                begin
                    Rec.SetFilter(Kind, '%1..%2', Rec.Kind::B, Rec.Kind::C);
                    CurrPage.Update(false);
                end;
            }
            field(TwoValues; TwoValues)
            {
                trigger OnValidate()
                begin
                    Rec.SetFilter(Kind, '%1|%2', Rec.Kind::B, Rec.Kind::C);
                    CurrPage.Update(false);
                end;
            }
            group(Hidden)
            {
                Visible = false;
                field(FilterText; ReadFilters()) { Editable = false; }
            }
            repeater(Lines)
            {
                field("Item No."; Rec."Item No.") { }
                field(Kind; Rec.Kind) { }
            }
        }
    }
    local procedure ReadFilters(): Text
    begin
        if Rec.GetFilter(Kind) <> '' then
            Evaluate(KindFilter, Rec.GetFilter(Kind));
        exit(Rec.GetFilter(Kind));
    end;

    var
        KindFilter: Option A,B,C,D;
        RangeFilter: Boolean;
        TwoValues: Boolean;
}

// The same as "N9121 Keyed Save" with OnAfterGetCurrRecord declared: every fact of the Microsoft pages' cell.
page 97503 "N9121 Keyed Cell"
{
    PageType = List;
    SourceTable = "N9121 Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(KindFilter; KindFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    Rec.SetRange(Kind, KindFilter);
                    CurrPage.Update(false);
                end;
            }
            repeater(Lines)
            {
                field("Item No."; Rec."Item No.") { }
                field(Kind; Rec.Kind) { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.SetRange(Kind, KindFilter);
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Seen := Rec.Kind;
    end;

    var
        KindFilter: Option A,B,C,D;
        Seen: Option A,B,C,D;
}
