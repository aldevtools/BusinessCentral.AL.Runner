// Review round 2 of the fix for al-runner issue 8969: a second read of the type in the saving procedure (before or after
// the set between), and a rollback between the save and the set back. Each shape is a test codeunit of its own: a first
// test that is refused while it runs and could set the type, then the shape, then a test that reads the type.
table 50100 "NB TV2 Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

report 50101 "NB TV2 Update"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    TransactionType = Update;

    dataset
    {
        dataitem(Loop; "NB TV2 Row") { }
    }
}

codeunit 50102 "NB TV2 Lib"
{
    procedure SaveReadSetRestore()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        if CurrentTransactionType = TransactionType::Update then;
        Commit();
        CurrentTransactionType(Saved);
    end;

    procedure SaveRestoreAfterRollback()
    var
        Saved: TransactionType;
        Row: Record "NB TV2 Row";
    begin
        Commit();
        Saved := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Row."Entry No." := 8;
        Row.Insert();
        asserterror Error('x');
        CurrentTransactionType(Saved);
    end;

    procedure SaveReadBeforeSet()
    var
        Saved: TransactionType;
    begin
        Commit();
        Saved := CurrentTransactionType;
        if CurrentTransactionType = TransactionType::Update then;
        CurrentTransactionType(TransactionType::Update);
        Commit();
        CurrentTransactionType(Saved);
    end;
}

codeunit 50110 "NB TV Saved Read Between"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV2 Row";
        UpdateReport: Report "NB TV2 Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveReadSetRestore_Refused()
    var
        Lib: Codeunit "NB TV2 Lib";
        Row: Record "NB TV2 Row";
    begin
        Lib.SaveReadSetRestore();
        Row."Entry No." := 1;
        Row.Insert();
    end;

    [Test]
    procedure C_ReadsType_Refused()
    begin
        if CurrentTransactionType <> TransactionType::UpdateNoLocks then
            Error('The type is %1.', CurrentTransactionType);
    end;
}

codeunit 50111 "NB TV Saved Read Before Set"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV2 Row";
        UpdateReport: Report "NB TV2 Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SaveReadBeforeSet_Refused()
    var
        Lib: Codeunit "NB TV2 Lib";
        Row: Record "NB TV2 Row";
    begin
        Lib.SaveReadBeforeSet();
        Row."Entry No." := 1;
        Row.Insert();
    end;

    [Test]
    procedure C_ReadsType_Refused()
    begin
        if CurrentTransactionType <> TransactionType::UpdateNoLocks then
            Error('The type is %1.', CurrentTransactionType);
    end;
}

codeunit 50112 "NB TV Restore After Rollback"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TV2 Row";
        UpdateReport: Report "NB TV2 Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.UseRequestPage := false;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_SetBackAfterRollback_Refused()
    var
        Lib: Codeunit "NB TV2 Lib";
        Row: Record "NB TV2 Row";
    begin
        Lib.SaveRestoreAfterRollback();
        Row."Entry No." := 1;
        Row.Insert();
    end;

    [Test]
    procedure C_ReadsType_Refused()
    begin
        if CurrentTransactionType <> TransactionType::UpdateNoLocks then
            Error('The type is %1.', CurrentTransactionType);
    end;
}

// The same shapes with the type known (no test before): they run as on main.
codeunit 50113 "NB TV Saved Shapes Known"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SaveReadSetRestore_TypeKnown_Runs()
    var
        Lib: Codeunit "NB TV2 Lib";
    begin
        Lib.SaveReadSetRestore();
    end;

    [Test]
    procedure SaveReadBeforeSet_TypeKnown_Runs()
    var
        Lib: Codeunit "NB TV2 Lib";
    begin
        Lib.SaveReadBeforeSet();
    end;

    [Test]
    procedure SetBackAfterRollback_TypeKnown_Runs()
    var
        Lib: Codeunit "NB TV2 Lib";
    begin
        Lib.SaveRestoreAfterRollback();
    end;
}
