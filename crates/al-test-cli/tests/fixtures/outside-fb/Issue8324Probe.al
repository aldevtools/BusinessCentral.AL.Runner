// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-currenttransactiontype-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-transactiontype-property
// Scope: in-scope
// local object: the session's transaction type before a TransactionType = Update report runs without its request page, and the
//   type across a Commit, are what is under test
// Fixtures used: shared Assert (60021); own table 97180, reports 97181 and 97182, codeunit 97183, test codeunit 97184
// Probe (al-runner issues 8324 and 8605): the sibling of "Test TxModel Report Run" (60040), which measures an Update report only at
//   the test's start, where the session's type is UpdateNoLocks. Here the type is set first. A trapped Error after the run tells
//   whether the report entered a transaction world: a world commits the report's writes, so they survive the trapped error.
//   * The type is Update (set after a Commit, or at the test's start): no world. The report runs in the caller's transaction, a
//     pending write does not refuse it, and its writes are rolled back with the caller's (TxU03 to TxU06, TxU09, TxU10, TxU18,
//     TxU27 to TxU33). Run and RunModal; nothing, a Get, FindFirst, FindSet and Next, Count and IsEmpty, an Insert, a Modify,
//     a Delete, or a Get and an Insert before the run; two runs in a row. TxU01 and TxU02 are the steps of
//     Library - Manufacturing.RefreshProdOrder (codeunit 97183) around a report shaped like 99001025 "Refresh Production Order".
//   * Any other type (UpdateNoLocks, Snapshot, Report, Browse after a Commit): a world (TxU11, TxU13 to TxU15); a pending Insert
//     under UpdateNoLocks refuses the run (TxU12). With the request page shown the type does not matter (TxU16, TxU17).
//   * A Commit keeps the type the transaction had (TxU07, TxU08, TxU21 to TxU26, TxU34 to TxU38: after a write, a read or no
//     call, and over two Commits). So after a Commit at UpdateNoLocks the report
//     enters a world (TxU07 Run; TxU19 RunModal of the report shaped like Refresh Production Order), and a pending Insert
//     refuses it (TxU08, TxU20).
//   The session's type carries into the next test method, so every test that sets a type ends with
//   Commit() and CurrentTransactionType(TransactionType::UpdateNoLocks), as Library - Manufacturing does.
//   Measured on a local BC 29.1 OnPrem w1 container (40/40 green); BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
table 97180 "TxU Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Touched; Boolean) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

// No request page, TransactionType = Update: writes one marker row.
report 97181 "TxU Update Report"
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
                TxURow: Record "TxU Row";
            begin
                TxURow."Entry No." := MarkerNo;
                TxURow.Insert();
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

// Shaped like report 99001025 "Refresh Production Order": a request page, TransactionType = Update, and a data item the
// caller filters with SetTableView, whose rows the report modifies.
report 97182 "TxU Refresh Report"
{
    ProcessingOnly = true;
    TransactionType = Update;

    dataset
    {
        dataitem(TxURow; "TxU Row")
        {
            DataItemTableView = sorting("Entry No.");
            RequestFilterFields = "Entry No.";

            trigger OnAfterGetRecord()
            begin
                Touched := true;
                Modify();
            end;
        }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                group(Options)
                {
                    field(Direction; Direction)
                    {
                        ApplicationArea = All;
                        Caption = 'Direction';
                        ToolTip = 'Unused.';
                    }
                }
            }
        }
    }

    var
        Direction: Option Forward,Backward;

    procedure InitializeRequest(NewDirection: Option Forward,Backward)
    begin
        Direction := NewDirection;
    end;
}

