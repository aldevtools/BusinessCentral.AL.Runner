// What OnRun of a test codeunit reaches through the codeunit's global Records (al-runner issue 8992). The subscriber
// of table "NB8992 Row" writes isolated storage. "NB8992 Unused" never names its global in OnRun; "NB8992 Named" inserts
// through it in OnRun; "NB8992 Helper Named" names it in a procedure OnRun calls. The later codeunit reads the keys.
// Main and head must give the same results: only which codeunits join the serial carry chain changes.
table 99600 "NB8992 Row"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 99601 "NB8992 Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"NB8992 Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertRow(var Rec: Record "NB8992 Row"; RunTrigger: Boolean)
    begin
        IsolatedStorage.Set('NB8992-' + Format(Rec.K), 'x', DataScope::Module);
    end;
}

codeunit 99602 "NB8992 Unused"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992 Row";
        IsInitialized: Boolean;

    trigger OnRun()
    begin
        IsInitialized := false;
    end;

    [Test]
    procedure UnusedTakesNumber()
    var
        NextNo: BigInteger;
    begin
        if not NumberSequence.Exists('NB8992Seq') then
            NumberSequence.Insert('NB8992Seq');
        NextNo := NumberSequence.Next('NB8992Seq');
        Row.K := 1;
        Row.Insert();
        if not IsolatedStorage.Contains('NB8992-1', DataScope::Module) then
            Error('no key 1');
    end;
}

codeunit 99603 "NB8992 Named"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992 Row";

    trigger OnRun()
    begin
        Row.K := 2;
        if Row.Insert() then;
    end;

    [Test]
    procedure NamedSeesOwnKey()
    begin
        if not IsolatedStorage.Contains('NB8992-2', DataScope::Module) then
            Error('no key 2');
    end;
}

codeunit 99604 "NB8992 Helper Named"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992 Row";

    trigger OnRun()
    begin
        AddRow();
    end;

    local procedure AddRow()
    begin
        Row.K := 3;
        if Row.Insert() then;
    end;

    [Test]
    procedure HelperSeesOwnKey()
    begin
        if not IsolatedStorage.Contains('NB8992-3', DataScope::Module) then
            Error('no key 3');
    end;
}

codeunit 99605 "NB8992 Reads"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ReadsKeyOfUnusedTest()
    begin
        if IsolatedStorage.Contains('NB8992-1', DataScope::Module) then
            Error('key 1 of a test is here');
    end;

    [Test]
    procedure ReadsKeyOfNamedOnRun()
    begin
        if not IsolatedStorage.Contains('NB8992-2', DataScope::Module) then
            Error('no key 2');
    end;

    [Test]
    procedure ReadsKeyOfHelperOnRun()
    begin
        if not IsolatedStorage.Contains('NB8992-3', DataScope::Module) then
            Error('no key 3');
    end;
}

// The implicit Rec of a TableNo codeunit is a global Record that a bare Insert() names.
codeunit 99606 "NB8992 TableNo"
{
    TableNo = "NB8992 Row";

    trigger OnRun()
    begin
        Rec.K := 4;
        if Insert() then;
    end;
}

codeunit 99607 "NB8992 TableNo Caller"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RunsTableNo()
    var
        Row: Record "NB8992 Row";
    begin
        Codeunit.Run(Codeunit::"NB8992 TableNo", Row);
    end;
}

// A subscriber bound by hand with BindSubscription(this) in OnRun, naming the global.
codeunit 99608 "NB8992 Manual Sub"
{
    Subtype = Test;
    TestPermissions = Disabled;
    EventSubscriberInstance = Manual;
    var
        Row: Record "NB8992 Row";

    trigger OnRun()
    begin
        BindSubscription(this);
    end;

    [EventSubscriber(ObjectType::Table, Database::"NB8992 Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertRow(var Rec: Record "NB8992 Row"; RunTrigger: Boolean)
    begin
        if Row.IsEmpty() then;
    end;

    [Test]
    procedure BindsInOnRun()
    begin
        Row.K := 5;
        if Row.Insert() then;
    end;
}

// A global named only inside an asserterror block of OnRun.
codeunit 99609 "NB8992 Asserterror"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992 Row";

    trigger OnRun()
    begin
        asserterror Row.Get(-1);
    end;

    [Test]
    procedure SeesAssertErrorGlobal()
    begin
        Row.K := 6;
        if Row.Insert() then;
    end;
}
