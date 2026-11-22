// The dependency app: its install code clears two platform tables a fresh tenant fills with rows nothing pins, then
// inserts one row into each. What a fresh tenant holds there when this install runs is not measured: only a DeleteAll
// in the test session is (corpus al-language-onprem/metadata/PtOnPremTables.al:324, :365).
codeunit 61960 "UE Fill Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Blob: Record "Power BI Blob";
        Selection: Record "Power BI Default Selection";
    begin
        Blob.Id := CreateGuid();
        Blob.Name := 'UE Before';
        Blob.Insert();
        Blob.DeleteAll();
        Blob.Id := CreateGuid();
        Blob.Name := 'UE After';
        Blob.Insert();

        Selection.Id := CreateGuid();
        Selection.Context := 'UE BEFORE';
        Selection.Insert();
        Selection.DeleteAll();
        Selection.Id := CreateGuid();
        Selection.Context := 'UE AFTER';
        Selection.Insert();
    end;
}
