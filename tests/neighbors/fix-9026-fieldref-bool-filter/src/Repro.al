// The reporter's repro of al-runner issue 9026, one codeunit per cell (Code[20] or Text[50], SetRange or SetFilter).
table 51000 "FC Row"
{
    fields
    {
        field(1; "Key"; Integer) { }
        field(5; Cod; Code[20]) { }
        field(6; Txt; Text[50]) { }
    }
    keys { key(PK; "Key") { Clustered = true; } }
}

codeunit 51097 "FC Code SetRange Bool"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Cell()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        V: Boolean;
    begin
        RecRef.Open(51000);
        FR := RecRef.Field(5);
        V := false;
        FR.SetRange(V);
        Error('V:[%1]', FR.GetFilter());
    end;
}

codeunit 51098 "FC Code SetFilter Bool"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Cell()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        V: Boolean;
    begin
        RecRef.Open(51000);
        FR := RecRef.Field(5);
        V := false;
        FR.SetFilter('%1', V);
        Error('V:[%1]', FR.GetFilter());
    end;
}

codeunit 51099 "FC Text SetRange Bool"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Cell()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        V: Boolean;
    begin
        RecRef.Open(51000);
        FR := RecRef.Field(6);
        V := false;
        FR.SetRange(V);
        Error('V:[%1]', FR.GetFilter());
    end;
}

codeunit 51100 "FC Text SetFilter Bool"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Cell()
    var
        RecRef: RecordRef;
        FR: FieldRef;
        V: Boolean;
    begin
        RecRef.Open(51000);
        FR := RecRef.Field(6);
        V := false;
        FR.SetFilter('%1', V);
        Error('V:[%1]', FR.GetFilter());
    end;
}
