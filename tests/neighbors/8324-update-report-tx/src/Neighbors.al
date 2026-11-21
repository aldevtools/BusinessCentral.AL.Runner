// Neighbors of al-runner issues 8324, 8605 and 8606 (corpus pull request 1488): an Update report run without its request
// page after a Commit or a CurrentTransactionType set, the type a Commit keeps, the type the next test method starts with.
// Each shape is a test codeunit of its own: a codeunit runs in a session of its own, so the type one shape leaves reaches no
// other shape. Every test that sets a type puts UpdateNoLocks back after a Commit, as Library - Manufacturing does.
table 50000 "NB UR Row"
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

report 50001 "NB UR Update"
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
                Row: Record "NB UR Row";
            begin
                Row."Entry No." := MarkerNo;
                Row.Insert();
            end;
        }
    }

    var
        MarkerNo: Integer;

    procedure SetMarker(NewMarkerNo: Integer)
    begin
        MarkerNo := NewMarkerNo;
    end;
}

codeunit 50002 "NB UR Helper"
{
    trigger OnRun()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;

    procedure CommitAndSetUpdate()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;
}

// The steps of Library - Manufacturing.RefreshProdOrder around report 50001.
codeunit 50003 "NB UR Library"
{
    procedure Refresh(var Row: Record "NB UR Row"; MarkerNo: Integer)
    var
        UpdateReport: Report "NB UR Update";
        TempTransactionType: TransactionType;
    begin
        Commit();
        TempTransactionType := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);
        Row.Get(Row."Entry No.");
        UpdateReport.SetMarker(MarkerNo);
        UpdateReport.UseRequestPage := false;
        UpdateReport.RunModal();
        Commit();
        CurrentTransactionType(TempTransactionType);
    end;
}

codeunit 50010 "NB UR Library Steps"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure LibrarySteps_RunTheReport()
    var
        Row: Record "NB UR Row";
        Library: Codeunit "NB UR Library";
    begin
        Row."Entry No." := 1;
        Row.Insert();
        Library.Refresh(Row, 2);
        if not Row.Get(2) then
            Error('The report did not write its marker.');
        if Format(CurrentTransactionType()) <> 'UpdateNoLocks' then
            Error('Type after the library: %1', CurrentTransactionType());
    end;
}

codeunit 50011 "NB UR Issue Repro"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSetUpdateInsertRun_NoWorld()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        Row."Entry No." := 1;
        Row.Insert();
        UpdateReport.SetMarker(2);
        UpdateReport.Run();
        asserterror Error('trapped');
        if Row.Get(2) then
            Error('The report committed its marker: a transaction world.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50012 "NB UR Commit No Set"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitNoSetRun_World()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        Commit();
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
        asserterror Error('trapped');
        if not Row.Get(1) then
            Error('The report did not commit its marker: no transaction world.');
    end;
}

codeunit 50013 "NB UR Commit Set Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSetSnapshotRun_World()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
        asserterror Error('trapped');
        if not Row.Get(1) then
            Error('The report did not commit its marker: no transaction world.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50014 "NB UR Start DeleteAll"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetUpdateAtStartDeleteAll_Refused()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        CurrentTransactionType(TransactionType::Update);
        Row.DeleteAll();
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50015 "NB UR FindLast"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSetUpdateFindLast_Refused()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50016 "NB UR Run Insert Run"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSetUpdateRunInsertRun_Refused()
    var
        Row: Record "NB UR Row";
        First: Report "NB UR Update";
        Second: Report "NB UR Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        First.SetMarker(1);
        First.Run();
        Row."Entry No." := 2;
        Row.Insert();
        Second.SetMarker(3);
        Second.Run();
    end;
}

codeunit 50017 "NB UR Codeunit Run Commits"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CodeunitRunCommitsAndSetsUpdate_Refused()
    var
        UpdateReport: Report "NB UR Update";
    begin
        Codeunit.Run(Codeunit::"NB UR Helper");
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50018 "NB UR Procedure Commits"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitAndSetInAProcedure_Refused()
    var
        Helper: Codeunit "NB UR Helper";
        UpdateReport: Report "NB UR Update";
    begin
        Helper.CommitAndSetUpdate();
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50019 "NB UR LockTable"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSetUpdateLockTable_Refused()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        Row.LockTable();
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50020 "NB UR Run In Try"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSetUpdateRunInTry_Refused()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if TryRun() then;
        if GetLastErrorText() <> '' then
            Error(GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryRun()
    var
        UpdateReport: Report "NB UR Update";
    begin
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50021 "NB UR Type After Commit"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SetReportReadCommit_ReadsReport()
    var
        Row: Record "NB UR Row";
        AfterCommit: Text;
    begin
        CurrentTransactionType(TransactionType::Report);
        if Row.FindFirst() then;
        Commit();
        AfterCommit := Format(CurrentTransactionType());
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        if AfterCommit <> 'Report' then
            Error('Type after the Commit: %1', AfterCommit);
    end;
}

codeunit 50022 "NB UR Set After Write"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitWriteSetUpdate_Raises()
    var
        Row: Record "NB UR Row";
    begin
        Commit();
        Row."Entry No." := 1;
        Row.Insert();
        asserterror CurrentTransactionType(TransactionType::Update);
        if GetLastErrorCode() <> 'DB:CannotChangeTransactionType' then
            Error('No error, or another: %1 %2', GetLastErrorCode(), GetLastErrorText());
    end;
}

codeunit 50023 "NB UR Carry Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_LeavesBrowse()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Browse);
    end;

    [Test]
    procedure B_StartsWithBrowse()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        if AtStart <> 'Browse' then
            Error('Type at the start: %1', AtStart);
    end;
}

codeunit 50024 "NB UR Carry Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_LeavesUpdate()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;

    [Test]
    procedure B_UpdateReportAtStart_NoWorld()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
        asserterror Error('trapped');
        if Row.Get(1) then
            Error('The report committed its marker: a transaction world.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 50025 "NB UR Carry Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_LeavesSnapshot()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
    end;

    [Test]
    procedure B_UpdateReportAtStart_Refused()
    var
        UpdateReport: Report "NB UR Update";
    begin
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
    end;
}

codeunit 50026 "NB UR After Refused Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure A_RefusedWhileItRuns()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        if Row.FindLast() then;
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    [Test]
    procedure B_ReadsTheType_Refused()
    begin
        if Format(CurrentTransactionType()) <> 'UpdateNoLocks' then
            Error('Type at the start: %1', CurrentTransactionType());
    end;
}

codeunit 50027 "NB UR Pinned Test Start"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure UpdateReportAtTestStart_World()
    var
        Row: Record "NB UR Row";
        UpdateReport: Report "NB UR Update";
    begin
        UpdateReport.SetMarker(1);
        UpdateReport.Run();
        asserterror Error('trapped');
        if not Row.Get(1) then
            Error('The report did not commit its marker: no transaction world.');
    end;
}
