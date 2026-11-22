// Review of PR 8526: install shapes next to an install DeleteAll on a platform table of UNSEEDED_ROWS that leave no
// emptied mark, so a read in the tests refuses on main and on head alike. One codeunit per shape, so the shapes run on
// different threads. Each test asserts the answer the install shape would give if it counted.

// A filtered install DeleteAll on Power BI Blob: two readers.
codeunit 61970 "UE Blob Filtered Count"
{
    Subtype = Test;

    [Test]
    procedure FilteredInstallBlob_Count()
    var
        Blob: Record "Power BI Blob";
    begin
        if Blob.Count() <> 1 then
            Error('count=%1', Blob.Count());
    end;
}

codeunit 61971 "UE Blob Filtered FindFirst"
{
    Subtype = Test;

    [Test]
    procedure FilteredInstallBlob_FindFirst()
    var
        Blob: Record "Power BI Blob";
    begin
        Blob.FindFirst();
        if Blob.Name <> 'UE Keep' then
            Error('first=%1', Blob.Name);
    end;
}

// An install DeleteAll on Power BI Default Selection that a failed Codeunit.Run rolled back: two readers.
codeunit 61972 "UE Selection Undone Count"
{
    Subtype = Test;

    [Test]
    procedure RolledBackInstallSelection_Count()
    var
        Selection: Record "Power BI Default Selection";
    begin
        if Selection.Count() <> 1 then
            Error('count=%1', Selection.Count());
    end;
}

codeunit 61973 "UE Selection Undone IsEmpty"
{
    Subtype = Test;

    [Test]
    procedure RolledBackInstallSelection_IsEmpty()
    var
        Selection: Record "Power BI Default Selection";
    begin
        if Selection.IsEmpty() then
            Error('empty');
    end;
}

// The pinned shape on the same table: the test session's own DeleteAll, then an Insert (PtOnPremTables.al:324-356).
codeunit 61974 "UE Blob Session DeleteAll"
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
