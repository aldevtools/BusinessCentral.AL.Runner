// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/database/database-currenttransactiontype-method
// Scope: in-scope
// local object: the session's transaction type once a database call has started the transaction, and a write at a type that
//   does not write
// Fixtures used: shared Assert (60021); own table 97210 and test codeunit 97211
// Probe (al-runner issues 8707, 8708 and 8709; review of al-runner pull request 8607): what CurrentTransactionType(X) does once the transaction has started, by the
//   type it has and the call that started it, and what a write does at a type that does not write. Each cell commits, sets
//   the type before any database call (that takes effect), runs the call, then sets the type again or writes.
//   * After a write at UpdateNoLocks a set to Update or Snapshot raises DB:CannotChangeTransactionType, and a set to
//     UpdateNoLocks, Browse or Report has no effect (TxS01 to TxS05). After a write at Update no set has an effect (TxS06 to
//     TxS10). A set to the type the transaction has has no effect after a read as well (TxS11 to TxS15).
//   * Snapshot, Browse and Report do not write: Insert, Modify, Delete, DeleteAll (also of no row) and ModifyAll raise
//     DB:TransActionIsReadOnly, the type stays, and the row is unchanged (TxS16 to TxS30, TxS34). The error is trapped and
//     rolled back, and a set after it takes effect (TxS31 to TxS33). A temporary record writes (TxS35).
//   Skipped: a set to another type after a read. Whether the read has started the transaction depends on whether the server
//   answered it from its data cache: on one container the same read and set raised in one run and took effect in the next.
//   Measured on a local BC 29.1 OnPrem w1 container (35/35 green, twice); BC 27.5, 28.5 and 29.0 are left to CI.
// BC versions: 27, 28, 29
table 97210 "TxS Row"
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

