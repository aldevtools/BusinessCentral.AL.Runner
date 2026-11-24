// A global RecordRef bound to a global Record in one routine reaches the table in another routine that never names the
// Record (al-runner issue 8992, review round 1). Main and head must give the same results.
table 99700 "NB8992A Row"
{
    fields { field(1; K; Integer) { } }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 99701 "NB8992A Sub"
{
    [EventSubscriber(ObjectType::Table, Database::"NB8992A Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertRow(var Rec: Record "NB8992A Row"; RunTrigger: Boolean)
    begin
        IsolatedStorage.Set('NB8992A-' + Format(Rec.K), 'x', DataScope::Module);
    end;
}

// OnRun binds RR to Row and inserts through RR, naming only the RecordRef.
codeunit 99702 "NB8992A Alias"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992A Row";
        RR: RecordRef;

    trigger OnRun()
    begin
        Bind();
        Row.K := 1;
        RR.GetTable(Row);
        if RR.Insert() then;
    end;

    local procedure Bind()
    begin
        RR.GetTable(Row);
    end;

    [Test]
    procedure RefOnlyBody()
    begin
        RR.Reset();
        if RR.IsEmpty() then;
    end;
}

// OnRun names only the RecordRef; Row is bound in a procedure the test calls.
codeunit 99703 "NB8992A OnRun Ref Only"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992A Row";
        RR: RecordRef;

    trigger OnRun()
    begin
        RR.Reset();
    end;

    [Test]
    procedure BindsAndInserts()
    begin
        Row.K := 2;
        RR.GetTable(Row);
        if RR.Insert() then;
    end;
}

codeunit 99704 "NB8992A Reads"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ReadsKeyOfRefInsert()
    begin
        if IsolatedStorage.Contains('NB8992A-2', DataScope::Module) then
            Error('key 2 of a test is here');
    end;
}

// A Variant slot named by OnRun: the same alias rule.
codeunit 99705 "NB8992A Variant"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        Row: Record "NB8992A Row";
        Any: Variant;

    trigger OnRun()
    begin
        Any := 1;
    end;

    [Test]
    procedure InsertsRow()
    begin
        Row.K := 3;
        if Row.Insert() then;
    end;
}
