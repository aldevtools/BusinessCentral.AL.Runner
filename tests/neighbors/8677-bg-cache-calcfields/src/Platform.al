// A neighbor from the review of al-runner PR 8682, apart from Neighbors.al because the runner's unit tests compile that file
// without the platform tables: an exist FlowField over Tenant Media, with a Tenant Media row of its ID that the test inserts
// without a commit between the worker's two reads (BC on a local 29.1 container answers No both times).
table 50125 "BQN Media Parent"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Media Id"; Guid) { }
        field(3; "Has Media"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("Tenant Media" where(ID = field("Media Id")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 50126 "BQN Media Reader"
{
    trigger OnRun()
    var
        Parent: Record "BQN Media Parent";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        Parent.Get(Parameters.Get('Group'));
        Parent.CalcFields("Has Media");
        Results.Add('Seen', Format(Parent."Has Media"));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50127 "BQN Media Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Lines: TestPage "BQN Lines"; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Results := Lines.RunPageBackgroundTask(Codeunit::"BQN Media Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    [Test]
    procedure TenantMediaExist_MediaAfterFirstRead()
    var
        Lines: TestPage "BQN Lines";
        Parent: Record "BQN Media Parent";
        TenantMedia: Record "Tenant Media";
        G: Code[20];
        Seen: Text;
    begin
        G := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Parent."No." := G;
        Parent."Media Id" := CreateGuid();
        Parent.Insert();
        Commit();
        Lines.OpenView();
        Seen := Run(Lines, G);
        TenantMedia.ID := Parent."Media Id";
        TenantMedia.Description := 'm';
        TenantMedia.Insert(false);
        Seen += ' / ' + Run(Lines, G);
        Lines.Close();
        Assert.AreEqual('No / No', Seen, 'an exist over Tenant Media');
    end;
}
