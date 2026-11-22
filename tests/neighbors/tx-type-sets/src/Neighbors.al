// Neighbors of al-runner issues 8707, 8708 and 8709: what CurrentTransactionType(X) does once the transaction has started,
// and a write at a type that does not write. Each shape is a test codeunit of its own; the shapes that do not test a
// codeunit's first test method start with an empty one. Refused shapes come last.
table 50000 "NB TS Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Flag; Boolean) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

report 50001 "NB TS Update"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    TransactionType = Update;

    dataset
    {
        dataitem(Loop; Integer)
        {
            DataItemTableView = sorting(Number) where(Number = const(1));

            trigger OnAfterGetRecord()
            var
                Row: Record "NB TS Row";
            begin
                Row."Entry No." := 99;
                if Row.Insert() then;
            end;
        }
    }
}

table 50003 "NB TS Other"
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

table 50004 "NB TS Trigger Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Flag; Boolean) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }

    trigger OnInsert()
    begin
        Flag := true;
    end;
}

table 50005 "NB TS Subscribed Row"
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

codeunit 50006 "NB TS Row Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NB TS Subscribed Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBeforeInsert(var Rec: Record "NB TS Subscribed Row"; RunTrigger: Boolean)
    begin
    end;
}

codeunit 50007 "NB TS Failer"
{
    trigger OnRun()
    begin
        Error('The run fails.');
    end;
}

codeunit 50002 "NB TS Writer"
{
    trigger OnRun()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 98;
        Row.Insert();
    end;
}

codeunit 50010 "NB TS Insert Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure InsertAtSnapshot_Raises()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Row."Entry No." := 1;
        asserterror Row.Insert();
        if GetLastErrorCode() <> 'DB:TransActionIsReadOnly' then
            Error('No error, or another: %1', GetLastErrorCode());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50011 "NB TS Modify Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure ModifyAtBrowse_Raises()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 2;
        Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::Browse);
        Row.Get(2);
        Row.Flag := true;
        asserterror Row.Modify();
        if GetLastErrorCode() <> 'DB:TransActionIsReadOnly' then
            Error('No error, or another: %1', GetLastErrorCode());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50012 "NB TS ModifyAll Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure ModifyAllAtReport_Raises()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 7;
        Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::Report);
        asserterror Row.ModifyAll(Flag, true);
        if GetLastErrorCode() <> 'DB:TransActionIsReadOnly' then
            Error('No error, or another: %1', GetLastErrorCode());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50013 "NB TS Temporary Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure TemporaryInsertAtSnapshot_Writes()
    var
        TempRow: Record "NB TS Row" temporary;
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        TempRow."Entry No." := 1;
        TempRow.Insert();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50014 "NB TS Update Report World"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure UpdateReportAtSnapshot_Writes()
    var
        Row: Record "NB TS Row";
        UpdateReport: Report "NB TS Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        if not Row.Get(99) then
            Error('The report did not write.');
    end;
}

codeunit 50015 "NB TS Snapshot After Write"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure NoLocksWriteSetSnapshot_Raises()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 3;
        Row.Insert();
        asserterror CurrentTransactionType(TransactionType::Snapshot);
        if GetLastErrorCode() <> 'DB:CannotChangeTransactionType' then
            Error('No error, or another: %1', GetLastErrorCode());
    end;
}

codeunit 50016 "NB TS Browse After Write"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure NoLocksWriteSetBrowse_NoEffect()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 4;
        Row.Insert();
        CurrentTransactionType(TransactionType::Browse);
        if Format(CurrentTransactionType()) <> 'UpdateNoLocks' then
            Error('Type: %1', CurrentTransactionType());
    end;
}

codeunit 50017 "NB TS NoLocks After Upd Write"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure UpdateWriteSetNoLocks_NoEffect()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        Row."Entry No." := 5;
        Row.Insert();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        if Format(CurrentTransactionType()) <> 'Update' then
            Error('Type: %1', CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50018 "NB TS Update Read Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure UpdateReadSetReport_NoEffect()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindFirst() then;
        CurrentTransactionType(TransactionType::Report);
        if Format(CurrentTransactionType()) <> 'Update' then
            Error('Type: %1', CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50019 "NB TS First Test Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FirstTestSetSnapshot_Raises()
    begin
        asserterror CurrentTransactionType(TransactionType::Snapshot);
        if GetLastErrorCode() <> 'DB:CannotChangeTransactionType' then
            Error('No error, or another: %1', GetLastErrorCode());
    end;
}

codeunit 50020 "NB TS First Test Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FirstTestSetBrowse_NoEffect()
    begin
        CurrentTransactionType(TransactionType::Browse);
        if Format(CurrentTransactionType()) <> 'UpdateNoLocks' then
            Error('Type: %1', CurrentTransactionType());
    end;
}

