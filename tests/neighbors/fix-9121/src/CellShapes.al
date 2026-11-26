// Stand-ins written from the facts of the two measured pages (no Microsoft text): an Enum key field with one filter
// (page 7002), two key filters with the key's first field (Enum) unchanged and an Option key field changed (page 7004),
// and their neighbors.
enum 97500 "N9121 Kind"
{
    Extensible = false;
    value(0; A) { }
    value(1; B) { }
    value(2; C) { }
    value(3; D) { }
}

enum 97501 "N9121 Line Type"
{
    Extensible = false;
    value(0; Item) { }
    value(1; Group) { }
    value(2; None) { }
}

table 97502 "N9121 Enum Keyed"
{
    fields
    {
        field(1; "Item No."; Code[20]) { }
        field(2; Kind; Enum "N9121 Kind") { }
        field(3; "Entry No."; Integer) { }
    }
    keys { key(PK; "Item No.", Kind, "Entry No.") { } }
}

// As in the measured page's table, the key's first field comes after the other key fields in field order.
table 97503 "N9121 Two Keyed"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(13; "Sales Type"; Option) { OptionMembers = A,B,C,D; }
        field(21; Type; Enum "N9121 Line Type") { }
    }
    keys { key(PK; Type, "Code", "Sales Type") { } }
}

// Table 97503 with an Option `Type`: both key filters on Option fields.
table 97506 "N9121 Option Two Keyed"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(13; "Sales Type"; Option) { OptionMembers = A,B,C,D; }
        field(21; Type; Option) { OptionMembers = Item,Group,None; }
    }
    keys { key(PK; Type, "Code", "Sales Type") { } }
}

table 97504 "N9121 Enum Two Keyed"
{
    fields
    {
        field(1; Type; Option) { OptionMembers = Item,Group,None; }
        field(2; "Code"; Code[20]) { }
        field(3; "Sales Type"; Enum "N9121 Kind") { }
    }
    keys { key(PK; Type, "Code", "Sales Type") { } }
}

table 97505 "N9121 Log"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
    }
    keys { key(PK; "Entry No.") { } }
}

page 97504 "N9121 Enum Cell"
{
    PageType = List;
    SourceTable = "N9121 Enum Keyed";
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
                    SetRecFilters();
                end;
            }
            field(TwoFilters; TwoFilters)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    Rec.SetRange("Item No.", 'X');
                    SetRecFilters();
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
        SetRecFilters();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Editable := Rec.Kind <> Rec.Kind::C;
    end;

    local procedure SetRecFilters()
    begin
        Rec.SetRange(Kind, KindFilter);
        CurrPage.Update(false);
    end;

    var
        KindFilter: Enum "N9121 Kind";
        TwoFilters: Boolean;
        Editable: Boolean;
}

page 97505 "N9121 Two Cell"
{
    PageType = List;
    SourceTable = "N9121 Two Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(SalesTypeFilter; SalesTypeFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    SetRecFilters();
                end;
            }
            field(BothFilter; BothFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    TypeFilter := TypeFilter::Group;
                    SalesTypeFilter := SalesTypeFilter::C;
                    SetRecFilters();
                end;
            }
            field(AssignFilter; AssignFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    Rec."Code" := 'X';
                    SalesTypeFilter := SalesTypeFilter::C;
                    SetRecFilters();
                end;
            }
            repeater(Lines)
            {
                field(Type; Rec.Type) { }
                field("Code"; Rec."Code") { }
                field("Sales Type"; Rec."Sales Type") { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        SetRecFilters();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        SetEditableFields();
    end;

    local procedure SetRecFilters()
    begin
        Rec.SetRange("Sales Type", SalesTypeFilter);
        Rec.SetRange(Type, TypeFilter);
        CurrPage.Update(false);
    end;

    local procedure SetEditableFields()
    begin
        CodeEditable := Rec."Sales Type" <> Rec."Sales Type"::C;
    end;

    var
        SalesTypeFilter: Option A,B,C,D;
        TypeFilter: Enum "N9121 Line Type";
        BothFilter: Boolean;
        AssignFilter: Boolean;
        CodeEditable: Boolean;
}

page 97506 "N9121 Enum Two Cell"
{
    PageType = List;
    SourceTable = "N9121 Enum Two Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(SalesTypeFilter; SalesTypeFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    SetRecFilters();
                end;
            }
            repeater(Lines)
            {
                field(Type; Rec.Type) { }
                field("Sales Type"; Rec."Sales Type") { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        SetRecFilters();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CodeEditable := Rec."Sales Type" <> Rec."Sales Type"::C;
    end;

    local procedure SetRecFilters()
    begin
        Rec.SetRange("Sales Type", SalesTypeFilter);
        Rec.SetRange(Type, Rec.Type::Item);
        CurrPage.Update(false);
    end;

    var
        SalesTypeFilter: Enum "N9121 Kind";
        CodeEditable: Boolean;
}

// The page 7004 stand-in whose OnAfterGetCurrRecord inserts a log row.
page 97507 "N9121 Two Cell Log"
{
    PageType = List;
    SourceTable = "N9121 Two Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(SalesTypeFilter; SalesTypeFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    SetRecFilters();
                end;
            }
            field(Runs; Runs) { Editable = false; }
            repeater(Lines)
            {
                field("Sales Type"; Rec."Sales Type") { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        SetRecFilters();
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Record "N9121 Log";
    begin
        Log."Entry No." := Log.Count() + 1;
        Log.Insert();
    end;

    local procedure SetRecFilters()
    begin
        Rec.SetRange("Sales Type", SalesTypeFilter);
        Rec.SetRange(Type, Rec.Type::Item);
        CurrPage.Update(false);
    end;

    var
        SalesTypeFilter: Option A,B,C,D;
        Runs: Integer;
}

// The page 7004 stand-in whose OnAfterGetCurrRecord counts its runs in a page variable.
page 97508 "N9121 Two Cell Counts"
{
    PageType = List;
    SourceTable = "N9121 Two Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(SalesTypeFilter; SalesTypeFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    SetRecFilters();
                end;
            }
            field(Runs; Runs) { Editable = false; }
            repeater(Lines)
            {
                field("Sales Type"; Rec."Sales Type") { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        SetRecFilters();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Runs += 1;
    end;

    local procedure SetRecFilters()
    begin
        Rec.SetRange("Sales Type", SalesTypeFilter);
        Rec.SetRange(Type, Rec.Type::Item);
        CurrPage.Update(false);
    end;

    var
        SalesTypeFilter: Option A,B,C,D;
        Runs: Integer;
}

page 97509 "N9121 Option Two Cell"
{
    PageType = List;
    SourceTable = "N9121 Option Two Keyed";
    DelayedInsert = true;
    layout
    {
        area(Content)
        {
            field(SalesTypeFilter; SalesTypeFilter)
            {
                trigger OnValidate()
                begin
                    CurrPage.SaveRecord();
                    SetRecFilters();
                end;
            }
            repeater(Lines)
            {
                field(Type; Rec.Type) { }
                field("Sales Type"; Rec."Sales Type") { }
            }
        }
    }
    trigger OnOpenPage()
    begin
        SetRecFilters();
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CodeEditable := Rec."Sales Type" <> Rec."Sales Type"::C;
    end;

    local procedure SetRecFilters()
    begin
        Rec.SetRange("Sales Type", SalesTypeFilter);
        Rec.SetRange(Type, Rec.Type::Item);
        CurrPage.Update(false);
    end;

    var
        SalesTypeFilter: Option A,B,C,D;
        CodeEditable: Boolean;
}
