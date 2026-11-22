// The dependency app: install shapes next to the cleared one (tests/neighbors/unseeded-emptied-baseline/) that leave
// no emptied mark: a DeleteAll with a filter on Power BI Blob, and a DeleteAll on Power BI Default Selection that a
// failed Codeunit.Run rolls back. Each table gets its own shape, since the mark is per table.
codeunit 61990 "UE Unmarked Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Blob: Record "Power BI Blob";
        Selection: Record "Power BI Default Selection";
    begin
        // First, while no write is open, so Codeunit.Run may return its result.
        if Codeunit.Run(Codeunit::"UE Clear Selection And Fail") then
            Error('the clearing codeunit did not fail');
        Selection.Id := CreateGuid();
        Selection.Context := 'UE AFTER';
        Selection.Insert();

        Blob.Id := CreateGuid();
        Blob.Name := 'UE Keep';
        Blob.Insert();
        Blob.Id := CreateGuid();
        Blob.Name := 'UE Drop';
        Blob.Insert();
        Blob.SetRange(Name, 'UE Drop');
        Blob.DeleteAll();
    end;
}

codeunit 61991 "UE Clear Selection And Fail"
{
    trigger OnRun()
    var
        Selection: Record "Power BI Default Selection";
    begin
        Selection.Id := CreateGuid();
        Selection.Context := 'UE ROLLED BACK';
        Selection.Insert();
        Selection.DeleteAll();
        Error('UE clearing fails');
    end;
}