codeunit 50021 "NB TS First Test Rollback"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FirstTestRollbackSetBrowse_TakesEffect()
    begin
        asserterror Error('Rolled back.');
        CurrentTransactionType(TransactionType::Browse);
        if Format(CurrentTransactionType()) <> 'Browse' then
            Error('Type: %1', CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50028 "NB TS Leaves Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    // The test ends at Browse: the test runner's steps after it still write its result.
    [Test]
    procedure B_EndsAtBrowse()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Browse);
    end;

    [Test]
    procedure C_StartsAfterIt()
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50029 "NB TS Upd Read Report NoCommit"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure NoCommitUpdateReadReport_StaysUpdate()
    var
        Row: Record "NB TS Row";
    begin
        CurrentTransactionType(TransactionType::Update);
        if Row.FindFirst() then;
        CurrentTransactionType(TransactionType::Report);
        if Format(CurrentTransactionType()) <> 'Update' then
            Error('Type: %1', CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50030 "NB TS First Test Write Set"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FirstTestWriteSetSnapshot_Raises()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 10;
        Row.Insert();
        asserterror CurrentTransactionType(TransactionType::Snapshot);
        if GetLastErrorCode() <> 'DB:CannotChangeTransactionType' then
            Error('No error, or another: %1', GetLastErrorCode());
    end;
}

codeunit 50031 "NB TS Same Type After Read"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure SnapshotReadSetSnapshot_NoEffect()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        if Row.FindFirst() then;
        CurrentTransactionType(TransactionType::Snapshot);
        if Format(CurrentTransactionType()) <> 'Snapshot' then
            Error('Type: %1', CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50022 "NB TS Rename Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure RenameAtSnapshot_Refused()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 6;
        Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Row.Get(6);
        asserterror Row.Rename(7);
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50023 "NB TS Codeunit Run Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure CodeunitRunInsertAtSnapshot_Refused()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        if Codeunit.Run(Codeunit::"NB TS Writer") then;
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50024 "NB TS NoLocks Read Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure NoLocksReadSetReport_Refused()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        if Row.FindFirst() then;
        CurrentTransactionType(TransactionType::Report);
        if Format(CurrentTransactionType()) = 'Update' then
            Error('Type: Update');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50025 "NB TS NoLocks Read Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure NoLocksReadSetBrowse_Refused()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        if Row.FindFirst() then;
        CurrentTransactionType(TransactionType::Browse);
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50026 "NB TS First Test AutoRollbk"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure FirstTestAutoRollbackSetUpdate_Refused()
    begin
        CurrentTransactionType(TransactionType::Update);
    end;
}

codeunit 50027 "NB TS After Refused Setter"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Refused at the run (a FindLast after the set is a call no cell measures); what it did not run sets the type.
    [Test]
    procedure A_RefusedThenSetsType()
    var
        Row: Record "NB TS Row";
        UpdateReport: Report "NB TS Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    // A write of another table, then a failure: the write reads the type the test before may have left.
    [Test]
    procedure B_WriteThenFail_NotTrusted()
    var
        Other: Record "NB TS Other";
    begin
        Other."Entry No." := 1;
        Other.Insert();
        Error('The write ran.');
    end;

    // A write of another table that passes: it rests on the type the test before left being one that writes.
    [Test]
    procedure C_WriteThenPass_Flagged()
    var
        Other: Record "NB TS Other";
    begin
        Other."Entry No." := 2;
        Other.Insert();
    end;
}

codeunit 50032 "NB TS Insert RunTrigger"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure InsertTrueAtSnapshot_Refused()
    var
        Row: Record "NB TS Trigger Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Row."Entry No." := 1;
        asserterror Row.Insert(true);
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50033 "NB TS Insert Subscribed"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure InsertSubscribedAtSnapshot_Refused()
    var
        Row: Record "NB TS Subscribed Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Row."Entry No." := 1;
        asserterror Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50034 "NB TS Insert Duplicate"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure InsertDuplicateAtSnapshot_Refused()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 20;
        Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        asserterror Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50035 "NB TS Modify Unchanged"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure ModifyUnchangedAtSnapshot_Refused()
    var
        Row: Record "NB TS Row";
    begin
        Row."Entry No." := 21;
        Row.Insert();
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Row.Get(21);
        asserterror Row.Modify();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50036 "NB TS ModifyAll No Row"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_Nothing()
    begin
    end;

    [Test]
    procedure ModifyAllNoRowAtSnapshot_Refused()
    var
        Row: Record "NB TS Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Row.SetRange("Entry No.", -1);
        asserterror Row.ModifyAll(Flag, true);
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50037 "NB TS First Test Guarded Run"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FailedRunThenSetBrowse_Refused()
    begin
        if Codeunit.Run(Codeunit::"NB TS Failer") then;
        CurrentTransactionType(TransactionType::Browse);
        if Format(CurrentTransactionType()) <> 'Browse' then
            Error('Type: %1', CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}
