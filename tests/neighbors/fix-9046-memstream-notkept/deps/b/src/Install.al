dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.MemoryStream"; "FP MemStream Cap MS B") { }
        type("System.Security.Cryptography.HashAlgorithm"; "FP MemStream Cap Hash B") { }
        type("System.Array"; "FP MemStream Cap Arr B") { }
    }
}

table 53110 "FP MemStream Cap TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
        field(3; "Data"; Blob) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 53111 "FP MemStream Cap B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Buffer: Record "FP MemStream Cap TB" temporary;
        Row: Record "FP MemStream Cap TB";
        MemoryStream: DotNet "FP MemStream Cap MS B";
        HashAlgorithm: DotNet "FP MemStream Cap Hash B";
        HashBytes: DotNet "FP MemStream Cap Arr B";
        OutStr: OutStream;
        InStr: InStream;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('B');
        Buffer.Data.CreateInStream(InStr);
        MemoryStream := MemoryStream.MemoryStream(16);
        CopyStream(MemoryStream, InStr);
        HashAlgorithm := HashAlgorithm.Create('SHA256');
        HashBytes := HashAlgorithm.ComputeHash(MemoryStream.ToArray());
        HashAlgorithm.Dispose();
        Row.Code := 'B';
        Row.Value := Format(HashBytes.Length()) + '|' + Format(HashBytes.GetValue(0));
        Row.Insert();
    end;
}
