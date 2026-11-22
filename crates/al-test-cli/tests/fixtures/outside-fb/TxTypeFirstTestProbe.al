// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-currenttransactiontype-method
// Scope: in-scope
// local object: the session's transaction type at the start of a test codeunit's first test method
// Fixtures used: shared Assert (60021); table 97210 of record/TxTypeSetAfterCall.al; own test codeunits 97212 to 97231
// Probe (review of al-runner pull request 8607): at the start of a codeunit's first test method the test runner's transaction
//   has started. Each codeunit's first test method A sets the type after nothing, a read, an asserterror (with and without a
//   write in it) or a Commit; its second test method B reads the type it starts with.
//   * After nothing or a read, a set to Update or Snapshot raises DB:CannotChangeTransactionType ("... from UpdateNoLocks to
//     ..."), and a set to Browse or Report has no effect: the type stays UpdateNoLocks, and B starts with it. This holds
//     when another codeunit ran before in the same run, as here: run as the only codeunit of a run, "TxF None Update" fails,
//     because the set to Update takes effect and B starts with Update.
//   * After an asserterror, which rolls the transaction back, and after a Commit, the set takes effect, and B starts with the
//     type set; also as the only codeunit of a run ("TxF AssertError Update", "TxF AssertErrorWrite Snapshot").
//   Measured on a local BC 29.1 OnPrem w1 container (40/40 green, twice, after the other codeunits of the app);
//   BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
codeunit 97212 "TxF None Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_None_SetUpdate()
    begin
        Assert.IsFalse(TrySet(TransactionType::Update), 'The set raises.');
        Assert.AreEqual('DB:CannotChangeTransactionType', GetLastErrorCode(), 'The error code of the set.');
        Assert.AreEqual(StrSubstNo(CannotChangeTxt, 'UpdateNoLocks', 'Update'), GetLastErrorText(), 'The error text of the set.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97213 "TxF None Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_None_SetSnapshot()
    begin
        Assert.IsFalse(TrySet(TransactionType::Snapshot), 'The set raises.');
        Assert.AreEqual('DB:CannotChangeTransactionType', GetLastErrorCode(), 'The error code of the set.');
        Assert.AreEqual(StrSubstNo(CannotChangeTxt, 'UpdateNoLocks', 'Snapshot'), GetLastErrorText(), 'The error text of the set.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97214 "TxF None Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_None_SetBrowse()
    begin
        Assert.IsTrue(TrySet(TransactionType::Browse), 'The set does not raise.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97215 "TxF None Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_None_SetReport()
    begin
        Assert.IsTrue(TrySet(TransactionType::Report), 'The set does not raise.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97216 "TxF Read Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Read_SetUpdate()
    var
        TxSRow: Record "TxS Row";
    begin
        if TxSRow.FindFirst() then;
        Assert.IsFalse(TrySet(TransactionType::Update), 'The set raises.');
        Assert.AreEqual('DB:CannotChangeTransactionType', GetLastErrorCode(), 'The error code of the set.');
        Assert.AreEqual(StrSubstNo(CannotChangeTxt, 'UpdateNoLocks', 'Update'), GetLastErrorText(), 'The error text of the set.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97217 "TxF Read Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Read_SetSnapshot()
    var
        TxSRow: Record "TxS Row";
    begin
        if TxSRow.FindFirst() then;
        Assert.IsFalse(TrySet(TransactionType::Snapshot), 'The set raises.');
        Assert.AreEqual('DB:CannotChangeTransactionType', GetLastErrorCode(), 'The error code of the set.');
        Assert.AreEqual(StrSubstNo(CannotChangeTxt, 'UpdateNoLocks', 'Snapshot'), GetLastErrorText(), 'The error text of the set.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97218 "TxF Read Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Read_SetBrowse()
    var
        TxSRow: Record "TxS Row";
    begin
        if TxSRow.FindFirst() then;
        Assert.IsTrue(TrySet(TransactionType::Browse), 'The set does not raise.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97219 "TxF Read Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Read_SetReport()
    var
        TxSRow: Record "TxS Row";
    begin
        if TxSRow.FindFirst() then;
        Assert.IsTrue(TrySet(TransactionType::Report), 'The set does not raise.');
        Assert.AreEqual('UpdateNoLocks', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdateNoLocks()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('UpdateNoLocks', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97220 "TxF AssertError Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertError_SetUpdate()
    begin
        asserterror Error('Rolled back.');
        Assert.IsTrue(TrySet(TransactionType::Update), 'The set does not raise.');
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdate()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Update', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97221 "TxF AssertError Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertError_SetSnapshot()
    begin
        asserterror Error('Rolled back.');
        Assert.IsTrue(TrySet(TransactionType::Snapshot), 'The set does not raise.');
        Assert.AreEqual('Snapshot', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithSnapshot()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Snapshot', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97222 "TxF AssertError Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertError_SetBrowse()
    begin
        asserterror Error('Rolled back.');
        Assert.IsTrue(TrySet(TransactionType::Browse), 'The set does not raise.');
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithBrowse()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Browse', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97223 "TxF AssertError Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertError_SetReport()
    begin
        asserterror Error('Rolled back.');
        Assert.IsTrue(TrySet(TransactionType::Report), 'The set does not raise.');
        Assert.AreEqual('Report', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithReport()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Report', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97224 "TxF AssertErrorWrite Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertErrorWrite_SetUpdate()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97224;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        Assert.IsTrue(TrySet(TransactionType::Update), 'The set does not raise.');
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdate()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Update', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97225 "TxF AssertErrorWrite Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertErrorWrite_SetSnapshot()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97225;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        Assert.IsTrue(TrySet(TransactionType::Snapshot), 'The set does not raise.');
        Assert.AreEqual('Snapshot', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithSnapshot()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Snapshot', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97226 "TxF AssertErrorWrite Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertErrorWrite_SetBrowse()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97226;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        Assert.IsTrue(TrySet(TransactionType::Browse), 'The set does not raise.');
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithBrowse()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Browse', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97227 "TxF AssertErrorWrite Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_AssertErrorWrite_SetReport()
    var
        TxSRow: Record "TxS Row";
    begin
        asserterror begin
            TxSRow."Entry No." := 97227;
            TxSRow.Insert();
            Error('Rolled back.');
        end;
        Assert.IsTrue(TrySet(TransactionType::Report), 'The set does not raise.');
        Assert.AreEqual('Report', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithReport()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Report', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97228 "TxF Commit Update"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Commit_SetUpdate()
    begin
        Commit();
        Assert.IsTrue(TrySet(TransactionType::Update), 'The set does not raise.');
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithUpdate()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Update', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97229 "TxF Commit Snapshot"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Commit_SetSnapshot()
    begin
        Commit();
        Assert.IsTrue(TrySet(TransactionType::Snapshot), 'The set does not raise.');
        Assert.AreEqual('Snapshot', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithSnapshot()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Snapshot', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97230 "TxF Commit Browse"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Commit_SetBrowse()
    begin
        Commit();
        Assert.IsTrue(TrySet(TransactionType::Browse), 'The set does not raise.');
        Assert.AreEqual('Browse', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithBrowse()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Browse', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}

codeunit 97231 "TxF Commit Report"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;

    [Test]
    procedure A_Commit_SetReport()
    begin
        Commit();
        Assert.IsTrue(TrySet(TransactionType::Report), 'The set does not raise.');
        Assert.AreEqual('Report', Format(CurrentTransactionType()), 'The type after the set.');
    end;

    [Test]
    procedure B_StartsWithReport()
    var
        AtStart: Text;
    begin
        AtStart := Format(CurrentTransactionType());
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        Assert.AreEqual('Report', AtStart, 'The type at the start of the next test method.');
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}
