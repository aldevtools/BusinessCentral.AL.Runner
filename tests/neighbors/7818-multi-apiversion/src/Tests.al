codeunit 50285 "Probe Api Version Tests"
{
    Subtype = Test;

    [Test]
    procedure ApiVersionHoldsEveryVersion()
    var
        PageMetadata: Record "Page Metadata";
    begin
        // The reporter's repro of al-runner issue 7818, unchanged.
        PageMetadata.Get(Page::"Probe Multi Version Api");
        if PageMetadata.APIVersion <> 'v1.0,v2.0,beta' then
            Error('APIVersion is "%1" (length %2), expected "v1.0,v2.0,beta"', PageMetadata.APIVersion, StrLen(PageMetadata.APIVersion));
    end;

    [Test]
    procedure TwoVersionsKeepDeclaredOrder()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"Probe Two Desc Api");
        if PageMetadata.APIVersion <> 'v2.0,v1.0' then
            Error('APIVersion is "%1"', PageMetadata.APIVersion);
    end;

    [Test]
    procedure FilterOnFullList()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.SetRange(ID, Page::"Probe Multi Version Api");
        PageMetadata.SetRange(APIVersion, 'v1.0,v2.0,beta');
        if PageMetadata.Count() <> 1 then
            Error('full list: %1', PageMetadata.Count());
    end;

    [Test]
    procedure FilterOnFirstVersionOnly()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.SetRange(ID, Page::"Probe Multi Version Api");
        PageMetadata.SetRange(APIVersion, 'v1.0');
        if PageMetadata.Count() <> 0 then
            Error('first version: %1', PageMetadata.Count());
    end;

    [Test]
    procedure MultiVersionRowNameOnly()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"Probe Multi Version Api");
        if PageMetadata.Name <> 'Probe Multi Version Api' then
            Error('name: %1', PageMetadata.Name);
    end;

    [Test]
    procedure OneVersion()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"Probe One Version Api");
        if PageMetadata.APIVersion <> 'v2.0' then
            Error('APIVersion is "%1"', PageMetadata.APIVersion);
    end;

    [Test]
    procedure NoVersionReadsBeta()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"Probe Plain Card");
        if PageMetadata.APIVersion <> 'beta' then
            Error('APIVersion is "%1"', PageMetadata.APIVersion);
    end;

    [Test]
    procedure FourVersionsRefuse()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"Probe Four Version Api");
        if PageMetadata.APIVersion = '' then
            Error('empty');
    end;

    [Test]
    procedure FourVersionsOtherColumn()
    var
        PageMetadata: Record "Page Metadata";
    begin
        PageMetadata.Get(Page::"Probe Four Version Api");
        if PageMetadata.Name <> 'Probe Four Version Api' then
            Error('name: %1', PageMetadata.Name);
    end;

    [Test]
    procedure QueryThreeVersions()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.Get(Query::"Probe Multi Version Query");
        if QueryMetadata.APIVersion <> 'v2.0,beta,v1.0' then
            Error('APIVersion is "%1"', QueryMetadata.APIVersion);
    end;

    [Test]
    procedure QueryFourVersionsRefuse()
    var
        QueryMetadata: Record "Query Metadata";
    begin
        QueryMetadata.Get(Query::"Probe Four Version Query");
        if QueryMetadata.APIVersion = '' then
            Error('empty');
    end;
}
