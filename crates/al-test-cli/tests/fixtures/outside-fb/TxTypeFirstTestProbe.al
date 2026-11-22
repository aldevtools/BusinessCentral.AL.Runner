// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-currenttransactiontype-method
// Scope: in-scope
// local object: the session's transaction type in a test codeunit's first test method, after a rollback or a Commit
// Fixtures used: shared Assert (60021); table 97210 of record/TxTypeSetAfterCall.al; own test codeunits 97212 to 97223
// Probe (review of al-runner pull request 8607): a rollback in a codeunit's first test method ends the transaction the test
//   runner may have started before it, as a Commit does. Each codeunit has one test method: an asserterror (with and without
//   a write in it) or a Commit, then a set to Update, Snapshot, Browse or Report, which takes effect. Each test sets
//   UpdateNoLocks back after a Commit, since a test that ends at Snapshot, Browse or Report makes the next write of the
//   test runner fail on some runners.
//   Skipped: a set at the start of the first test method with nothing or a read before it. Whether the test runner's
//   transaction has started there depends on the runner and on what ran before: on a local container a set to Update
//   raised DB:CannotChangeTransactionType after another codeunit and took effect when the codeunit ran alone, and on the
//   BC 27.5 leg of this repository's CI it took effect.
//   Measured on a local BC 29.1 OnPrem w1 container; BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
codeunit 97212 "TxF AssertError Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertError_SetUpdate_TakesEffect()
    begin
        asserterror Error('Rolled back.');
        CurrentTransactionType(TransactionType::Update);
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97213 "TxF AssertError Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertError_SetSnapshot_TakesEffect()
    begin
        asserterror Error('Rolled back.');
        CurrentTransactionType(TransactionType::Snapshot);
        Assert.AreEqual('Snapshot', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97214 "TxF AssertError Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertError_SetBrowse_TakesEffect()
    begin
        asserterror Error('Rolled back.');
        CurrentTransactionType(TransactionType::Browse);
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97215 "TxF AssertError Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertError_SetReport_TakesEffect()
    begin
        asserterror Error('Rolled back.');
        CurrentTransactionType(TransactionType::Report);
        Assert.AreEqual('Report', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97216 "TxF AssertErrorWrite Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertErrorWrite_SetUpdate_TakesEffect()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97216;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        CurrentTransactionType(TransactionType::Update);
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97217 "TxF AssertErrorWrite Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertErrorWrite_SetSnapshot_TakesEffect()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97217;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        CurrentTransactionType(TransactionType::Snapshot);
        Assert.AreEqual('Snapshot', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97218 "TxF AssertErrorWrite Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertErrorWrite_SetBrowse_TakesEffect()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97218;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        CurrentTransactionType(TransactionType::Browse);
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97219 "TxF AssertErrorWrite Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure AssertErrorWrite_SetReport_TakesEffect()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97219;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        CurrentTransactionType(TransactionType::Report);
        Assert.AreEqual('Report', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97220 "TxF Commit Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure Commit_SetUpdate_TakesEffect()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Update);
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97221 "TxF Commit Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure Commit_SetSnapshot_TakesEffect()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Snapshot);
        Assert.AreEqual('Snapshot', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97222 "TxF Commit Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure Commit_SetBrowse_TakesEffect()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Browse);
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}

codeunit 97223 "TxF Commit Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    [Test]
    procedure Commit_SetReport_TakesEffect()
    begin
        Commit();
        CurrentTransactionType(TransactionType::Report);
        Assert.AreEqual('Report', Format(CurrentTransactionType()), 'The type after the set.');
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
    end;
}