codeunit 97211 "TxS Set After Call"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        CannotChangeTxt: Label 'The transaction type cannot be changed after a transaction has started.\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\The error occurred because the transaction type was changed from %1 to %2.', Locked = true;
        ReadOnlyTxt: Label 'Write operations in the database (INSERT, DELETE, and MODIFY) are not allowed for the selected transaction type.', Locked = true;

    [Test]
    procedure TxS01_NoLocksWrite_SetUpdateNoLocks_NoEffect()
    begin
        SetAfterCall(TransactionType::UpdateNoLocks, true, TransactionType::UpdateNoLocks, 1, false, 'UpdateNoLocks');
    end;

    [Test]
    procedure TxS02_NoLocksWrite_SetUpdate_Raises()
    begin
        SetAfterCall(TransactionType::UpdateNoLocks, true, TransactionType::Update, 2, true, 'UpdateNoLocks');
    end;

    [Test]
    procedure TxS03_NoLocksWrite_SetSnapshot_Raises()
    begin
        SetAfterCall(TransactionType::UpdateNoLocks, true, TransactionType::Snapshot, 3, true, 'UpdateNoLocks');
    end;

    [Test]
    procedure TxS04_NoLocksWrite_SetBrowse_NoEffect()
    begin
        SetAfterCall(TransactionType::UpdateNoLocks, true, TransactionType::Browse, 4, false, 'UpdateNoLocks');
    end;

    [Test]
    procedure TxS05_NoLocksWrite_SetReport_NoEffect()
    begin
        SetAfterCall(TransactionType::UpdateNoLocks, true, TransactionType::Report, 5, false, 'UpdateNoLocks');
    end;

    [Test]
    procedure TxS06_UpdateWrite_SetUpdateNoLocks_NoEffect()
    begin
        SetAfterCall(TransactionType::Update, true, TransactionType::UpdateNoLocks, 6, false, 'Update');
    end;

    [Test]
    procedure TxS07_UpdateWrite_SetUpdate_NoEffect()
    begin
        SetAfterCall(TransactionType::Update, true, TransactionType::Update, 7, false, 'Update');
    end;

    [Test]
    procedure TxS08_UpdateWrite_SetSnapshot_NoEffect()
    begin
        SetAfterCall(TransactionType::Update, true, TransactionType::Snapshot, 8, false, 'Update');
    end;

    [Test]
    procedure TxS09_UpdateWrite_SetBrowse_NoEffect()
    begin
        SetAfterCall(TransactionType::Update, true, TransactionType::Browse, 9, false, 'Update');
    end;

    [Test]
    procedure TxS10_UpdateWrite_SetReport_NoEffect()
    begin
        SetAfterCall(TransactionType::Update, true, TransactionType::Report, 10, false, 'Update');
    end;

    [Test]
    procedure TxS11_UpdateNoLocksRead_SetUpdateNoLocks_NoEffect()
    begin
        SetAfterCall(TransactionType::UpdateNoLocks, false, TransactionType::UpdateNoLocks, 11, false, 'UpdateNoLocks');
    end;

    [Test]
    procedure TxS12_UpdateRead_SetUpdate_NoEffect()
    begin
        SetAfterCall(TransactionType::Update, false, TransactionType::Update, 12, false, 'Update');
    end;

    [Test]
    procedure TxS13_SnapshotRead_SetSnapshot_NoEffect()
    begin
        SetAfterCall(TransactionType::Snapshot, false, TransactionType::Snapshot, 13, false, 'Snapshot');
    end;

    [Test]
    procedure TxS14_BrowseRead_SetBrowse_NoEffect()
    begin
        SetAfterCall(TransactionType::Browse, false, TransactionType::Browse, 14, false, 'Browse');
    end;

    [Test]
    procedure TxS15_ReportRead_SetReport_NoEffect()
    begin
        SetAfterCall(TransactionType::Report, false, TransactionType::Report, 15, false, 'Report');
    end;

    [Test]
    procedure TxS16_Snapshot_Insert_Raises()
    begin
        WriteAtType(TransactionType::Snapshot, 'Insert', 16);
    end;

    [Test]
    procedure TxS17_Snapshot_Modify_Raises()
    begin
        WriteAtType(TransactionType::Snapshot, 'Modify', 17);
    end;

    [Test]
    procedure TxS18_Snapshot_Delete_Raises()
    begin
        WriteAtType(TransactionType::Snapshot, 'Delete', 18);
    end;

    [Test]
    procedure TxS19_Snapshot_DeleteAll_Raises()
    begin
        WriteAtType(TransactionType::Snapshot, 'DeleteAll', 19);
    end;

    [Test]
    procedure TxS20_Snapshot_ModifyAll_Raises()
    begin
        WriteAtType(TransactionType::Snapshot, 'ModifyAll', 20);
    end;

    [Test]
    procedure TxS21_Browse_Insert_Raises()
    begin
        WriteAtType(TransactionType::Browse, 'Insert', 21);
    end;

    [Test]
    procedure TxS22_Browse_Modify_Raises()
    begin
        WriteAtType(TransactionType::Browse, 'Modify', 22);
    end;

    [Test]
    procedure TxS23_Browse_Delete_Raises()
    begin
        WriteAtType(TransactionType::Browse, 'Delete', 23);
    end;

    [Test]
    procedure TxS24_Browse_DeleteAll_Raises()
    begin
        WriteAtType(TransactionType::Browse, 'DeleteAll', 24);
    end;

    [Test]
    procedure TxS25_Browse_ModifyAll_Raises()
    begin
        WriteAtType(TransactionType::Browse, 'ModifyAll', 25);
    end;

    [Test]
    procedure TxS26_Report_Insert_Raises()
    begin
        WriteAtType(TransactionType::Report, 'Insert', 26);
    end;

    [Test]
    procedure TxS27_Report_Modify_Raises()
    begin
        WriteAtType(TransactionType::Report, 'Modify', 27);
    end;

    [Test]
    procedure TxS28_Report_Delete_Raises()
    begin
        WriteAtType(TransactionType::Report, 'Delete', 28);
    end;

    [Test]
    procedure TxS29_Report_DeleteAll_Raises()
    begin
        WriteAtType(TransactionType::Report, 'DeleteAll', 29);
    end;

    [Test]
    procedure TxS30_Report_ModifyAll_Raises()
    begin
        WriteAtType(TransactionType::Report, 'ModifyAll', 30);
    end;

    [Test]
    procedure TxS31_Snapshot_TrappedWrite_SetUpdate_TakesEffect()
    begin
        SetAfterTrappedWrite(TransactionType::Snapshot, 31);
    end;

    [Test]
    procedure TxS32_Browse_TrappedWrite_SetUpdate_TakesEffect()
    begin
        SetAfterTrappedWrite(TransactionType::Browse, 32);
    end;

    [Test]
    procedure TxS33_Report_TrappedWrite_SetUpdate_TakesEffect()
    begin
        SetAfterTrappedWrite(TransactionType::Report, 33);
    end;

    [Test]
    procedure TxS34_Snapshot_DeleteAllNoRows_Raises()
    begin
        WriteAtType(TransactionType::Snapshot, 'DeleteAllNoRows', 34);
    end;

    [Test]
    procedure TxS35_Browse_TemporaryInsert_Writes()
    begin
        TemporaryInsertAtType(TransactionType::Browse, 35);
    end;

    // Ends the transaction, sets UpdateNoLocks and deletes the rows of the cell, so a cell starts as the one before it left.
    local procedure Reset(EntryNo: Integer)
    var
        TxSRow: Record "TxS Row";
    begin
        Commit();
        CurrentTransactionType(TransactionType::UpdateNoLocks);
        TxSRow.SetRange("Entry No.", EntryNo, EntryNo + 1);
        TxSRow.DeleteAll();
        Commit();
    end;

    // A Commit, a set to Before, then an Insert (Write) or a FindFirst of a row the cell inserted before the Commit, then a set
    // to Next: whether it raises, and the type after it.
    local procedure SetAfterCall(Before: TransactionType; Write: Boolean; Next: TransactionType; EntryNo: Integer; Raises: Boolean; TypeAfter: Text)
    var
        TxSRow: Record "TxS Row";
    begin
        Reset(EntryNo);
        if not Write then begin
            TxSRow."Entry No." := EntryNo;
            TxSRow.Insert();
            Commit();
        end;
        CurrentTransactionType(Before);
        if Write then begin
            TxSRow."Entry No." := EntryNo;
            TxSRow.Insert();
        end else
            TxSRow.FindFirst();
        Assert.AreEqual(Format(Before), Format(CurrentTransactionType()), 'The type after the database call.');
        if Raises then begin
            Assert.IsFalse(TrySet(Next), 'The set raises.');
            Assert.AreEqual('DB:CannotChangeTransactionType', GetLastErrorCode(), 'The error code of the set.');
            Assert.AreEqual(StrSubstNo(CannotChangeTxt, Format(Before), Format(Next)), GetLastErrorText(), 'The error text of the set.');
        end else
            Assert.IsTrue(TrySet(Next), 'The set does not raise.');
        Assert.AreEqual(TypeAfter, Format(CurrentTransactionType()), 'The type after the set.');
        Reset(EntryNo);
    end;

    // A Commit, a row, a set to Before, then a write of the row: it raises, the type stays, and the row is unchanged.
    local procedure WriteAtType(Before: TransactionType; Op: Text; EntryNo: Integer)
    var
        TxSRow: Record "TxS Row";
    begin
        Reset(EntryNo);
        TxSRow."Entry No." := EntryNo;
        TxSRow.Insert();
        Commit();
        CurrentTransactionType(Before);
        TxSRow.Get(EntryNo);
        asserterror case Op of
            'Insert':
                begin
                    TxSRow."Entry No." := EntryNo + 1;
                    TxSRow.Insert();
                end;
            'Modify':
                begin
                    TxSRow.Touched := true;
                    TxSRow.Modify();
                end;
            'Delete':
                TxSRow.Delete();
            'DeleteAll':
                begin
                    TxSRow.SetRange("Entry No.", EntryNo);
                    TxSRow.DeleteAll();
                end;
            'DeleteAllNoRows':
                begin
                    TxSRow.SetRange("Entry No.", -1);
                    TxSRow.DeleteAll();
                end;
            'ModifyAll':
                begin
                    TxSRow.SetRange("Entry No.", EntryNo);
                    TxSRow.ModifyAll(Touched, true);
                end;
        end;
        Assert.AreEqual('DB:TransActionIsReadOnly', GetLastErrorCode(), 'The error code of the write.');
        Assert.AreEqual(ReadOnlyTxt, GetLastErrorText(), 'The error text of the write.');
        Assert.AreEqual(Format(Before), Format(CurrentTransactionType()), 'The type after the write.');
        TxSRow.Reset();
        TxSRow.SetRange("Entry No.", EntryNo, EntryNo + 1);
        Assert.AreEqual(1, TxSRow.Count(), 'The rows after the write.');
        TxSRow.FindFirst();
        Assert.IsFalse(TxSRow.Touched, 'The row after the write.');
        Reset(EntryNo);
    end;

    // After a write the type refuses, which asserterror traps and rolls back, a set to Update takes effect.
    local procedure SetAfterTrappedWrite(Before: TransactionType; EntryNo: Integer)
    var
        TxSRow: Record "TxS Row";
    begin
        Reset(EntryNo);
        CurrentTransactionType(Before);
        TxSRow."Entry No." := EntryNo;
        asserterror TxSRow.Insert();
        Assert.AreEqual('DB:TransActionIsReadOnly', GetLastErrorCode(), 'The error code of the write.');
        Assert.IsTrue(TrySet(TransactionType::Update), 'The set does not raise.');
        Assert.AreEqual('Update', Format(CurrentTransactionType()), 'The type after the set.');
        Reset(EntryNo);
    end;

    // A temporary record holds no database row: its Insert writes at a type that refuses database writes.
    local procedure TemporaryInsertAtType(Before: TransactionType; EntryNo: Integer)
    var
        TempTxSRow: Record "TxS Row" temporary;
    begin
        Reset(EntryNo);
        CurrentTransactionType(Before);
        TempTxSRow."Entry No." := EntryNo;
        TempTxSRow.Insert();
        Assert.AreEqual(1, TempTxSRow.Count(), 'The temporary rows.');
        Assert.AreEqual(Format(Before), Format(CurrentTransactionType()), 'The type after the temporary Insert.');
        Reset(EntryNo);
    end;

    [TryFunction]
    local procedure TrySet(NewType: TransactionType)
    begin
        CurrentTransactionType(NewType);
    end;
}