// The steps of Library - Manufacturing.RefreshProdOrder, around report 97182.
codeunit 97183 "TxU Library"
{
    procedure RefreshRow(var TxURow: Record "TxU Row")
    var
        TmpTxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
        TempTransactionType: TransactionType;
        Direction: Option Forward,Backward;
    begin
        Commit();
        TempTransactionType := CurrentTransactionType;
        CurrentTransactionType(TransactionType::Update);

        Direction := Direction::Forward;
        if TxURow.HasFilter then
            TmpTxURow.CopyFilters(TxURow)
        else begin
            TxURow.Get(TxURow."Entry No.");
            TmpTxURow.SetRange("Entry No.", TxURow."Entry No.");
        end;
        TxURefreshReport.InitializeRequest(Direction);
        TxURefreshReport.SetTableView(TmpTxURow);
        TxURefreshReport.UseRequestPage := false;
        TxURefreshReport.RunModal();

        Commit();
        CurrentTransactionType(TempTransactionType);
    end;
}

// The tests are declaration-ordered and share a codeunit. TxU00 and TxU99 delete this codeunit's rows, without an attribute,
// so the platform commits the deletes at the test boundary: the tests below commit rows on purpose.
codeunit 97184 "TxU Update Report Tx"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    local procedure InsertRow(EntryNo: Integer)
    var
        TxURow: Record "TxU Row";
    begin
        TxURow."Entry No." := EntryNo;
        TxURow.Insert();
    end;

    local procedure RowExists(EntryNo: Integer): Boolean
    var
        TxURow: Record "TxU Row";
    begin
        exit(TxURow.Get(EntryNo));
    end;

    local procedure IsTouched(EntryNo: Integer): Boolean
    var
        TxURow: Record "TxU Row";
    begin
        TxURow.Get(EntryNo);
        exit(TxURow.Touched);
    end;

    local procedure TypeText(): Text
    begin
        exit(Format(CurrentTransactionType()));
    end;

    // Puts the session's transaction type back to the test runner's default: BC carries it into the next test method.
    local procedure RestoreType()
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;

    local procedure TrapAnError()
    begin
        asserterror Error('txu trapped error');
        Assert.ExpectedError('txu trapped error');
    end;

    [Test]
    procedure TxU00_ClearsTheRows()
    var
        TxURow: Record "TxU Row";
    begin
        // The type a test leaves carries into the next one: start this codeunit from the test runner's default.
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        TxURow.DeleteAll();
        Assert.AreEqual(0, TxURow.Count(), 'No row may exist before the tests below write them.');
    end;

    // ----- The steps of Library - Manufacturing.RefreshProdOrder (codeunit 97183) -----

    [Test]
    procedure TxU01_LibrarySteps_GetPath()
    var
        TxURow: Record "TxU Row";
        TxULibrary: Codeunit "TxU Library";
    begin
        InsertRow(97184011);
        TxURow.Get(97184011);
        TxULibrary.RefreshRow(TxURow);
        Assert.IsTrue(IsTouched(97184011), 'The report must have touched the row.');
        Assert.AreEqual('UpdateNoLocks', TypeText(), 'The library sets back the type it read after its first Commit.');
        TrapAnError();
        Assert.IsTrue(IsTouched(97184011), 'The library''s second Commit committed the touch.');
    end;

    [Test]
    procedure TxU02_LibrarySteps_FilterPath()
    var
        TxURow: Record "TxU Row";
        TxULibrary: Codeunit "TxU Library";
    begin
        InsertRow(97184021);
        TxURow.SetRange("Entry No.", 97184021);
        TxULibrary.RefreshRow(TxURow);
        Assert.IsTrue(IsTouched(97184021), 'The report must have touched the row.');
    end;

    // ----- The same steps inline, observed before the library's second Commit -----

    [Test]
    procedure TxU03_CommitSetUpdateGetRunModal_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184031);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.Get(97184031);
        TxURow.SetRange("Entry No.", 97184031);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage := false;
        TxURefreshReport.RunModal();
        Assert.IsTrue(IsTouched(97184031), 'The report must have touched the row.');
        Assert.AreEqual('Update', TypeText(), 'The type after the run.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184031), 'The row committed before the run survives.');
        Assert.IsFalse(IsTouched(97184031), 'The trapped error rolls back the report''s touch: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU04_CommitSetUpdateRunModalNoRead_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184041);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.SetRange("Entry No.", 97184041);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage := false;
        TxURefreshReport.RunModal();
        Assert.IsTrue(IsTouched(97184041), 'The report must have touched the row.');
        TrapAnError();
        Assert.IsFalse(IsTouched(97184041), 'The trapped error rolls back the report''s touch: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU05_CommitSetUpdateGetRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184051);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.Get(97184051);
        TxURow.SetRange("Entry No.", 97184051);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage(false);
        TxURefreshReport.Run();
        Assert.IsTrue(IsTouched(97184051), 'The report must have touched the row.');
        TrapAnError();
        Assert.IsFalse(IsTouched(97184051), 'The trapped error rolls back the report''s touch: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU06_CommitSetUpdateInsertRun_NoWorld()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        InsertRow(97184061);
        TxUUpdateReport.SetMarker(97184062);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184062), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184061), 'The pending write is rolled back.');
        Assert.IsFalse(RowExists(97184062), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    // ----- After a Commit without a set: the type is the one before the Commit, UpdateNoLocks here -----

    [Test]
    procedure TxU07_CommitNoSetRun_World()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        Assert.AreEqual('UpdateNoLocks', TypeText(), 'The type after the Commit.');
        TxUUpdateReport.SetMarker(97184071);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184071), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184071), 'The report committed its marker: a transaction world.');
    end;

    [Test]
    procedure TxU08_CommitNoSetInsertRun_Refused()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        InsertRow(97184081);
        TxUUpdateReport.SetMarker(97184082);
        asserterror TxUUpdateReport.Run();
        Assert.ExpectedError('An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.');
        Assert.IsFalse(RowExists(97184082), 'A refused report must not have run its body.');
        Assert.IsFalse(RowExists(97184081), 'The trapped refusal rolls back the pending write.');
    end;

    // ----- The set at the test's start, without a Commit -----

    [Test]
    procedure TxU09_SetUpdateAtStartRun_NoWorld()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        TxUUpdateReport.SetMarker(97184091);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184091), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184091), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU10_SetUpdateAtStartInsertRun_NoWorld()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        InsertRow(97184101);
        TxUUpdateReport.SetMarker(97184102);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184102), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184101), 'The pending write is rolled back.');
        Assert.IsFalse(RowExists(97184102), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    // ----- Other types after a Commit -----

    [Test]
    procedure TxU11_CommitSetUpdateNoLocksRun_World()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        TxUUpdateReport.SetMarker(97184111);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184111), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184111), 'The report committed its marker: a transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU12_CommitSetUpdateNoLocksInsertRun_Refused()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        InsertRow(97184121);
        TxUUpdateReport.SetMarker(97184122);
        asserterror TxUUpdateReport.Run();
        Assert.ExpectedError('An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.');
        Assert.IsFalse(RowExists(97184122), 'A refused report must not have run its body.');
        Assert.IsFalse(RowExists(97184121), 'The trapped refusal rolls back the pending write.');
        RestoreType();
    end;

    [Test]
    procedure TxU13_CommitSetSnapshotRun_World()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        TxUUpdateReport.SetMarker(97184131);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184131), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184131), 'The report committed its marker: a transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU14_CommitSetReportRun_World()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Report);
        TxUUpdateReport.SetMarker(97184141);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184141), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184141), 'The report committed its marker: a transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU15_CommitSetBrowseRun_World()
    var
        TxUUpdateReport: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Browse);
        TxUUpdateReport.SetMarker(97184151);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184151), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184151), 'The report committed its marker: a transaction world.');
        RestoreType();
    end;

    // ----- The request page shown after the set -----

    [Test]
    [HandlerFunctions('RefreshRequestPageHandler')]
    procedure TxU16_CommitSetUpdateRequestPageShown_World()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184161);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.SetRange("Entry No.", 97184161);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage := true;
        TxURefreshReport.RunModal();
        Assert.IsTrue(IsTouched(97184161), 'The report must have touched the row.');
        TrapAnError();
        Assert.IsTrue(IsTouched(97184161), 'The report committed its touch: a transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU17_CommitSetUpdateInsertRequestPageShown_Refused()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184171);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        InsertRow(97184172);
        TxURow.SetRange("Entry No.", 97184171);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage := true;
        // No [HandlerFunctions]: the refusal comes before the request page is shown.
        asserterror TxURefreshReport.RunModal();
        Assert.ExpectedError('An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.');
        Assert.IsFalse(IsTouched(97184171), 'A refused report must not have touched the row.');
        Assert.IsFalse(RowExists(97184172), 'The trapped refusal rolls back the pending write.');
        RestoreType();
    end;

    // ----- Two runs after one set -----

    [Test]
    procedure TxU18_CommitSetUpdateTwoRuns_NoWorld()
    var
        First: Report "TxU Update Report";
        Second: Report "TxU Update Report";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        First.SetMarker(97184181);
        First.Run();
        Second.SetMarker(97184182);
        Second.Run();
        Assert.IsTrue(RowExists(97184181) and RowExists(97184182), 'Both reports must have written their markers.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184181), 'The first marker is rolled back: no transaction world.');
        Assert.IsFalse(RowExists(97184182), 'The second marker is rolled back: no transaction world.');
        RestoreType();
    end;

    // ----- The report shaped like Refresh Production Order, RunModal after a Commit without a set -----

    [Test]
    procedure TxU19_CommitNoSetRunModal_World()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184191);
        Commit();
        TxURow.SetRange("Entry No.", 97184191);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage(false);
        TxURefreshReport.RunModal();
        Assert.IsTrue(IsTouched(97184191), 'The report must have touched the row.');
        TrapAnError();
        Assert.IsTrue(IsTouched(97184191), 'The report committed its touch: a transaction world.');
    end;

    [Test]
    procedure TxU20_CommitNoSetInsertRunModal_Refused()
    var
        TxURow: Record "TxU Row";
        TxURefreshReport: Report "TxU Refresh Report";
    begin
        InsertRow(97184201);
        Commit();
        InsertRow(97184202);
        TxURow.SetRange("Entry No.", 97184201);
        TxURefreshReport.SetTableView(TxURow);
        TxURefreshReport.UseRequestPage(false);
        asserterror TxURefreshReport.RunModal();
        Assert.ExpectedError('An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.');
        Assert.IsFalse(IsTouched(97184201), 'A refused report must not have touched the row.');
        Assert.IsFalse(RowExists(97184202), 'The trapped refusal rolls back the pending write.');
    end;

    // ----- The transaction type across a Commit (al-runner issue 8605) -----

    [Test]
    procedure TxU21_TypeAfterCommit_DefaultWrite()
    begin
        InsertRow(97184211);
        Commit();
        Assert.AreEqual('UpdateNoLocks', TypeText(), 'The type after the Commit.');
    end;

    [Test]
    procedure TxU22_TypeAfterCommit_UpdateWrite()
    begin
        CurrentTransactionType(TransactionType::Update);
        InsertRow(97184221);
        Commit();
        Assert.AreEqual('Update', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU23_TypeAfterCommit_SnapshotRead()
    var
        TxURow: Record "TxU Row";
    begin
        CurrentTransactionType(TransactionType::Snapshot);
        if TxURow.FindFirst() then;
        Commit();
        Assert.AreEqual('Snapshot', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU24_TypeAfterCommit_BrowseRead()
    var
        TxURow: Record "TxU Row";
    begin
        CurrentTransactionType(TransactionType::Browse);
        if TxURow.FindFirst() then;
        Commit();
        Assert.AreEqual('Browse', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU25_TypeAfterCommit_ReportRead()
    var
        TxURow: Record "TxU Row";
    begin
        CurrentTransactionType(TransactionType::Report);
        if TxURow.FindFirst() then;
        Commit();
        Assert.AreEqual('Report', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU26_TypeAfterCommit_UpdateNoCall()
    begin
        CurrentTransactionType(TransactionType::Update);
        Commit();
        Assert.AreEqual('Update', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    // ----- Other reads and writes after the Update set (the cells above read with Get and write with Insert) -----

    [Test]
    procedure TxU27_CommitSetUpdateFindFirstRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        InsertRow(97184271);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.SetRange("Entry No.", 97184271);
        TxURow.FindFirst();
        TxUUpdateReport.SetMarker(97184272);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184272), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184272), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU28_CommitSetUpdateFindSetRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        InsertRow(97184281);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.SetRange("Entry No.", 97184281);
        if TxURow.FindSet() then
            repeat
            until TxURow.Next() = 0;
        TxUUpdateReport.SetMarker(97184282);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184282), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184282), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU29_CommitSetUpdateCountRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        InsertRow(97184291);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.SetRange("Entry No.", 97184291);
        Assert.AreEqual(1, TxURow.Count(), 'The row committed before the set.');
        Assert.IsFalse(TxURow.IsEmpty(), 'The row committed before the set.');
        TxUUpdateReport.SetMarker(97184292);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184292), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184292), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU30_CommitSetUpdateModifyRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        InsertRow(97184301);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow."Entry No." := 97184301;
        TxURow.Touched := true;
        TxURow.Modify();
        TxUUpdateReport.SetMarker(97184302);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184302), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(IsTouched(97184301), 'The pending Modify is rolled back.');
        Assert.IsFalse(RowExists(97184302), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU31_CommitSetUpdateDeleteRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        InsertRow(97184311);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow."Entry No." := 97184311;
        TxURow.Delete();
        TxUUpdateReport.SetMarker(97184312);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184312), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsTrue(RowExists(97184311), 'The pending Delete is rolled back.');
        Assert.IsFalse(RowExists(97184312), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU32_CommitSetUpdateGetInsertRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        InsertRow(97184321);
        Commit();
        CurrentTransactionType(TransactionType::Update);
        TxURow.Get(97184321);
        InsertRow(97184322);
        TxUUpdateReport.SetMarker(97184323);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184323), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184322), 'The pending Insert is rolled back.');
        Assert.IsFalse(RowExists(97184323), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU33_SetUpdateAtStartFindFirstRun_NoWorld()
    var
        TxURow: Record "TxU Row";
        TxUUpdateReport: Report "TxU Update Report";
    begin
        CurrentTransactionType(TransactionType::Update);
        if TxURow.FindFirst() then;
        TxUUpdateReport.SetMarker(97184331);
        TxUUpdateReport.Run();
        Assert.IsTrue(RowExists(97184331), 'The report must have written its marker.');
        TrapAnError();
        Assert.IsFalse(RowExists(97184331), 'The report''s marker is rolled back: no transaction world.');
        RestoreType();
    end;

    [Test]
    procedure TxU34_TypeAfterCommit_UpdateNoLocksNoCall()
    begin
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Commit();
        Assert.AreEqual('UpdateNoLocks', TypeText(), 'The type after the Commit.');
    end;

    [Test]
    procedure TxU35_TypeAfterCommit_SnapshotNoCall()
    begin
        CurrentTransactionType(TransactionType::Snapshot);
        Commit();
        Assert.AreEqual('Snapshot', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU36_TypeAfterCommit_BrowseNoCall()
    begin
        CurrentTransactionType(TransactionType::Browse);
        Commit();
        Assert.AreEqual('Browse', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU37_TypeAfterCommit_ReportNoCall()
    begin
        CurrentTransactionType(TransactionType::Report);
        Commit();
        Assert.AreEqual('Report', TypeText(), 'The type after the Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU38_TypeAfterTwoCommits_SnapshotRead()
    var
        TxURow: Record "TxU Row";
    begin
        CurrentTransactionType(TransactionType::Snapshot);
        if TxURow.FindFirst() then;
        Commit();
        if TxURow.FindFirst() then;
        Commit();
        Assert.AreEqual('Snapshot', TypeText(), 'The type after the second Commit.');
        RestoreType();
    end;

    [Test]
    procedure TxU99_ClearsTheRowsAgain()
    var
        TxURow: Record "TxU Row";
    begin
        TxURow.DeleteAll();
        Assert.AreEqual(0, TxURow.Count(), 'The rows the tests above committed must be removable.');
    end;

    [RequestPageHandler]
    procedure RefreshRequestPageHandler(var RequestPage: TestRequestPage "TxU Refresh Report")
    begin
        RequestPage.OK().Invoke();
    end;
}
