// Neighbors of al-runner issue 8912: what the test session reads in a stand-in test library's SingleInstance codeunits
// that hold temporary records, after the partner's login subscriber reached them.
codeunit 61855 "UT Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
    end;
}

codeunit 61856 "UT Login Tests"
{
    Subtype = Test;

    [Test]
    procedure UntouchedTempRecords()
    var
        M: Codeunit "UT Untouched";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure TempTableReadOnly()
    var
        M: Codeunit "UT Read";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure StoreMadeByFirstCall()
    var
        M: Codeunit "UT Used Store";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure TempTableWrittenThenEmptied()
    var
        M: Codeunit "UT Written";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure TempTableShared()
    var
        M: Codeunit "UT Shared";
    begin
        Check(M.Count(), 1);
    end;

    [Test]
    procedure TempRecordFiltered()
    var
        M: Codeunit "UT Filtered";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure StoreRecordRefOpened()
    var
        M: Codeunit "UT Opened Ref";
    begin
        Check(M.Count(), 100);
    end;

    [Test]
    procedure UntouchedTempRecordArray()
    var
        M: Codeunit "UT Temp Array";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure StoreSharedByAlias()
    var
        M: Codeunit "UT Aliased Store";
    begin
        Check(M.Count(), 1);
    end;

    [Test]
    procedure TempRecordKeptByRecordRef()
    var
        M: Codeunit "UT Ref Kept";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure LabelStoreMadeByFirstCall()
    var
        M: Codeunit "UT Label Store Owner";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure ExtraGlobalStoreMadeByFirstCall()
    var
        M: Codeunit "UT Extra Store Owner";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure SwappedStoreMadeByFirstCall()
    var
        M: Codeunit "UT Swapped Store Owner";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure TempArrayStoreMadeByFirstCall()
    var
        M: Codeunit "UT Arr Store Owner";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure StoreOneLevelDeeper()
    var
        M: Codeunit "UT Wrapped Store Owner";
    begin
        Check(M.Count(), 0);
    end;

    [Test]
    procedure StoreRowWrittenAtLogin()
    var
        M: Codeunit "UT Store Row Owner";
    begin
        Check(M.Count(), 1);
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;

    local procedure Check(Got: Integer; Want: Integer)
    begin
        if Got <> Want then
            Error('got %1, want %2', Got, Want);
    end;
}
