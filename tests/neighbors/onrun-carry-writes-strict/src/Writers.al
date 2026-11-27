// Each test codeunit's OnRun is one shape of al-runner issue 7622; "OCW Reader" (86910) reads what they left.
// The codeunits run under runner 130450 (no TestPermissions = Disabled), where OnRun's writes reach the later codeunits
// (corpus codeunits/run-onrun/RqIsolationModes.al:357-369, inserted rows only).
codeunit 86901 "OCW Seed"
{
    Subtype = Test;

    trigger OnRun()
    var
        Row: Record "OCW Row";
    begin
        Row."Key" := 'DA1';
        Row.Insert();
        Row."Key" := 'DA2';
        Row.Insert();
        Row."Key" := 'D1';
        Row.Insert();
    end;

    [Test]
    procedure Seed_Runs()
    begin
    end;
}

codeunit 86902 "OCW DeleteAll Only"
{
    Subtype = Test;

    trigger OnRun()
    var
        Row: Record "OCW Row";
    begin
        Row.SetFilter("Key", 'DA*');
        Row.DeleteAll();
    end;

    [Test]
    procedure DeleteAllOnly_Runs()
    begin
    end;
}

codeunit 86903 "OCW Delete One Only"
{
    Subtype = Test;

    trigger OnRun()
    var
        Row: Record "OCW Row";
    begin
        if Row.Get('D1') then
            Row.Delete();
    end;

    [Test]
    procedure DeleteOneOnly_Runs()
    begin
    end;
}

codeunit 86904 "OCW RecordRef Insert"
{
    Subtype = Test;

    trigger OnRun()
    var
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        RecRef.Open(86900);
        RecRef.Init();
        FldRef := RecRef.Field(1);
        FldRef.Value := 'RR';
        RecRef.Insert();
    end;

    [Test]
    procedure RecordRefInsert_Runs()
    begin
    end;
}

codeunit 86905 "OCW Subscriber Write"
{
    Subtype = Test;

    trigger OnRun()
    begin
        OnHook();
    end;

    [IntegrationEvent(false, false)]
    procedure OnHook()
    begin
    end;

    [Test]
    procedure SubscriberWrite_Runs()
    begin
    end;
}

codeunit 86906 "OCW Temp Only"
{
    Subtype = Test;

    trigger OnRun()
    var
        TempRow: Record "OCW Row" temporary;
    begin
        TempRow."Key" := 'TMP';
        TempRow.Insert();
        TempRow.DeleteAll();
    end;

    [Test]
    procedure TempOnly_Runs()
    begin
    end;
}

codeunit 86907 "OCW Temp Only Raises"
{
    Subtype = Test;

    trigger OnRun()
    var
        TempRow: Record "OCW Row" temporary;
    begin
        TempRow."Key" := 'TMPR';
        TempRow.Insert();
        Error('OnRun stops');
    end;

    [Test]
    procedure TempOnlyRaises_Runs()
    begin
    end;
}

codeunit 86908 "OCW Temp Only Refused"
{
    Subtype = Test;

    trigger OnRun()
    var
        TempRow: Record "OCW Row" temporary;
    begin
        TempRow."Key" := 'TMPX';
        TempRow.Insert();
        Database.ExportData(true, 'x', '');
    end;

    [Test]
    procedure TempOnlyRefused_Runs()
    begin
    end;
}
