table 50283 "Probe Api Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys { key(PK; Id) { } }
}

page 50284 "Probe Multi Version Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'beta';
    EntityName = 'probeRow';
    EntitySetName = 'probeRows';
    SourceTable = "Probe Api Row";
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

page 50286 "Probe Two Desc Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v2.0', 'v1.0';
    EntityName = 'probeDescRow';
    EntitySetName = 'probeDescRows';
    SourceTable = "Probe Api Row";
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

page 50287 "Probe One Version Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v2.0';
    EntityName = 'probeOneRow';
    EntitySetName = 'probeOneRows';
    SourceTable = "Probe Api Row";
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

page 50288 "Probe Four Version Api"
{
    PageType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'v3.0', 'beta';
    EntityName = 'probeFourRow';
    EntitySetName = 'probeFourRows';
    SourceTable = "Probe Api Row";
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

page 50289 "Probe Plain Card"
{
    PageType = Card;
    SourceTable = "Probe Api Row";

    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
        }
    }
}

query 50290 "Probe Multi Version Query"
{
    QueryType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v2.0', 'beta', 'v1.0';
    EntityName = 'probeQueryRow';
    EntitySetName = 'probeQueryRows';

    elements
    {
        dataitem(Row; "Probe Api Row")
        {
            column(id; Id) { }
        }
    }
}

query 50291 "Probe Four Version Query"
{
    QueryType = API;
    APIPublisher = 'probe';
    APIGroup = 'probe';
    APIVersion = 'v1.0', 'v2.0', 'v3.0', 'beta';
    EntityName = 'probeFourQueryRow';
    EntitySetName = 'probeFourQueryRows';

    elements
    {
        dataitem(Row; "Probe Api Row")
        {
            column(id; Id) { }
        }
    }
}
