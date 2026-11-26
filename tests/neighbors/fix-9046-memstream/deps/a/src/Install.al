dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.MemoryStream"; "FP MemStream MS A") { }
        type("System.Security.Cryptography.HashAlgorithm"; "FP MemStream Hash A") { }
        type("System.Array"; "FP MemStream Arr A") { }
    }
}

table 53000 "FP MemStream TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
        field(3; "Data"; Blob) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 53001 "FP MemStream A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Buffer: Record "FP MemStream TA" temporary;
        Row: Record "FP MemStream TA";
        MemoryStream: DotNet "FP MemStream MS A";
        HashAlgorithm: DotNet "FP MemStream Hash A";
        HashBytes: DotNet "FP MemStream Arr A";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('A');
        Buffer.Data.CreateInStream(InStr);
        MemoryStream := MemoryStream.MemoryStream();
        CopyStream(MemoryStream, InStr);
        HashAlgorithm := HashAlgorithm.Create('SHA256');
        HashBytes := HashAlgorithm.ComputeHash(MemoryStream.ToArray());
        HashAlgorithm.Dispose();
        Row.Code := 'A';
        Row.Value := Format(HashBytes.Length()) + '|' + Format(HashBytes.GetValue(0));
        Row.Insert();
    end;
}
