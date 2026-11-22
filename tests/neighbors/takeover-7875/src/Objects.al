table 50100 "NB Cnt Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Qty; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50101 "NB Cnt Field Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Count"; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50100 "NB Cnt Count"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count) { }
            }
        }
    }
}

page 50101 "NB Cnt CountApprox"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.CountApprox) { }
            }
        }
    }
}

page 50102 "NB Cnt GetFilters"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Fl; Rec.GetFilters) { }
            }
        }
    }
}

page 50103 "NB Cnt Count Parens"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count()) { }
            }
        }
    }
}

page 50104 "NB Cnt Field Count"
{
    PageType = List;
    SourceTable = "NB Cnt Field Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count) { }
            }
        }
    }
}

table 50102 "NB Cnt Proc Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }

    procedure Count(): Integer
    begin
        exit(42);
    end;
}

page 50105 "NB Cnt Card"
{
    PageType = Card;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            group(G)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count) { }
            }
        }
    }
}

page 50106 "NB Cnt Worksheet"
{
    PageType = Worksheet;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count) { }
            }
        }
    }
}

page 50107 "NB Cnt HasFilter"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.HasFilter) { }
            }
        }
    }
}

page 50108 "NB Cnt Bare Count"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Count) { }
            }
        }
    }
}

page 50109 "NB Cnt Proc Count"
{
    PageType = List;
    SourceTable = "NB Cnt Proc Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count) { }
            }
        }
    }
}

page 50110 "NB Cnt Plain"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Id) { }
            }
        }
    }
}

pageextension 50100 "NB Cnt Plain Ext" extends "NB Cnt Plain"
{
    layout
    {
        addlast(R)
        {
            field(ExtTotal; Rec.Count) { }
        }
    }
}

page 50111 "NB Cnt Other Expr"
{
    PageType = List;
    SourceTable = "NB Cnt Row";
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(Id; Rec.Id) { }
                field(Total; Rec.Count) { }
                field(Text; Format(Rec.Id)) { }
            }
        }
    }
}
