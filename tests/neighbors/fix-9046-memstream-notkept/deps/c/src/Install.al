dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.MemoryStream"; "FP MemStream Cap MS C") { }
        type("System.Security.Cryptography.HashAlgorithm"; "FP MemStream Cap Hash C") { }
        type("System.Array"; "FP MemStream Cap Arr C") { }
    }
}

table 53120 "FP MemStream Cap TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
        field(3; "Data"; Blob) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 53121 "FP MemStream Cap C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Buffer: Record "FP MemStream Cap TC" temporary;
        Row: Record "FP MemStream Cap TC";
        MemoryStream: DotNet "FP MemStream Cap MS C";
        HashAlgorithm: DotNet "FP MemStream Cap Hash C";
        HashBytes: DotNet "FP MemStream Cap Arr C";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('C');
        Buffer.Data.CreateInStream(InStr);
        MemoryStream := MemoryStream.MemoryStream(16);
        CopyStream(MemoryStream, InStr);
        HashAlgorithm := HashAlgorithm.Create('SHA256');
        HashBytes := HashAlgorithm.ComputeHash(MemoryStream.ToArray());
        HashAlgorithm.Dispose();
        Row.Code := 'C';
        Row.Value := Format(HashBytes.Length()) + '|' + Format(HashBytes.GetValue(0));
        Row.Insert();
    end;
}
