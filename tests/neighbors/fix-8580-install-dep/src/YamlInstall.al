// al-runner issue 8580 with a dependency app: the reporter's path. The workspace app depends on an app whose install code
// runs in the install baseline, and the tests read what the baseline holds (a dependency table row and the dataset
// rows its own install code imported through JsonObject.ReadFromYaml). The app covers the dependency and baseline path; only the
// <baseline> row carries the assumption mark. The two tests that read the dataset tables are in issue 8636 (table-scoped marks).
table 60940 "YD Suite"
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

table 60941 "YD Dataset Line"
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

codeunit 60940 "YD Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        CreateSuite();
        ImportDatasets();
    end;

    local procedure CreateSuite()
    var
        Suite: Record "YD Suite";
    begin
        Suite.Code := 'DEMO';
        Suite.Insert();
    end;

    local procedure ImportDatasets()
    var
        Line: Record "YD Dataset Line";
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

codeunit 60941 "YD Install Tests"
{
    Subtype = Test;

    [Test]
    procedure DependencyRowFromInstall()
    var
        Row: Record "YD Dep Row";
    begin
        Row.Get('DEP');
    end;
}
