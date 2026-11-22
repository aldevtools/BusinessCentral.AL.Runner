// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-currenttransactiontype-method
// Scope: in-scope
// local object: the session's transaction type at the start of a test method, after the test method before it set one
// Fixtures used: shared Assert (60021); table 97180 and report 97181 of record/TxTypeUpdateReport.al; own test codeunit 97185
// Probe (al-runner issue 8606): the type a test method leaves is the type the next test method of the codeunit starts with; the
//   test runner does not put it back to UpdateNoLocks. TxN01 commits, sets Browse and ends with no database call; TxN02 reads it at its
//   start, then sets UpdateNoLocks back after a Commit, as Library - Manufacturing.RefreshProdOrder does after its run. The
//   same for Snapshot (TxN03, TxN04) and Report (TxN05, TxN06), for Browse with a read after the set (TxN07, TxN08), for a
//   Snapshot a Commit kept (TxN09, TxN10), and for Update, under which an Update report at the next test's start enters no
//   transaction world (TxN11, TxN12).
//   Measured on a local BC 29.1 OnPrem w1 container (12/12 green); BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
codeunit 97185 "TxU Type Next Test"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure TxN01_SetsTypeBrowse()
    begin
        // A Commit first: at the first test method of a codeunit the test runner's own transaction has started, and a set
        // to Browse there has no effect.
        Commit();
        CurrentTransactionType(TransactionType::Browse);
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure TxN02_StartsWithThePreviousTestsType()
    var
        TypeAtStart: Text;
    begin
        TypeAtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Browse', TypeAtStart, 'The type at the start of the test after the one that set Browse.');
    end;

    [Test]
    procedure TxN03_SetsTypeSnapshot()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
    end;

    [Test]
    procedure TxN04_StartsWithSnapshot()
    var
        TypeAtStart: Text;
    begin
        TypeAtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Snapshot', TypeAtStart, 'The type at the start of the test after the one that set Snapshot.');
    end;

    [Test]
    procedure TxN05_SetsTypeReport()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Report);
    end;

    [Test]
    procedure TxN06_StartsWithReport()
    var
        TypeAtStart: Text;
    begin
        TypeAtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Report', TypeAtStart, 'The type at the start of the test after the one that set Report.');
    end;

    [Test]
    procedure TxN07_SetsTypeBrowseThenReads()
    var
        Row: Record "TxU Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Browse);
        if Row.FindFirst() then;
    end;

    [Test]
    procedure TxN08_StartsWithBrowseAfterARead()
    var
        TypeAtStart: Text;
    begin
        TypeAtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Browse', TypeAtStart, 'The type at the start of the test after the one that set Browse and read.');
    end;

    [Test]
    procedure TxN09_SetsTypeSnapshotReadsAndCommits()
    var
        Row: Record "TxU Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        if Row.FindFirst() then;
        Commit();
    end;

    [Test]
    procedure TxN10_StartsWithTheSnapshotACommitKept()
    var
        TypeAtStart: Text;
    begin
        TypeAtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Snapshot', TypeAtStart, 'The type at the start of the test after the one whose Commit kept Snapshot.');
    end;

    [Test]
    procedure TxN11_SetsTypeUpdate()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
    end;

    [Test]
    procedure TxN12_StartsWithUpdateAndAnUpdateReportJoinsTheTransaction()
    var
        Row: Record "TxU Row";
        UpdateReport: Report "TxU Update Report";
        TypeAtStart: Text;
    begin
        TypeAtStart := Format(CurrentTransactionType());
        UpdateReport.SetMarker(97185121);
        UpdateReport.Run();
        Assert.IsTrue(Row.Get(97185121), 'The report must have written its marker.');
        asserterror Error('txn trapped error');
        Assert.ExpectedError('txn trapped error');
        Assert.IsFalse(Row.Get(97185121), 'The report''s marker is rolled back: no transaction world.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Update', TypeAtStart, 'The type at the start of the test after the one that set Update.');
    end;
}
