dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.MemoryStream"; "FP MemStream Cap MS A") { }
        type("System.Security.Cryptography.HashAlgorithm"; "FP MemStream Cap Hash A") { }
        type("System.Array"; "FP MemStream Cap Arr A") { }
    }
}

table 53100 "FP MemStream Cap TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
        field(3; "Data"; Blob) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 53101 "FP MemStream Cap A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Buffer: Record "FP MemStream Cap TA" temporary;
        Row: Record "FP MemStream Cap TA";
        MemoryStream: DotNet "FP MemStream Cap MS A";
        HashAlgorithm: DotNet "FP MemStream Cap Hash A";
        HashBytes: DotNet "FP MemStream Cap Arr A";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('A');
        Buffer.Data.CreateInStream(InStr);
        MemoryStream := MemoryStream.MemoryStream(16);
        CopyStream(MemoryStream, InStr);
        HashAlgorithm := HashAlgorithm.Create('SHA256');
        HashBytes := HashAlgorithm.ComputeHash(MemoryStream.ToArray());
        HashAlgorithm.Dispose();
        Row.Code := 'A';
        Row.Value := Format(HashBytes.Length()) + '|' + Format(HashBytes.GetValue(0));
        Row.Insert();
    end;
}
