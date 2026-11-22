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
