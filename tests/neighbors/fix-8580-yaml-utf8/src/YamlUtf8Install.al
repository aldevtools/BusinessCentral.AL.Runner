// al-runner issue 8580, second round: the reporter's install routine imports AI eval datasets (.yaml app resources)
// through JsonObject.ReadFromYaml, and every dataset holds a section sign (U+00A7) in a top `>-` block scalar. The
// dataset is the document of corpus PR 1515 (JsonYamlUtf8.al, ReadFromYaml_Utf8IssueDocument_Read). The install baseline
// runs the import; only the <baseline> row carries the marks. No test reads the dataset table: install marks are not
// carried to the tests that read it (issue 8636).
table 60930 "YW Dataset"
{
    fields
    {
        field(1; Name; Text[100]) { }
        field(2; Description; Text[250]) { }
    }
    keys
    {
        key(PK; Name) { Clustered = true; }
    }
}

codeunit 60930 "YW Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Dataset: Record "YW Dataset";
        Doc: JsonObject;
        Tok: JsonToken;
        Name: Text;
    begin
        foreach Name in NavApp.ListResources('*.yaml') do begin
            Doc.ReadFromYaml(NavApp.GetResourceAsText(Name, TextEncoding::UTF8));
            Doc.Get('name', Tok);
            Dataset.Name := CopyStr(Tok.AsValue().AsText(), 1, MaxStrLen(Dataset.Name));
            Doc.Get('description', Tok);
            Dataset.Description := CopyStr(Tok.AsValue().AsText(), 1, MaxStrLen(Dataset.Description));
            Dataset.Insert();
        end;
    end;
}

codeunit 60931 "YW Install Tests"
{
    Subtype = Test;

    [Test]
    procedure DependencyRowFromInstall()
    var
        Row: Record "YW Dep Row";
    begin
        Row.Get('DEP');
    end;
}
