// Neighbors of al-runner issue 8721: a copy of a record (`Copy := Src`) whose key is moved to a row that exists, whose guarded
//   Insert(true) fails, then TransferFields and Modify(true). Whether that Modify raises DB:RecordChanged depends on whether the
//   open transaction had written the table before the buffer's row version reached it (corpus PR 1523,
//   record/crud/TestRecordFailedInsertMovedKey.al). Each test is one shape next to the measured ones. Invented tables only.

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
    begin
        Row.DeleteAll();
        Side.DeleteAll();
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

    // The 2nd upsert inside asserterror; returns 'written' or the error code, and rolls the cell back.
    local procedure SecondUpsert(var Src: Record "NB Upsert Row"): Text
    begin
        Src.Name := 'second';
        asserterror begin
            CopyTo(Src, 'TARGET');
            Error(NoErrorTxt);
        end;
        if GetLastErrorText() = NoErrorTxt then
            exit('written');
        exit(GetLastErrorCode());
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
    procedure OwnInsertOfAnotherKey_Written()
    var
        Src: Record "NB Upsert Row";
        Copy: Record "NB Upsert Row";
        Done: Boolean;
        Outcome: Text;
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
            Done := Copy.Insert(true);
            if not Done then
                Copy.Modify(true);
            Error(NoErrorTxt);
        end;
        if GetLastErrorText() = NoErrorTxt then
            Outcome := 'written'
        else
            Outcome := GetLastErrorCode();
        Assert.AreEqual('written', Outcome, 'a version from the buffer''s own Insert');
        Clean();
    end;

    [Test]
    procedure OwnModifyAfterGet_OtherKey_Written()
    var
        Src: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
        Done: Boolean;
        Outcome: Text;
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
            Done := Src.Insert();
            if not Done then
                Src.Modify();
            Error(NoErrorTxt);
        end;
        if GetLastErrorText() = NoErrorTxt then
            Outcome := 'written'
        else
            Outcome := GetLastErrorCode();
        Assert.AreEqual('written', Outcome, 'a version from the buffer''s own Modify of another row');
        Clean();
    end;

    [Test]
    procedure SameTableWritten_ThenFindFirst()
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
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a FindFirst after a write of the table');
        Clean();
    end;

    [Test]
    procedure SameTableLocked_ThenGet()
    var
        Src: Record "NB Upsert Row";
    begin
        Clean();
        AddRow('SOURCE', 'new');
        Commit();
        Src.LockTable();
        Src.Get('SOURCE');
        CopyTo(Src, 'TARGET');
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a Get after a LockTable of the table');
        Clean();
    end;

    [Test]
    procedure SameTableRowDeleted_ThenGet()
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
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a Get after a Delete of another row of the table');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_SameKey_ChangedElsewhere()
    var
        R: Record "NB Upsert Row";
        Other: Record "NB Upsert Row";
        Done: Boolean;
        Outcome: Text;
    begin
        Clean();
        AddRow('SOURCE', 'new');
        R.Get('SOURCE');
        Other.Get('SOURCE');
        Other.Name := 'elsewhere';
        Other.Modify();
        R.Name := 'mine';
        asserterror begin
            Done := R.Insert();
            if not Done then
                R.Modify();
            Error(NoErrorTxt);
        end;
        if GetLastErrorText() = NoErrorTxt then
            Outcome := 'written'
        else
            Outcome := GetLastErrorCode();
        Assert.AreEqual('DB:RecordChanged', Outcome, 'the same key, the row changed after the read');
        Clean();
    end;

    [Test]
    procedure UncommittedSeed_MovedKey_TargetModifiedAfterInsert()
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
        Assert.AreEqual('DB:RecordChanged', SecondUpsert(Src), 'a moved key onto a row modified after its Insert');
        Clean();
    end;
}
