// The CRM neighbor of "@ alone" (al-runner issue 8637), apart from Neighbors.al because the runner's unit tests compile that
// file without a CRM table connection: @ alone on a Code field of a TableType = CRM table, read through the platform's test
// connection ('@@test@@'). BC does not read these rows from SQL, and no corpus cell measures them; the runner refuses the read.
table 97504 "AS Crm Kind"
{
    TableType = CRM;
    ExternalName = 'as_crm_kind';
    DataClassification = SystemMetadata;

    fields
    {
        field(1; EntityId; Guid)
        {
            ExternalName = 'as_crm_kindid';
            ExternalType = 'Uniqueidentifier';
            ExternalAccess = Insert;
            DataClassification = SystemMetadata;
        }
        field(2; Kind; Code[20])
        {
            ExternalName = 'as_kind';
            ExternalType = 'String';
            DataClassification = SystemMetadata;
        }
    }

    keys
    {
        key(PK; EntityId) { Clustered = true; }
    }
}

codeunit 97536 "AS Crm Neighbor Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AtSignAlone_CrmTable_CodeField()
    var
        Row: Record "AS Crm Kind";
        Seen: Integer;
    begin
        if HasTableConnection(TableConnectionType::CRM, 'ASKIND') then
            UnregisterTableConnection(TableConnectionType::CRM, 'ASKIND');
        RegisterTableConnection(TableConnectionType::CRM, 'ASKIND', '@@test@@');
        SetDefaultTableConnection(TableConnectionType::CRM, 'ASKIND');
        Evaluate(Row.EntityId, '{00000001-0000-0000-0000-000000000000}');
        Row.Kind := 'K';
        Row.Insert();
        Evaluate(Row.EntityId, '{00000002-0000-0000-0000-000000000000}');
        Row.Kind := '';
        Row.Insert();
        Row.SetFilter(Kind, '@');
        Seen := Row.Count();
        UnregisterTableConnection(TableConnectionType::CRM, 'ASKIND');
        Assert.AreEqual(2, Seen, '@ on a CRM Code field');
    end;
}
