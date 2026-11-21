// al-runner issue 8580, end to end: an install codeunit first fills a suite table that every test reads, then imports
// an evaluation dataset that ships as a .yaml app resource through JsonObject.ReadFromYaml (a blank line between two
// block list items). Main refused the read, skipped the install routine and refused every test.
table 60930 "YI Suite"
{
    fields
    {
        field(1; Code; Code[20]) { }
    }
    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

table 60931 "YI Dataset Line"
{
    fields
    {
        field(1; Name; Text[100]) { }
        field(2; Question; Text[250]) { }
        field(3; Amount; Text[30]) { }
    }
    keys
    {
        key(PK; Name) { Clustered = true; }
    }
}

codeunit 60930 "YI Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        CreateSuite();
        ImportDatasets();
    end;

    local procedure CreateSuite()
    var
        Suite: Record "YI Suite";
    begin
        Suite.Code := 'DEMO';
        Suite.Insert();
    end;

    local procedure ImportDatasets()
    var
        Line: Record "YI Dataset Line";
        Doc: JsonObject;
        Tok: JsonToken;
        Item: JsonToken;
        Field: JsonToken;
        Name: Text;
    begin
        foreach Name in NavApp.ListResources('*.yaml') do begin
            Doc.ReadFromYaml(NavApp.GetResourceAsText(Name, TextEncoding::UTF8));
            Doc.Get('tests', Tok);
            foreach Item in Tok.AsArray() do begin
                Line.Init();
                Item.AsObject().Get('name', Field);
                Line.Name := CopyStr(Field.AsValue().AsText(), 1, MaxStrLen(Line.Name));
                Item.AsObject().Get('question', Field);
                Line.Question := CopyStr(Field.AsValue().AsText(), 1, MaxStrLen(Line.Question));
                Item.AsObject().Get('Expected Amount', Field);
                Line.Amount := CopyStr(Field.AsValue().AsText(), 1, MaxStrLen(Line.Amount));
                Line.Insert();
            end;
        end;
    end;
}

codeunit 60931 "YI Install Tests"
{
    Subtype = Test;

    [Test]
    procedure SuiteRowFromInstall()
    var
        Suite: Record "YI Suite";
    begin
        Suite.Get('DEMO');
    end;

    [Test]
    procedure DatasetRowsFromInstall()
    var
        Line: Record "YI Dataset Line";
        Lf: Text[1];
    begin
        Lf[1] := 10;
        if Line.Count() <> 2 then
            Error('Expected 2 dataset lines, got %1', Line.Count());
        Line.Get('First case');
        if Line.Question <> 'Line one' + Lf + 'Line two' + Lf then
            Error('Unexpected question <%1>', Line.Question);
        if Line.Amount <> '10' then
            Error('Unexpected amount <%1>', Line.Amount);
        Line.Get('Second case');
        if Line.Question <> 'Only line' then
            Error('Unexpected question <%1>', Line.Question);
    end;
}
