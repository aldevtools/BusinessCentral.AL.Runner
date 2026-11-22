// Neighbor shapes of the Feature Key list gate (al-runner issue 8192): the list follows the platform build. Each test
// asserts the 14-row answer (the 29.0 rows and NewNotificationBar); a 13-row list fails it, a refused list does not run it.
codeunit 50000 "NB FK Neighbors"
{
    Subtype = Test;

    [Test]
    procedure Get_NewNotificationBar()
    var
        K: Record "Feature Key";
    begin
        if not K.Get('NewNotificationBar') then
            Error('no row');
        if K."Mandatory By Version" <> '30.0' then
            Error('version %1', K."Mandatory By Version");
    end;

    [Test]
    procedure Count_Is14()
    var
        K: Record "Feature Key";
    begin
        if K.Count() <> 14 then
            Error('count %1', K.Count());
    end;

    [Test]
    procedure RecordRef_Count_Is14()
    var
        R: RecordRef;
    begin
        R.Open(Database::"Feature Key");
        if R.Count() <> 14 then
            Error('count %1', R.Count());
    end;

    [Test]
    procedure Get_KeyOfBothLists()
    var
        K: Record "Feature Key";
    begin
        if not K.Get('ConcurrentInventoryPosting') then
            Error('no row');
    end;

    [Test]
    procedure Get_NoSuchKey()
    var
        K: Record "Feature Key";
    begin
        if K.Get('NoSuchFeatureKeyZzz') then
            Error('row');
    end;

    [Test]
    procedure Description_PinnedKey()
    var
        K: Record "Feature Key";
    begin
        K.Get('SalesPrices');
        if K.Description <> 'Feature Update: New sales pricing experience' then
            Error(K.Description);
    end;

    [Test]
    procedure Description_UnpinnedKey()
    var
        K: Record "Feature Key";
    begin
        K.Get('AdvancedTellMe');
        if K.Description <> 'Feature: Advanced Tell Me' then
            Error(K.Description);
    end;

    [Test]
    procedure Temporary_StartsEmpty()
    var
        K: Record "Feature Key" temporary;
    begin
        if not K.IsEmpty() then
            Error('not empty');
    end;
}
