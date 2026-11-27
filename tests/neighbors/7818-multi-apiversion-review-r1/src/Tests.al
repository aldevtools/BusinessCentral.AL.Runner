codeunit 60310 "R1 Api Version Tests"
{
    Subtype = Test;

    local procedure Check(Got: Text; Want: Text; What: Text)
    begin
        if Got <> Want then
            Error('%1 is "%2", expected "%3"', What, Got, Want);
    end;

    [Test]
    procedure QueryTwoVersions()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.Get(Query::"R1 Two Query");
        Check(QueryMetadata.APIVersion, 'v2.0,v1.0', 'value');
        Check(Format(StrLen(QueryMetadata.APIVersion)), '9', 'length');
    end;

    [Test]
    procedure QueryExactFilters()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.SetRange(ID, Query::"R1 Two Query");
        QueryMetadata.SetRange(APIVersion, 'v2.0,v1.0');
        Check(Format(QueryMetadata.Count()), '1', 'full list');
        QueryMetadata.SetRange(APIVersion, 'v1.0');
        Check(Format(QueryMetadata.Count()), '0', 'one version');
    end;

    [Test]
    procedure PageWildcardFilters()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.SetRange(ID, Page::"R1 Three Api");
        PageMetadata.SetFilter(APIVersion, '*v2.0*');
        Check(Format(PageMetadata.Count()), '1', 'contains');
        PageMetadata.SetFilter(APIVersion, 'v1.0*');
        Check(Format(PageMetadata.Count()), '1', 'prefix');
        PageMetadata.SetFilter(APIVersion, 'v1.0|v1.0,v2.0,beta');
        Check(Format(PageMetadata.Count()), '1', 'or full list');
        PageMetadata.SetRange(ID, Page::"R1 Desc Api");
        PageMetadata.SetFilter(APIVersion, '*v1.0');
        Check(Format(PageMetadata.Count()), '1', 'suffix');
        PageMetadata.SetFilter(APIVersion, 'v1.0');
        Check(Format(PageMetadata.Count()), '0', 'one version');
    end;

    [Test]
    procedure QueryWildcardFilters()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.SetRange(ID, Query::"R1 Three Query");
        QueryMetadata.SetFilter(APIVersion, '*v2.0*');
        Check(Format(QueryMetadata.Count()), '1', 'contains');
        QueryMetadata.SetFilter(APIVersion, '*beta');
        Check(Format(QueryMetadata.Count()), '0', 'suffix beta');
        QueryMetadata.SetFilter(APIVersion, 'v1.0|v2.0,beta,v1.0');
        Check(Format(QueryMetadata.Count()), '1', 'or full list');
    end;

    [Test]
    procedure PageRecordRefValue()
    var
        PageMetadata: Record "Page Metadata";
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        PageMetadata.Get(Page::"R1 Three Api");
        RecRef.GetTable(PageMetadata);
        FldRef := RecRef.Field(PageMetadata.FieldNo(APIVersion));
        Check(Format(FldRef.Value), 'v1.0,v2.0,beta', 'FieldRef value');
        Check(Format(FldRef.Length), '250', 'FieldRef length');
    end;

    [Test]
    procedure PageFourVersions()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"R1 Four Api");
        Check(PageMetadata.APIVersion, 'v1.0,v2.0,v3.0,beta', 'value');
        PageMetadata.SetRange(ID, Page::"R1 Four Api");
        PageMetadata.SetFilter(APIVersion, '*v3.0*');
        Check(Format(PageMetadata.Count()), '1', 'contains v3');
    end;

    [Test]
    procedure QueryFourVersions()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.Get(Query::"R1 Four Query");
        Check(QueryMetadata.APIVersion, 'v1.0,v2.0,v3.0,beta', 'value');
        QueryMetadata.SetRange(ID, Query::"R1 Four Query");
        QueryMetadata.SetRange(APIVersion, 'v1.0');
        Check(Format(QueryMetadata.Count()), '0', 'one version');
    end;

    [Test]
    procedure PageFiveVersionsRefuse()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"R1 Five Api");
        if PageMetadata.APIVersion = '' then
            Error('empty');
    end;

    [Test]
    procedure QueryFiveVersionsRefuse()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.Get(Query::"R1 Five Query");
        if QueryMetadata.APIVersion = '' then
            Error('empty');
    end;

    [Test]
    procedure FiveVersionsOtherColumn()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"R1 Five Api");
        Check(PageMetadata.Name, 'R1 Five Api', 'Name');
    end;
}
