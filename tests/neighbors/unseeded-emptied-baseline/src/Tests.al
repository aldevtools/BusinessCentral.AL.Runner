// Review of PR 8526: a DeleteAll that install code runs on a platform table of UNSEEDED_ROWS. One codeunit per shape,
// so the shapes run on different threads: the one that holds the captured install baseline, and those that thaw the
// shared one or read it back from the baseline cache. Each test asserts the answer the install shape would give if it
// counted, so a run that answers shows it.

// Install DeleteAll, then Insert, on Power BI Blob: four readers.
codeunit 61940 "UE Blob Install Count"
{
    Subtype = Test;

    [Test]
    procedure InstallClearedBlob_Count()
    var
        Blob: Record "Power BI Blob";
    begin
        if Blob.Count() <> 1 then
            Error('count=%1', Blob.Count());
    end;
}

codeunit 61941 "UE Blob Install FindFirst"
{
    Subtype = Test;

    [Test]
    procedure InstallClearedBlob_FindFirst()
    var
        Blob: Record "Power BI Blob";
    begin
        Blob.FindFirst();
        if Blob.Name <> 'UE After' then
            Error('first=%1', Blob.Name);
    end;
}

codeunit 61942 "UE Blob Install IsEmpty"
{
    Subtype = Test;

    [Test]
    procedure InstallClearedBlob_IsEmpty()
    var
        Blob: Record "Power BI Blob";
    begin
        if Blob.IsEmpty() then
            Error('empty');
    end;
}

codeunit 61943 "UE Blob Install Get Missing"
{
    Subtype = Test;

    [Test]
    procedure InstallClearedBlob_GetMissing()
    var
        Blob: Record "Power BI Blob";
    begin
        if Blob.Get(CreateGuid()) then
            Error('found %1', Blob.Name);
    end;
}

// The pinned shape: the test session's own DeleteAll, then an Insert (PtOnPremTables.al:324-356).
codeunit 61944 "UE Blob Session DeleteAll"
{
    Subtype = Test;

    [Test]
    procedure SessionClearedBlob_Count()
    var
        Blob: Record "Power BI Blob";
    begin
        Blob.DeleteAll();
        Blob.Id := CreateGuid();
        Blob.Name := 'UE Session';
        Blob.Insert();
        if Blob.Count() <> 1 then
            Error('count=%1', Blob.Count());
    end;
}

// The second unseeded platform table, Power BI Default Selection: the install shape and the pinned one (:365-385).
codeunit 61945 "UE Selection Install Count"
{
    Subtype = Test;

    [Test]
    procedure InstallClearedSelection_Count()
    var
        Selection: Record "Power BI Default Selection";
    begin
        if Selection.Count() <> 1 then
            Error('count=%1', Selection.Count());
    end;
}

codeunit 61946 "UE Selection Install FindFirst"
{
    Subtype = Test;

    [Test]
    procedure InstallClearedSelection_FindFirst()
    var
        Selection: Record "Power BI Default Selection";
    begin
        Selection.FindFirst();
        if Selection.Context <> 'UE AFTER' then
            Error('first=%1', Selection.Context);
    end;
}

codeunit 61947 "UE Selection Session DeleteAll"
{
    Subtype = Test;

    [Test]
    procedure SessionClearedSelection_Count()
    var
        Selection: Record "Power BI Default Selection";
    begin
        Selection.DeleteAll();
        Selection.Id := CreateGuid();
        Selection.Context := 'UE SESSION';
        Selection.Insert();
        if Selection.Count() <> 1 then
            Error('count=%1', Selection.Count());
    end;
}
