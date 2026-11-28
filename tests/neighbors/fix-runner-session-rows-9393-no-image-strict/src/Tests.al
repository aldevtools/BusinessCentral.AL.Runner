codeunit 54110 "FRN H Hub Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure H_MarkHub()
    var
        Hub: Record "FRN Hub";
    begin
        Hub.Code := 'HUB';
        Hub.Value := SessionId();
        Hub.Insert();
        Commit();
    end;
}

codeunit 54120 "FRN I Insert"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Row: Record "FRN Row";
    begin
        Row."Session Id" := SessionId();
        Row.Kind := 'INSERT';
        Row.Insert();
    end;

    // :795: the codeunit's own row.
    [Test]
    procedure I_Cell()
    var
        Check: Codeunit "FRN Check";
    begin
        Check.Cell('INSERT', 1);
    end;
}

codeunit 54121 "FRN C Commit"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Row: Record "FRN Row";
    begin
        Row."Session Id" := SessionId();
        Row.Kind := 'COMMIT';
        Row.Insert();
        Commit();
    end;

    // :839, :840: the INSERT row is gone, its own COMMIT row is there.
    [Test]
    procedure C_Cell()
    var
        Check: Codeunit "FRN Check";
    begin
        Check.Cell('COMMIT', 1);
        Check.Equal(0, Check.Rows('INSERT', SessionId()), 'INSERT rows');
    end;
}

// :877, :878: both rows are gone.
codeunit 54130 "FRN N Reader"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure N_Cell()
    var
        Check: Codeunit "FRN Check";
    begin
        Check.Cell('INSERT', 0);
        Check.Equal(0, Check.Rows('COMMIT', SessionId()), 'COMMIT rows');
    end;
}

// :917, :918: the same with a MessageHandler and TestPermissions = Disabled.
codeunit 54131 "FRN D Reader Handler"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('FrnMessage')]
    procedure D_Cell()
    var
        Check: Codeunit "FRN Check";
    begin
        Message('FRN');
        Check.Cell('INSERT', 0);
        Check.Equal(0, Check.Rows('COMMIT', SessionId()), 'COMMIT rows');
    end;

    [MessageHandler]
    procedure FrnMessage(Text: Text[1024])
    begin
    end;
}
