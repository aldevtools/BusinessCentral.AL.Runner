table 60300 "R1 Api Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys { key(PK; Id) { } }
}

page 60301 "R1 Three Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'beta';
    EntityName = 'r1Three';
    EntitySetName = 'r1Threes';
    SourceTable = "R1 Api Row";
    DelayedInsert = true;
    ODataKeyFields = Id;

    layout
    {
        area(Content)
        {
            field(id; Rec.Id) { }
        }
    }
}

page 60302 "R1 Desc Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v2.0', 'v1.0';
    EntityName = 'r1Desc';
    EntitySetName = 'r1Descs';
    SourceTable = "R1 Api Row";
    DelayedInsert = true;
    ODataKeyFields = Id;

    layout
    {
        area(Content)
        {
            field(id; Rec.Id) { }
        }
    }
}

page 60303 "R1 Four Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'v3.0', 'beta';
    EntityName = 'r1Four';
    EntitySetName = 'r1Fours';
    SourceTable = "R1 Api Row";
    DelayedInsert = true;
    ODataKeyFields = Id;

    layout
    {
        area(Content)
        {
            field(id; Rec.Id) { }
        }
    }
}

page 60304 "R1 Five Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'v3.0', 'v4.0', 'beta';
    EntityName = 'r1Five';
    EntitySetName = 'r1Fives';
    SourceTable = "R1 Api Row";
    DelayedInsert = true;
    ODataKeyFields = Id;

    layout
    {
        area(Content)
        {
            field(id; Rec.Id) { }
        }
    }
}

query 60305 "R1 Two Query"
{
    QueryType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v2.0', 'v1.0';
    EntityName = 'r1TwoQuery';
    EntitySetName = 'r1TwoQuerys';

    elements
    {
        dataitem(Row; "R1 Api Row")
        {
            column(id; Id) { }
        }
    }
}

query 60306 "R1 Three Query"
{
    QueryType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v2.0', 'beta', 'v1.0';
    EntityName = 'r1ThreeQuery';
    EntitySetName = 'r1ThreeQuerys';

    elements
    {
        dataitem(Row; "R1 Api Row")
        {
            column(id; Id) { }
        }
    }
}

query 60307 "R1 Four Query"
{
    QueryType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'v3.0', 'beta';
    EntityName = 'r1FourQuery';
    EntitySetName = 'r1FourQuerys';

    elements
    {
        dataitem(Row; "R1 Api Row")
        {
            column(id; Id) { }
        }
    }
}

query 60308 "R1 Five Query"
{
    QueryType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'v3.0', 'v4.0', 'beta';
    EntityName = 'r1FiveQuery';
    EntitySetName = 'r1FiveQuerys';

    elements
    {
        dataitem(Row; "R1 Api Row")
        {
            column(id; Id) { }
        }
    }
}

