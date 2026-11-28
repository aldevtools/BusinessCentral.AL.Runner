// Neighbors of al-runner issue 9336 (TestPage SetValue on a control with a TableRelation; corpus PR 1646 measures the
// served cells). Invented objects only.
table 70930 "NB TSV Book"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Active; Boolean) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 70931 "NB TSV Pair"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Group"; Code[10]) { }
        field(2; "Code"; Code[10]) { }
    }
    keys { key(PK; "Group", "Code") { Clustered = true; } }
}

table 70932 "NB TSV Num"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "No"; Integer) { }
    }
    keys { key(PK; "No") { Clustered = true; } }
}

table 70939 "NB TSV Named"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Name; Text[30]) { }
    }
    keys { key(PK; Name) { Clustered = true; } }
}

table 70933 "NB TSV Capt"
{
    Caption = 'Captioned Book';
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Code"; Code[10]) { Caption = 'Book No.'; }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

table 70934 "NB TSV Host"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Book Code"; Code[10])
        {
            TableRelation = "NB TSV Book";
            trigger OnValidate()
            begin
                ValidateCount += 1;
            end;
        }
        field(3; "Active Code"; Code[10]) { TableRelation = "NB TSV Book" where(Active = const(true)); }
        field(4; ValidateCount; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 70930 "NB TSV Text Var"
{
    PageType = Card;
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(BookCtl; BookCode)
            {
                ApplicationArea = All;
                TableRelation = "NB TSV Book";
                trigger OnValidate()
                begin
                    Validated += 1;
                end;
            }
            field(ValidatedCtl; Validated) { ApplicationArea = All; Editable = false; }
        }
    }
    var
        BookCode: Text[10];
        Validated: Integer;
}

page 70931 "NB TSV Code Var"
{
    PageType = Card;
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(BookCtl; BookCode) { ApplicationArea = All; TableRelation = "NB TSV Book"; }
            field(PairCtl; PairCode) { ApplicationArea = All; TableRelation = "NB TSV Pair"; }
            field(NumCtl; NumCode) { ApplicationArea = All; TableRelation = "NB TSV Num"; }
            field(CaptCtl; CaptCode) { ApplicationArea = All; TableRelation = "NB TSV Capt"; }
            field(ShortCtl; ShortCode) { ApplicationArea = All; TableRelation = "NB TSV Book"; }
            field(NamedCtl; NamedCode) { ApplicationArea = All; TableRelation = "NB TSV Named"; }
            field(FiltCtl; FiltCode) { ApplicationArea = All; TableRelation = "NB TSV Book" where(Active = const(true)); }
        }
    }
    var
        BookCode: Code[10];
        PairCode: Code[10];
        NumCode: Code[10];
        CaptCode: Code[10];
        ShortCode: Code[3];
        FiltCode: Code[10];
        NamedCode: Code[10];
}

pageextension 70932 "NB TSV Code Var Ext" extends "NB TSV Code Var"
{
    layout
    {
        addlast(Content)
        {
            field(ExtCtl; ExtCode) { ApplicationArea = All; TableRelation = "NB TSV Book"; }
        }
    }
    var
        ExtCode: Code[10];
}

page 70933 "NB TSV Filtered"
{
    PageType = Card;
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(BookCtl; BookCode) { ApplicationArea = All; TableRelation = "NB TSV Book" where(Active = const(true)); }
        }
    }
    var
        BookCode: Text[10];
}

page 70934 "NB TSV Name Var"
{
    PageType = Card;
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(BookCtl; BookName) { ApplicationArea = All; TableRelation = "NB TSV Book".Name; }
        }
    }
    var
        BookName: Text[30];
}

page 70935 "NB TSV Host Card"
{
    PageType = Card;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "NB TSV Host";
    layout
    {
        area(Content)
        {
            field(BookCtl; Rec."Book Code") { ApplicationArea = All; }
            field(ActiveCtl; Rec."Active Code") { ApplicationArea = All; }
        }
    }
}

page 70936 "NB TSV Host List"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "NB TSV Host";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(BookCtl; Rec."Book Code") { ApplicationArea = All; }
            }
        }
    }
}

report 70938 "NB TSV Request"
{
    ProcessingOnly = true;
    UsageCategory = None;
    dataset
    {
        dataitem(Int; Integer)
        {
            DataItemTableView = where(Number = const(1));
        }
    }
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(BookCtl; BookCode) { ApplicationArea = All; TableRelation = "NB TSV Book"; }
                field(PairCtl; PairCode) { ApplicationArea = All; TableRelation = "NB TSV Pair"; }
            }
        }
    }
    var
        BookCode: Code[10];
        PairCode: Code[10];
}
