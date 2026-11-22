// Neighbors of al-runner issue 8721: a copy of a record (`Copy := Src`) whose key is moved to a row that exists, whose guarded
//   Insert(true) fails, then TransferFields and Modify(true). Whether that Modify raises DB:RecordChanged depends on whether the
//   open transaction had written or locked the table before the buffer's row version reached it (corpus PR 1523,
//   record/crud/TestRecordFailedInsertMovedKey.al). Each test is one shape next to the measured ones: the measured shapes
//   expect BC's answer, the shapes no cell measures expect main's raise (and the head refuses them). Invented tables only.

table 97720 "NB Upsert Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Name; Text[50]) { }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

table 97721 "NB Upsert Side"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[20]) { }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

table 97723 "NB Upsert Hooked"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Name; Text[50]) { }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }

    trigger OnModify()
    begin
        Name := CopyStr(Name, 1, 50);
    end;
}

codeunit 97722 "NB Upsert Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;
        NoErrorTxt: Label 'NB_UPSERT_NO_ERROR', Locked = true;

    local procedure Clean()
    var
        Row: Record "NB Upsert Row";
        Side: Record "NB Upsert Side";
        Hooked: Record "NB Upsert Hooked";
    begin
        Row.DeleteAll();
        Side.DeleteAll();
        Hooked.DeleteAll();
        Commit();
    end;

    local procedure AddRow(NewCode: Code[20]; NewName: Text[50])
    var
        Row: Record "NB Upsert Row";
    begin
        Row.Code := NewCode;
        Row.Name := NewName;
        Row.Insert();
    end;

    local procedure CopyTo(var Src: Record "NB Upsert Row"; ToCode: Code[20])
    var
        Copy: Record "NB Upsert Row";
    begin
        Copy := Src;
        Copy.Code := ToCode;
        if not Copy.Insert(true) then begin
            Copy.TransferFields(Src, false);
            Copy.Modify(true);
        end;
    end;

    // What the asserterror block just before it did: 'written', or the error code.
    local procedure Outcome(): Text
    begin
        if GetLastErrorText() = NoErrorTxt then
            exit('written');
        exit(GetLastErrorCode());
    end;

    // The 2nd upsert inside asserterror; returns 'written' or the error code, and rolls the cell back.
    local procedure SecondUpsert(var Src: Record "NB Upsert Row"): Text
    begin
        Src.Name := 'second';
        asserterror begin
            CopyTo(Src, 'TARGET');
            Error(NoErrorTxt);
        end;
        exit(Outcome());
    end;

    [Test]
    procedure CommittedSeed_GetBeforeAnyWrite_Raises()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a read before any write');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_Get_Written()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a read after the seed Insert of the open transaction');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_Get_Unguarded_Written()
    var
        Src: Record "NB Upsert Row";
        Stored: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Src.Name := 'second';
        CopyTo(Src, 'TARGET');
        Stored.Get('TARGET');
        Assert.AreEqual('second', Stored.Name, 'the 2nd upsert outside asserterror');
        Clean();
    end;

    [Test]
    procedure SameTableOtherRowInserted_ThenGet_Written()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        AddRow('OTHER', 'other');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a read after an Insert of another row of the table');
        Clean();
    end;

    [Test]
    procedure SourceModifiedBeforeRead_Written()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Other.Get('SOURCE');
        Other.Name := 'modified';
        Other.Modify();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a read after a Modify of the row itself');
        Clean();
    end;

    [Test]
    procedure OtherTableInserted_ThenGet_Raises()
    var
        Src: Record "NB Upsert Row";
        Side: Record "NB Upsert Side";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Side.Code := 'S';
        Side.Insert();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a read after a write of another table only');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_Get_CommitAfterRead_Written()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        Commit();
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a Commit after the read');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_Get_TwoCommitsAfterRead()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        Commit();
        CopyTo(Src, 'TARGET');
        Commit();
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'two Commits after the read');
        Clean();
    end;

    [Test]
    procedure OwnInsertOfAnotherKey_Written()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.Get('SOURCE');
        Copy := Src;
        Copy.Code := 'OWN';
        Copy.Insert(true);
        CopyTo(Src, 'TARGET');
        asserterror begin
            Copy.Code := 'TARGET';
            if not Copy.Insert(true) then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'a version from the buffer''s own Insert');
        Clean();
    end;

    [Test]
    procedure OwnInsertThenCommit_Written()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.Get('SOURCE');
        Copy := Src;
        Copy.Code := 'OWN';
        Copy.Insert(true);
        Commit();
        CopyTo(Src, 'TARGET');
        asserterror begin
            Copy.Code := 'TARGET';
            if not Copy.Insert(true) then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'a Commit after the buffer''s own Insert');
        Clean();
    end;

    [Test]
    procedure OwnInsertRolledBack()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.Get('SOURCE');
        Copy := Src;
        Copy.Code := 'OWN';
        asserterror begin
            Copy.Insert(true);
            Error('NB_ROLLBACK');
        end;
        AddRow('TARGET', 'late');
        Copy.Code := 'TARGET';
        asserterror begin
            if not Copy.Insert(true) then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'the buffer''s own Insert rolled back');
        Clean();
    end;

    [Test]
    procedure OwnModifyAfterGet_OtherKey_Written()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.Get('SOURCE');
        Src.Name := 'own';
        Src.Modify();
        Other.Code := 'TARGET';
        Other.Insert();
        asserterror begin
            Src.Code := 'TARGET';
            if not Src.Insert() then
                Src.Modify();
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'a version from the buffer''s own Modify of another row');
        Clean();
    end;

    [Test]
    procedure SameTableWritten_ThenFindFirst_Written()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        AddRow('OTHER', 'other');
        Src.SetRange(Code, 'SOURCE');
        Src.FindFirst();
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a FindFirst after a write of the table');
        Clean();
    end;

    [Test]
    procedure SameTableLocked_ThenGet_Written()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.LockTable();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a Get after a LockTable of the table');
        Clean();
    end;

    [Test]
    procedure SameTableRowDeleted_ThenGet_Written()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        AddRow('OTHER', 'other');
        Commit();
        Other.Get('OTHER');
        Other.Delete();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('written', SecondUpsert(Src), 'a Get after a Delete of another row of the table');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_SameKey_ChangedElsewhere_Written()
    var
        R: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        R.Get('SOURCE');
        Other.Get('SOURCE');
        Other.Name := 'elsewhere';
        Other.Modify();
        R.Name := 'mine';
        asserterror begin
            if not R.Insert() then
                R.Modify();
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'the same key, the row changed after the read');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_MovedKey_TargetModifiedAfterInsert_Written()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Other.Get('TARGET');
        Other.Name := 'between';
        Other.Modify();
        Assert.AreEqual('written', SecondUpsert(Src), 'a moved key onto a row modified after its Insert');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_SourceModifiedBetweenCalls_Written()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Other.Get('SOURCE');
        Other.Name := 'between';
        Other.Modify();
        Assert.AreEqual('written', SecondUpsert(Src), 'the source row modified between the two calls');
        Clean();
    end;

    [Test]
    procedure OwnInsertThenOwnDelete()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.Get('SOURCE');
        Copy := Src;
        Copy.Code := 'OWN';
        Copy.Insert(true);
        Copy.Delete();
        CopyTo(Src, 'TARGET');
        asserterror begin
            Copy.Code := 'TARGET';
            if not Copy.Insert(true) then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'the buffer deleted its own inserted row');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_OtherTableWrittenAfterRead()
    var
        Src: Record "NB Upsert Row";
        Side: Record "NB Upsert Side";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Side.Code := 'S';
        Side.Insert();
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a write of another table after the read');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_ModifyAllAfterRead()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        AddRow('OTHER', 'other');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Other.SetRange(Code, 'OTHER');
        Other.ModifyAll(Name, 'all');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a ModifyAll of the table after the read');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_FailedInsertTwice()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
        Done: Boolean;
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        asserterror begin
            Copy := Src;
            Copy.Code := 'TARGET';
            Done := Copy.Insert(true);
            Done := Copy.Insert(true);
            if not Done then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'the Insert failed twice');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_GuardedModify()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
        Done: Boolean;
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        asserterror begin
            Copy := Src;
            Copy.Code := 'TARGET';
            if not Copy.Insert(true) then
                Done := Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'a Modify whose result is used');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_TableWithOnModifyTrigger()
    var
        Src: Record "NB Upsert Hooked";
        Copy: Record "NB Upsert Hooked";
    begin
        Clean();
        Src.Code := 'SOURCE';
        Src.Insert();
        Src.Get('SOURCE');
        Copy := Src;
        Copy.Code := 'TARGET';
        Copy.Insert(true);
        asserterror begin
            Copy := Src;
            Copy.Code := 'TARGET';
            if not Copy.Insert(true) then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'a table with an OnModify trigger');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_TransferFieldsWithKey_Raises()
    var
        Src: Record "NB Upsert Row";
        Buf: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        asserterror begin
            Buf.TransferFields(Src);
            Buf.Code := 'TARGET';
            if not Buf.Insert(true) then
                Buf.Modify(true);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'a version TransferFields copied with the key');
        Clean();
    end;

    // The own-Modify rule (al-runner issue 7798) on copies of the buffer: written, as on main.
    local procedure UpsertByValue(R: Record "NB Upsert Row")
    begin
        R.Code := 'B';
        if not R.Insert() then
            R.Modify();
    end;

    local procedure OwnModifyOfA(var R: Record "NB Upsert Row")
    var
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('A', 'a');
        Commit();
        R.Get('A');
        R.Name := 'own';
        R.Modify();
        Other.Code := 'B';
        Other.Insert();
    end;

    [Test]
    procedure OwnModifyOfA_ByValueUpsertOntoB_Written()
    var
        R: Record "NB Upsert Row";
    begin
        OwnModifyOfA(R);
        asserterror begin
            UpsertByValue(R);
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'an own-Modify version passed by value');
        Clean();
    end;

    [Test]
    procedure OwnModifyOfA_RecordRefUpsertOntoB_Written()
    var
        R: Record "NB Upsert Row";
        Ref: RecordRef;
    begin
        OwnModifyOfA(R);
        asserterror begin
            Ref.GetTable(R);
            Ref.Field(1).Value := 'B';
            if not Ref.Insert() then
                Ref.Modify();
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'an own-Modify version through a RecordRef');
        Clean();
    end;

    [Test]
    procedure OwnModifyOfA_OwnDeleteOfA_UpsertOntoB_Written()
    var
        R: Record "NB Upsert Row";
    begin
        OwnModifyOfA(R);
        R.Delete();
        asserterror begin
            R.Code := 'B';
            if not R.Insert() then
                R.Modify();
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('written', Outcome(), 'an own-Modify version after an own Delete');
        Clean();
    end;

    [Test]
    procedure RecordRefLockTable_ThenGet()
    var
        Src: Record "NB Upsert Row";
        Ref: RecordRef;
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Ref.Open(Database::"NB Upsert Row");
        Ref.LockTable();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a LockTable through a RecordRef');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_IsolatedStorageSetAfterRead()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        IsolatedStorage.Set('nb-upsert', 'v');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'an IsolatedStorage write after the read');
        Clean();
    end;

    [Test]
    procedure DeleteAllOfNoRowOnly_ThenGet()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Other.SetRange(Code, 'NONE');
        Other.DeleteAll();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'only a DeleteAll that matched no row before the read');
        Clean();
    end;

    [Test]
    procedure ModifyAllOfNoRowOnly_ThenGet()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Other.SetRange(Code, 'NONE');
        Other.ModifyAll(Name, 'all');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'only a ModifyAll that matched no row before the read');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_CommitAfterTargetInsert()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Commit();
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a Commit after the TARGET Insert');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_SameKeyDeletedAndInsertedAgain()
    var
        R: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        R.Get('SOURCE');
        Other.Get('SOURCE');
        Other.Delete();
        AddRow('SOURCE', 'again');
        R.Name := 'mine';
        asserterror begin
            if not R.Insert() then
                R.Modify();
            Error(NoErrorTxt);
        end;
        Assert.AreEqual('DB:RecordChanged', Outcome(), 'the same key deleted and inserted again after the read');
        Clean();
    end;

    [Test]
    procedure TableWritten_ThenGet_TwoRollbacks()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        AddRow('X', 'x');
        Src.Get('SOURCE');
        asserterror Error('NB_ROLLBACK_1');
        asserterror Error('NB_ROLLBACK_2');
        ClearLastError();
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'two rollbacks after the read');
        Clean();
    end;
}
